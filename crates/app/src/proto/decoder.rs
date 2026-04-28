use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage};

use super::AutoDetectResult;

/// Decode binary protobuf data using a specific message type from the pool.
pub(crate) fn decode_message(
    pool: &DescriptorPool,
    message_type: &str,
    data: &[u8],
) -> Result<String, String> {
    let desc = pool
        .get_message_by_name(message_type)
        .ok_or_else(|| format!("Unknown message type: {message_type}"))?;

    let msg = DynamicMessage::decode(desc, data).map_err(|e| format!("Decode failed: {e}"))?;

    let mut serializer = serde_json::Serializer::pretty(Vec::new());
    msg.serialize_with_options(
        &mut serializer,
        &prost_reflect::SerializeOptions::new().stringify_64_bit_integers(false),
    )
    .map_err(|e| format!("JSON serialization failed: {e}"))?;

    String::from_utf8(serializer.into_inner()).map_err(|e| format!("UTF-8 conversion failed: {e}"))
}

/// Encode JSON using a specific message type from the pool.
///
/// `prost-reflect` follows the canonical protobuf JSON mapping, so users can
/// edit a readable JSON representation while the publisher sends wire bytes.
pub(crate) fn encode_json_message(
    pool: &DescriptorPool,
    message_type: &str,
    json: &str,
) -> Result<Vec<u8>, String> {
    let desc = pool
        .get_message_by_name(message_type)
        .ok_or_else(|| format!("Unknown message type: {message_type}"))?;

    let mut deserializer = serde_json::Deserializer::from_str(json);
    let msg = DynamicMessage::deserialize(desc, &mut deserializer)
        .map_err(|e| format!("JSON protobuf mapping failed: {e}"))?;

    Ok(msg.encode_to_vec())
}

/// Try decoding with all known message types and categorize the result.
///
/// Uses a scoring heuristic: a message type that decodes without error and
/// consumes all input bytes scores higher than one that leaves trailing data.
pub(crate) fn auto_detect_message(
    pool: &DescriptorPool,
    message_types: &[String],
    data: &[u8],
) -> AutoDetectResult {
    if data.is_empty() {
        return AutoDetectResult::NoMatch;
    }

    let mut matches: Vec<(String, String)> = Vec::new();

    for type_name in message_types {
        let Some(desc) = pool.get_message_by_name(type_name) else {
            continue;
        };

        // Try to decode: must consume all bytes and have at least one field set
        if let Ok(msg) = DynamicMessage::decode(desc.clone(), data) {
            if !has_meaningful_fields(&msg) {
                continue;
            }

            let mut serializer = serde_json::Serializer::pretty(Vec::new());
            if msg
                .serialize_with_options(
                    &mut serializer,
                    &prost_reflect::SerializeOptions::new().stringify_64_bit_integers(false),
                )
                .is_ok()
                && let Ok(json) = String::from_utf8(serializer.into_inner())
            {
                matches.push((type_name.clone(), json));
            }
        }
    }

    match matches.len() {
        0 => AutoDetectResult::NoMatch,
        1 => {
            let (type_name, json) = matches.into_iter().next().unwrap();
            AutoDetectResult::Match { type_name, json }
        }
        _ => {
            let names = matches.into_iter().map(|(name, _)| name).collect();
            AutoDetectResult::Ambiguous(names)
        }
    }
}

/// Check whether a decoded message has at least one field with a non-default value.
/// This helps filter out "empty" decodes where protobuf happily returns all defaults.
fn has_meaningful_fields(msg: &DynamicMessage) -> bool {
    let desc = msg.descriptor();
    for field in desc.fields() {
        if msg.has_field(&field) {
            return true;
        }
    }
    false
}
