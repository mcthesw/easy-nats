mod decoder;
mod schema_loader;
mod wire_format;

use prost_reflect::{DescriptorPool, FieldDescriptor, Kind, MessageDescriptor};
use std::collections::HashSet;
use std::path::PathBuf;

/// Result of auto-detecting a protobuf message type.
pub enum AutoDetectResult {
    /// Exactly one message type decoded successfully.
    Match { type_name: String, json: String },
    /// Multiple types decoded — ambiguous.
    Ambiguous(Vec<String>),
    /// No message type could decode the payload.
    NoMatch,
}

/// Per-tab state for protobuf rendering controls.
#[derive(Debug)]
pub struct ProtoViewState {
    /// Selected message type (fully-qualified), or empty for auto-detect.
    pub selected_type: String,
    /// Cached decode output for the current payload.
    pub cached_output: Option<String>,
    /// Whether to use auto-detect mode.
    pub auto_detect: bool,
}

impl Default for ProtoViewState {
    fn default() -> Self {
        Self {
            selected_type: String::new(),
            cached_output: None,
            auto_detect: true,
        }
    }
}

/// Manages protobuf schema files and provides message decoding capabilities.
///
/// Compiles `.proto` files from a user-configured directory using `protox`,
/// then uses `prost-reflect` to decode binary messages at runtime.
#[derive(Default)]
pub struct ProtoSchemaManager {
    schema_dir: Option<PathBuf>,
    pool: Option<DescriptorPool>,
    message_types: Vec<String>,
    last_error: Option<String>,
}

impl ProtoSchemaManager {
    pub fn set_schema_dir(&mut self, dir: PathBuf) {
        self.last_error = None;
        match schema_loader::compile_schema_dir(&dir) {
            Ok((pool, types)) => {
                self.schema_dir = Some(dir);
                self.message_types = types;
                self.pool = Some(pool);
            }
            Err(e) => {
                self.last_error = Some(e);
                self.schema_dir = Some(dir);
                self.pool = None;
                self.message_types.clear();
            }
        }
    }

    pub fn message_types(&self) -> &[String] {
        &self.message_types
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Decode binary data using the specified fully-qualified message type.
    pub fn decode(&self, data: &[u8], message_type: &str) -> Result<String, String> {
        let pool = self.pool.as_ref().ok_or("No schemas loaded")?;
        decoder::decode_message(pool, message_type, data)
    }

    /// Encode JSON into protobuf wire bytes using the specified fully-qualified message type.
    pub fn encode_json(&self, json: &str, message_type: &str) -> Result<Vec<u8>, String> {
        let pool = self.pool.as_ref().ok_or("No schemas loaded")?;
        decoder::encode_json_message(pool, message_type, json)
    }

    /// Generate a JSON object template for the specified fully-qualified message type.
    pub fn json_template(&self, message_type: &str) -> Result<String, String> {
        let pool = self.pool.as_ref().ok_or("No schemas loaded")?;
        let message = pool
            .get_message_by_name(message_type)
            .ok_or_else(|| format!("Unknown message type: {message_type}"))?;
        let template = protobuf_message_template(&message, &mut HashSet::new(), 0);
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("JSON template serialization failed: {error}"))
    }

    /// Try all known message types and return categorized results.
    pub fn auto_detect_and_decode(&self, data: &[u8]) -> AutoDetectResult {
        match self.pool.as_ref() {
            Some(pool) => decoder::auto_detect_message(pool, &self.message_types, data),
            None => AutoDetectResult::NoMatch,
        }
    }

    /// Decode as raw wire-format (no schema needed).
    pub fn decode_wire_format(data: &[u8]) -> String {
        wire_format::decode_wire_format(data)
    }
}

fn protobuf_message_template(
    message: &MessageDescriptor,
    seen: &mut HashSet<String>,
    depth: usize,
) -> serde_json::Value {
    if let Some(template) = protobuf_well_known_template(message.full_name()) {
        return template;
    }

    const MAX_DEPTH: usize = 4;
    if depth >= MAX_DEPTH || !seen.insert(message.full_name().to_string()) {
        return serde_json::Value::Object(serde_json::Map::new());
    }

    let mut object = serde_json::Map::new();
    let mut emitted_oneofs = HashSet::new();
    for field in message.fields() {
        if let Some(oneof) = field.containing_oneof()
            && !emitted_oneofs.insert(oneof.full_name().to_string())
        {
            continue;
        }
        object.insert(
            field.json_name().to_string(),
            protobuf_field_template(&field, seen, depth + 1),
        );
    }

    seen.remove(message.full_name());
    serde_json::Value::Object(object)
}

fn protobuf_field_template(
    field: &FieldDescriptor,
    seen: &mut HashSet<String>,
    depth: usize,
) -> serde_json::Value {
    if field.is_map() {
        let key = protobuf_map_key_template(field);
        let value = protobuf_map_value_template(field, seen, depth);
        return serde_json::json!({ key: value });
    }
    if field.is_list() {
        return serde_json::Value::Array(vec![protobuf_kind_template(&field.kind(), seen, depth)]);
    }
    protobuf_kind_template(&field.kind(), seen, depth)
}

fn protobuf_well_known_template(full_name: &str) -> Option<serde_json::Value> {
    // ProtoJSON gives well-known types custom JSON shapes:
    // https://protobuf.dev/programming-guides/json/#json_mapping
    match full_name {
        "google.protobuf.Timestamp" => Some(serde_json::json!("1970-01-01T00:00:00Z")),
        "google.protobuf.Duration" => Some(serde_json::json!("0s")),
        "google.protobuf.FieldMask" => Some(serde_json::json!("")),
        "google.protobuf.Struct" | "google.protobuf.Empty" => Some(serde_json::json!({})),
        "google.protobuf.ListValue" => Some(serde_json::json!([])),
        "google.protobuf.Value" | "google.protobuf.Any" => Some(serde_json::Value::Null),
        "google.protobuf.FloatValue" | "google.protobuf.DoubleValue" => {
            Some(serde_json::json!(0.0))
        }
        "google.protobuf.Int32Value"
        | "google.protobuf.Int64Value"
        | "google.protobuf.UInt32Value"
        | "google.protobuf.UInt64Value" => Some(serde_json::json!(0)),
        "google.protobuf.BoolValue" => Some(serde_json::json!(false)),
        "google.protobuf.StringValue" | "google.protobuf.BytesValue" => Some(serde_json::json!("")),
        _ => None,
    }
}

fn protobuf_map_key_template(field: &FieldDescriptor) -> String {
    let Kind::Message(entry) = field.kind() else {
        return "key".to_string();
    };
    let Some(key_field) = entry.fields().find(|field| field.name() == "key") else {
        return "key".to_string();
    };
    match key_field.kind() {
        Kind::Bool => "false",
        Kind::String => "key",
        Kind::Int32
        | Kind::Int64
        | Kind::Uint32
        | Kind::Uint64
        | Kind::Sint32
        | Kind::Sint64
        | Kind::Fixed32
        | Kind::Fixed64
        | Kind::Sfixed32
        | Kind::Sfixed64 => "0",
        _ => "key",
    }
    .to_string()
}

fn protobuf_map_value_template(
    field: &FieldDescriptor,
    seen: &mut HashSet<String>,
    depth: usize,
) -> serde_json::Value {
    let Kind::Message(entry) = field.kind() else {
        return serde_json::Value::Null;
    };
    entry
        .fields()
        .find(|field| field.name() == "value")
        .map(|field| protobuf_kind_template(&field.kind(), seen, depth))
        .unwrap_or(serde_json::Value::Null)
}

fn protobuf_kind_template(
    kind: &Kind,
    seen: &mut HashSet<String>,
    depth: usize,
) -> serde_json::Value {
    match kind {
        Kind::Double | Kind::Float => serde_json::json!(0.0),
        Kind::Int32
        | Kind::Int64
        | Kind::Uint32
        | Kind::Uint64
        | Kind::Sint32
        | Kind::Sint64
        | Kind::Fixed32
        | Kind::Fixed64
        | Kind::Sfixed32
        | Kind::Sfixed64 => serde_json::json!(0),
        Kind::Bool => serde_json::json!(false),
        Kind::String | Kind::Bytes => serde_json::json!(""),
        Kind::Enum(enum_descriptor) => enum_descriptor
            .values()
            .next()
            .map(|value| serde_json::json!(value.name()))
            .unwrap_or(serde_json::Value::Null),
        Kind::Message(message) => protobuf_message_template(message, seen, depth),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_template_emits_single_oneof_case_that_can_encode() {
        let dir = unique_temp_dir("easy-nats-proto-oneof-template");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("event.proto"),
            r#"
                syntax = "proto3";
                package demo;
                message Event {
                    string id = 1;
                    oneof payload {
                        string created = 2;
                        int32 count = 3;
                    }
                }
            "#,
        )
        .unwrap();

        let mut manager = ProtoSchemaManager::default();
        manager.set_schema_dir(dir.clone());

        let template = manager.json_template("demo.Event").unwrap();
        let value: serde_json::Value = serde_json::from_str(&template).unwrap();
        assert!(value.get("id").is_some());
        assert!(value.get("created").is_some());
        assert!(value.get("count").is_none());
        manager.encode_json(&template, "demo.Event").unwrap();

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn json_template_uses_protojson_forms_for_well_known_types() {
        let dir = unique_temp_dir("easy-nats-proto-wkt-template");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("event.proto"),
            r#"
                syntax = "proto3";
                package demo;

                import "google/protobuf/any.proto";
                import "google/protobuf/duration.proto";
                import "google/protobuf/empty.proto";
                import "google/protobuf/field_mask.proto";
                import "google/protobuf/struct.proto";
                import "google/protobuf/timestamp.proto";
                import "google/protobuf/wrappers.proto";

                message Event {
                    google.protobuf.Timestamp created_at = 1;
                    google.protobuf.Duration elapsed = 2;
                    google.protobuf.FieldMask mask = 3;
                    google.protobuf.Struct attrs = 4;
                    google.protobuf.ListValue items = 5;
                    google.protobuf.Value arbitrary = 6;
                    google.protobuf.Empty empty = 7;
                    google.protobuf.StringValue label = 8;
                    google.protobuf.Int64Value count = 9;
                    google.protobuf.Any anything = 10;
                }
            "#,
        )
        .unwrap();

        let mut manager = ProtoSchemaManager::default();
        manager.set_schema_dir(dir.clone());

        let template = manager.json_template("demo.Event").unwrap();
        let value: serde_json::Value = serde_json::from_str(&template).unwrap();
        assert_eq!(value["createdAt"], "1970-01-01T00:00:00Z");
        assert_eq!(value["elapsed"], "0s");
        assert_eq!(value["mask"], "");
        assert_eq!(value["attrs"], serde_json::json!({}));
        assert_eq!(value["items"], serde_json::json!([]));
        assert_eq!(value["arbitrary"], serde_json::Value::Null);
        assert_eq!(value["empty"], serde_json::json!({}));
        assert_eq!(value["label"], "");
        assert_eq!(value["count"], 0);
        assert_eq!(value["anything"], serde_json::Value::Null);
        manager.encode_json(&template, "demo.Event").unwrap();

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn json_template_uses_type_valid_map_keys() {
        let dir = unique_temp_dir("easy-nats-proto-map-template");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("maps.proto"),
            r#"
                syntax = "proto3";
                package demo;

                message Maps {
                    map<string, string> by_name = 1;
                    map<int32, string> by_id = 2;
                    map<bool, int32> by_flag = 3;
                    map<uint64, bool> by_count = 4;
                }
            "#,
        )
        .unwrap();

        let mut manager = ProtoSchemaManager::default();
        manager.set_schema_dir(dir.clone());

        let template = manager.json_template("demo.Maps").unwrap();
        let value: serde_json::Value = serde_json::from_str(&template).unwrap();
        assert_eq!(value["byName"], serde_json::json!({ "key": "" }));
        assert_eq!(value["byId"], serde_json::json!({ "0": "" }));
        assert_eq!(value["byFlag"], serde_json::json!({ "false": 0 }));
        assert_eq!(value["byCount"], serde_json::json!({ "0": false }));
        manager.encode_json(&template, "demo.Maps").unwrap();

        let _ = std::fs::remove_dir_all(dir);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }
}
