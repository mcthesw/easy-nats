use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaSource {
    pub id: u64,
    pub name: String,
    pub kind: SchemaSourceKind,
    pub path: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaSourceKind {
    Protobuf,
    JsonSchema,
}

impl SchemaSourceKind {
    pub const ALL: [Self; 2] = [Self::Protobuf, Self::JsonSchema];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Protobuf => "message_schema.kind_protobuf",
            Self::JsonSchema => "message_schema.kind_json_schema",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PayloadInputFormat {
    #[default]
    Text,
    MessagePack,
}

impl PayloadInputFormat {
    pub const ALL: [Self; 2] = [Self::Text, Self::MessagePack];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Text => "common.payload_input_text",
            Self::MessagePack => "common.payload_input_messagepack",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaBinding {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<u64>,
    pub subject_pattern: String,
    pub source_id: u64,
    pub selector: SchemaSelector,
    pub policy: ValidationPolicy,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub order: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SchemaSelector {
    ProtobufMessage { type_name: String },
    JsonSchema { entry: String },
}

impl SchemaSelector {
    pub fn entry(&self) -> &str {
        match self {
            Self::ProtobufMessage { type_name } => type_name,
            Self::JsonSchema { entry } => entry,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationPolicy {
    #[default]
    Inspect,
    Warn,
    Block,
}

impl ValidationPolicy {
    pub const ALL: [Self; 3] = [Self::Inspect, Self::Warn, Self::Block];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Inspect => "message_schema.policy_inspect",
            Self::Warn => "message_schema.policy_warn",
            Self::Block => "message_schema.policy_block",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaSourceStatus {
    pub state: SchemaSourceState,
    pub message: Option<String>,
    pub entries: Vec<String>,
}

impl SchemaSourceStatus {
    pub(super) fn disabled() -> Self {
        Self {
            state: SchemaSourceState::Disabled,
            message: None,
            entries: Vec::new(),
        }
    }

    pub(super) fn loaded(entries: Vec<String>) -> Self {
        Self {
            state: SchemaSourceState::Loaded,
            message: None,
            entries,
        }
    }

    pub(super) fn error(message: String) -> Self {
        Self {
            state: SchemaSourceState::Error,
            message: Some(message),
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaSourceState {
    Disabled,
    Loaded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaStatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadSchemaStatus {
    pub level: SchemaStatusLevel,
    pub label: String,
    pub message: String,
    pub can_send: bool,
}

pub struct OutgoingPayload {
    pub payload: Vec<u8>,
    pub status: Option<PayloadSchemaStatus>,
    pub can_send: bool,
}

pub struct RenderedSchemaPayload {
    pub status: PayloadSchemaStatus,
    pub json: Option<String>,
}

pub enum BindingResolution<'a> {
    NoMatch,
    Match(&'a SchemaBinding),
    Ambiguous(Vec<&'a SchemaBinding>),
}

pub fn kv_subject(bucket: &str, key: &str) -> String {
    format!("$KV.{bucket}.{key}")
}

fn default_true() -> bool {
    true
}
