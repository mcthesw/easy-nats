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
    const MAX_DEPTH: usize = 4;
    if depth >= MAX_DEPTH || !seen.insert(message.full_name().to_string()) {
        return serde_json::Value::Object(serde_json::Map::new());
    }

    let mut object = serde_json::Map::new();
    for field in message.fields() {
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
        let value = protobuf_map_value_template(field, seen, depth);
        return serde_json::json!({ "key": value });
    }
    if field.is_list() {
        return serde_json::Value::Array(vec![protobuf_kind_template(&field.kind(), seen, depth)]);
    }
    protobuf_kind_template(&field.kind(), seen, depth)
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
