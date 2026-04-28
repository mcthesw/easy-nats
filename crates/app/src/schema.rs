use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use nats_backend::ProjectPaths;
use serde::{Deserialize, Serialize};

use crate::proto::{AutoDetectResult, ProtoSchemaManager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageSchemaConfig {
    #[serde(default)]
    pub sources: Vec<SchemaSource>,
    #[serde(default)]
    pub bindings: Vec<SchemaBinding>,
    #[serde(default)]
    pub next_source_id: u64,
    #[serde(default)]
    pub next_binding_id: u64,
    #[serde(default)]
    imported_legacy_proto_dirs: Vec<String>,
}

impl MessageSchemaConfig {
    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(mut config) => {
                        config.ensure_next_ids();
                        tracing::info!(?path, "Loaded message schema config");
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!(?path, %e, "Failed to parse message schema config");
                    }
                },
                Err(e) => {
                    tracing::warn!(?path, %e, "Failed to read message schema config");
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::error!(?parent, %e, "Failed to create message schema config directory");
            return;
        }
        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    tracing::error!(?path, %e, "Failed to write message schema config");
                }
            }
            Err(e) => {
                tracing::error!(%e, "Failed to serialize message schema config");
            }
        }
    }

    fn path() -> PathBuf {
        ProjectPaths::resolve().config_file("message-schemas.json")
    }

    fn ensure_next_ids(&mut self) {
        let next_source = self
            .sources
            .iter()
            .map(|source| source.id)
            .max()
            .unwrap_or(0)
            + 1;
        let next_binding = self
            .bindings
            .iter()
            .map(|binding| binding.id)
            .max()
            .unwrap_or(0)
            + 1;
        self.next_source_id = self.next_source_id.max(next_source);
        self.next_binding_id = self.next_binding_id.max(next_binding);
    }

    pub fn add_source(&mut self, name: String, kind: SchemaSourceKind, path: String) -> u64 {
        let id = self.next_source_id;
        self.next_source_id += 1;
        self.sources.push(SchemaSource {
            id,
            name,
            kind,
            path,
            enabled: true,
        });
        id
    }

    pub fn remove_source(&mut self, source_id: u64) {
        self.sources.retain(|source| source.id != source_id);
        self.bindings
            .retain(|binding| binding.source_id != source_id);
    }

    pub fn set_source_enabled(&mut self, source_id: u64, enabled: bool) {
        if let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)
        {
            source.enabled = enabled;
        }
    }

    pub fn add_binding(
        &mut self,
        name: String,
        connection_id: Option<u64>,
        subject_pattern: String,
        source_id: u64,
        selector: SchemaSelector,
        policy: ValidationPolicy,
    ) -> Result<u64, String> {
        SubjectPattern::parse(&subject_pattern)?;
        let id = self.next_binding_id;
        self.next_binding_id += 1;
        self.bindings.push(SchemaBinding {
            id,
            name,
            connection_id,
            subject_pattern,
            source_id,
            selector,
            policy,
            enabled: true,
            order: id,
        });
        Ok(id)
    }

    pub fn remove_binding(&mut self, binding_id: u64) {
        self.bindings.retain(|binding| binding.id != binding_id);
    }

    pub fn set_binding_enabled(&mut self, binding_id: u64, enabled: bool) {
        if let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.id == binding_id)
        {
            binding.enabled = enabled;
        }
    }

    pub fn import_legacy_proto_dir(&mut self, dir: &str) -> Option<u64> {
        let normalized = dir.trim();
        if normalized.is_empty()
            || self
                .imported_legacy_proto_dirs
                .iter()
                .any(|d| d == normalized)
        {
            return None;
        }
        if let Some(existing) = self.sources.iter().find(|source| {
            source.kind == SchemaSourceKind::Protobuf && source.path.trim() == normalized
        }) {
            self.imported_legacy_proto_dirs.push(normalized.to_string());
            return Some(existing.id);
        }
        let name = Path::new(normalized)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| format!("Imported {name}"))
            .unwrap_or_else(|| "Imported Protobuf schemas".to_string());
        let id = self.add_source(name, SchemaSourceKind::Protobuf, normalized.to_string());
        self.imported_legacy_proto_dirs.push(normalized.to_string());
        Some(id)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectPattern {
    tokens: Vec<SubjectToken>,
}

impl SubjectPattern {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return Err("Subject pattern cannot be empty".to_string());
        }
        let mut tokens = Vec::new();
        for (idx, token) in trimmed.split('.').enumerate() {
            if token.is_empty() {
                return Err("Subject pattern cannot contain empty tokens".to_string());
            }
            let parsed = match token {
                "*" => SubjectToken::One,
                ">" => {
                    if idx != trimmed.split('.').count() - 1 {
                        return Err("The > wildcard must be the final token".to_string());
                    }
                    SubjectToken::Tail
                }
                literal if literal.contains('*') || literal.contains('>') => {
                    return Err("Wildcards must occupy the whole token".to_string());
                }
                literal => SubjectToken::Literal(literal.to_string()),
            };
            tokens.push(parsed);
        }
        Ok(Self { tokens })
    }

    pub fn matches(&self, subject: &str) -> bool {
        let subject_tokens: Vec<&str> = subject.trim().split('.').collect();
        if subject.trim().is_empty() || subject_tokens.iter().any(|token| token.is_empty()) {
            return false;
        }
        let mut subject_idx = 0;
        for (pattern_idx, token) in self.tokens.iter().enumerate() {
            match token {
                SubjectToken::Literal(literal) => {
                    if subject_tokens.get(subject_idx).copied() != Some(literal.as_str()) {
                        return false;
                    }
                    subject_idx += 1;
                }
                SubjectToken::One => {
                    if subject_tokens.get(subject_idx).is_none() {
                        return false;
                    }
                    subject_idx += 1;
                }
                SubjectToken::Tail => {
                    return pattern_idx == self.tokens.len() - 1
                        && subject_tokens.len() > subject_idx;
                }
            }
        }
        subject_idx == subject_tokens.len()
    }

    pub fn specificity(&self) -> u32 {
        self.tokens
            .iter()
            .map(|token| match token {
                SubjectToken::Literal(_) => 10,
                SubjectToken::One => 3,
                SubjectToken::Tail => 1,
            })
            .sum::<u32>()
            + self.tokens.len() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SubjectToken {
    Literal(String),
    One,
    Tail,
}

#[derive(Debug, Clone)]
pub struct SchemaSourceStatus {
    pub state: SchemaSourceState,
    pub message: Option<String>,
    pub entries: Vec<String>,
}

impl SchemaSourceStatus {
    fn disabled() -> Self {
        Self {
            state: SchemaSourceState::Disabled,
            message: None,
            entries: Vec::new(),
        }
    }

    fn loaded(entries: Vec<String>) -> Self {
        Self {
            state: SchemaSourceState::Loaded,
            message: None,
            entries,
        }
    }

    fn error(message: String) -> Self {
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

#[derive(Default)]
pub struct MessageSchemaManager {
    config: MessageSchemaConfig,
    proto_sources: HashMap<u64, ProtoSchemaManager>,
    json_sources: HashMap<u64, JsonSchemaCatalog>,
    statuses: HashMap<u64, SchemaSourceStatus>,
}

impl MessageSchemaManager {
    pub fn load(legacy_proto_schema_dir: Option<&str>) -> Self {
        let mut config = MessageSchemaConfig::load();
        let imported = legacy_proto_schema_dir
            .and_then(|dir| config.import_legacy_proto_dir(dir))
            .is_some();
        if imported {
            config.save();
        }
        Self::from_config(config)
    }

    pub fn from_config(config: MessageSchemaConfig) -> Self {
        let mut manager = Self {
            config,
            proto_sources: HashMap::new(),
            json_sources: HashMap::new(),
            statuses: HashMap::new(),
        };
        manager.reload_all();
        manager
    }

    pub fn config(&self) -> &MessageSchemaConfig {
        &self.config
    }

    pub fn status(&self, source_id: u64) -> Option<&SchemaSourceStatus> {
        self.statuses.get(&source_id)
    }

    pub fn source_entries(&self, source_id: u64) -> &[String] {
        self.status(source_id)
            .map(|status| status.entries.as_slice())
            .unwrap_or(&[])
    }

    pub fn add_source(&mut self, name: String, kind: SchemaSourceKind, path: String) -> u64 {
        let id = self.config.add_source(name, kind, path);
        self.reload_source(id);
        self.config.save();
        id
    }

    pub fn remove_source(&mut self, source_id: u64) {
        self.config.remove_source(source_id);
        self.proto_sources.remove(&source_id);
        self.json_sources.remove(&source_id);
        self.statuses.remove(&source_id);
        self.config.save();
    }

    pub fn set_source_enabled(&mut self, source_id: u64, enabled: bool) {
        self.config.set_source_enabled(source_id, enabled);
        self.reload_source(source_id);
        self.config.save();
    }

    pub fn add_binding(
        &mut self,
        name: String,
        connection_id: Option<u64>,
        subject_pattern: String,
        source_id: u64,
        selector: SchemaSelector,
        policy: ValidationPolicy,
    ) -> Result<u64, String> {
        let id = self.config.add_binding(
            name,
            connection_id,
            subject_pattern,
            source_id,
            selector,
            policy,
        )?;
        self.config.save();
        Ok(id)
    }

    pub fn remove_binding(&mut self, binding_id: u64) {
        self.config.remove_binding(binding_id);
        self.config.save();
    }

    pub fn set_binding_enabled(&mut self, binding_id: u64, enabled: bool) {
        self.config.set_binding_enabled(binding_id, enabled);
        self.config.save();
    }

    pub fn reload_all(&mut self) {
        self.proto_sources.clear();
        self.json_sources.clear();
        self.statuses.clear();
        let source_ids: Vec<u64> = self.config.sources.iter().map(|source| source.id).collect();
        for source_id in source_ids {
            self.reload_source(source_id);
        }
    }

    pub fn reload_source(&mut self, source_id: u64) {
        self.proto_sources.remove(&source_id);
        self.json_sources.remove(&source_id);
        let Some(source) = self
            .config
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .cloned()
        else {
            self.statuses.remove(&source_id);
            return;
        };

        if !source.enabled {
            self.statuses
                .insert(source_id, SchemaSourceStatus::disabled());
            return;
        }

        match source.kind {
            SchemaSourceKind::Protobuf => {
                let mut manager = ProtoSchemaManager::default();
                manager.set_schema_dir(PathBuf::from(&source.path));
                if let Some(error) = manager.last_error() {
                    self.statuses
                        .insert(source_id, SchemaSourceStatus::error(error.to_string()));
                } else {
                    let entries = manager.message_types().to_vec();
                    self.proto_sources.insert(source_id, manager);
                    self.statuses
                        .insert(source_id, SchemaSourceStatus::loaded(entries));
                }
            }
            SchemaSourceKind::JsonSchema => match load_json_schema_catalog(Path::new(&source.path))
            {
                Ok(catalog) => {
                    let entries = catalog.entries();
                    self.json_sources.insert(source_id, catalog);
                    self.statuses
                        .insert(source_id, SchemaSourceStatus::loaded(entries));
                }
                Err(error) => {
                    self.statuses
                        .insert(source_id, SchemaSourceStatus::error(error));
                }
            },
        }
    }

    pub fn resolve_binding(&self, connection_id: u64, subject: &str) -> BindingResolution<'_> {
        let mut ranked: Vec<(&SchemaBinding, u8, u32, u64)> = Vec::new();
        for binding in self
            .config
            .bindings
            .iter()
            .filter(|binding| binding.enabled)
        {
            if binding
                .connection_id
                .is_some_and(|binding_connection| binding_connection != connection_id)
            {
                continue;
            }
            let Ok(pattern) = SubjectPattern::parse(&binding.subject_pattern) else {
                continue;
            };
            if pattern.matches(subject) {
                let scope = u8::from(binding.connection_id.is_some());
                ranked.push((binding, scope, pattern.specificity(), binding.order));
            }
        }

        ranked.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.3.cmp(&b.3))
        });

        let Some((best, scope, specificity, order)) = ranked.first().copied() else {
            return BindingResolution::NoMatch;
        };
        let ties: Vec<&SchemaBinding> = ranked
            .iter()
            .filter(
                |(_, candidate_scope, candidate_specificity, candidate_order)| {
                    *candidate_scope == scope
                        && *candidate_specificity == specificity
                        && *candidate_order == order
                },
            )
            .map(|(binding, _, _, _)| *binding)
            .collect();
        if ties.len() > 1 {
            BindingResolution::Ambiguous(ties)
        } else {
            BindingResolution::Match(best)
        }
    }

    pub fn prepare_outgoing(
        &self,
        connection_id: u64,
        subject: &str,
        payload_text: &str,
    ) -> OutgoingPayload {
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => OutgoingPayload {
                payload: payload_text.as_bytes().to_vec(),
                status: None,
                can_send: true,
            },
            BindingResolution::Ambiguous(bindings) => {
                let status = PayloadSchemaStatus {
                    level: SchemaStatusLevel::Warning,
                    label: "Ambiguous schema binding".to_string(),
                    message: format!("{} bindings match this subject", bindings.len()),
                    can_send: true,
                };
                OutgoingPayload {
                    payload: payload_text.as_bytes().to_vec(),
                    status: Some(status),
                    can_send: true,
                }
            }
            BindingResolution::Match(binding) => self.prepare_with_binding(binding, payload_text),
        }
    }

    pub fn render_schema_payload(
        &self,
        connection_id: u64,
        subject: &str,
        payload: &[u8],
    ) -> Option<RenderedSchemaPayload> {
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => None,
            BindingResolution::Ambiguous(bindings) => Some(RenderedSchemaPayload {
                status: PayloadSchemaStatus {
                    level: SchemaStatusLevel::Warning,
                    label: "Ambiguous schema binding".to_string(),
                    message: format!("{} bindings match this subject", bindings.len()),
                    can_send: true,
                },
                json: None,
            }),
            BindingResolution::Match(binding) => Some(self.render_with_binding(binding, payload)),
        }
    }

    pub fn payload_template(
        &self,
        connection_id: u64,
        subject: &str,
    ) -> Result<Option<String>, String> {
        if subject.trim().is_empty() {
            return Ok(None);
        }
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => Ok(None),
            BindingResolution::Ambiguous(bindings) => {
                Err(format!("{} bindings match this subject", bindings.len()))
            }
            BindingResolution::Match(binding) => self.template_with_binding(binding).map(Some),
        }
    }

    pub fn manual_message_types(&self) -> Vec<String> {
        let mut types = BTreeMap::new();
        for manager in self.proto_sources.values() {
            for ty in manager.message_types() {
                types.insert(ty.clone(), ());
            }
        }
        types.into_keys().collect()
    }

    pub fn decode_manual_proto(&self, data: &[u8], message_type: &str) -> Result<String, String> {
        let mut last_error = None;
        for manager in self.proto_sources.values() {
            if manager.message_types().iter().any(|ty| ty == message_type) {
                match manager.decode(data, message_type) {
                    Ok(json) => return Ok(json),
                    Err(error) => last_error = Some(error),
                }
            }
        }
        Err(last_error.unwrap_or_else(|| format!("Unknown message type: {message_type}")))
    }

    pub fn auto_detect_manual_proto(&self, data: &[u8]) -> AutoDetectResult {
        let mut matches = Vec::new();
        for manager in self.proto_sources.values() {
            match manager.auto_detect_and_decode(data) {
                AutoDetectResult::Match { type_name, json } => matches.push((type_name, json)),
                AutoDetectResult::Ambiguous(types) => {
                    return AutoDetectResult::Ambiguous(types);
                }
                AutoDetectResult::NoMatch => {}
            }
        }
        match matches.len() {
            0 => AutoDetectResult::NoMatch,
            1 => {
                let (type_name, json) = matches.into_iter().next().unwrap();
                AutoDetectResult::Match { type_name, json }
            }
            _ => AutoDetectResult::Ambiguous(
                matches
                    .into_iter()
                    .map(|(type_name, _)| type_name)
                    .collect(),
            ),
        }
    }

    pub fn decode_wire_format(data: &[u8]) -> String {
        ProtoSchemaManager::decode_wire_format(data)
    }

    fn prepare_with_binding(&self, binding: &SchemaBinding, payload_text: &str) -> OutgoingPayload {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return self.unavailable_outgoing(binding, payload_text);
                };
                match manager.encode_json(payload_text, type_name) {
                    Ok(payload) => {
                        let status = PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid Protobuf payload".to_string(),
                            can_send: true,
                        };
                        OutgoingPayload {
                            payload,
                            status: Some(status),
                            can_send: true,
                        }
                    }
                    Err(error) => self.invalid_outgoing(binding, payload_text, error),
                }
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return self.unavailable_outgoing(binding, payload_text);
                };
                let json = match serde_json::from_str::<serde_json::Value>(payload_text) {
                    Ok(json) => json,
                    Err(error) => {
                        return self.invalid_outgoing(
                            binding,
                            payload_text,
                            format!("Invalid JSON: {error}"),
                        );
                    }
                };
                match catalog.validate(entry, &json) {
                    Ok(()) => {
                        let status = PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid JSON Schema payload".to_string(),
                            can_send: true,
                        };
                        OutgoingPayload {
                            payload: payload_text.as_bytes().to_vec(),
                            status: Some(status),
                            can_send: true,
                        }
                    }
                    Err(error) => self.invalid_outgoing(binding, payload_text, error),
                }
            }
        }
    }

    fn render_with_binding(
        &self,
        binding: &SchemaBinding,
        payload: &[u8],
    ) -> RenderedSchemaPayload {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return RenderedSchemaPayload {
                        status: self.unavailable_status(binding),
                        json: None,
                    };
                };
                match manager.decode(payload, type_name) {
                    Ok(json) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Decoded with Protobuf schema".to_string(),
                            can_send: true,
                        },
                        json: Some(json),
                    },
                    Err(error) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Error,
                            label: self.binding_label(binding),
                            message: error,
                            can_send: true,
                        },
                        json: None,
                    },
                }
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return RenderedSchemaPayload {
                        status: self.unavailable_status(binding),
                        json: None,
                    };
                };
                let json = match serde_json::from_slice::<serde_json::Value>(payload) {
                    Ok(json) => json,
                    Err(error) => {
                        return RenderedSchemaPayload {
                            status: PayloadSchemaStatus {
                                level: SchemaStatusLevel::Error,
                                label: self.binding_label(binding),
                                message: format!("Invalid JSON: {error}"),
                                can_send: true,
                            },
                            json: None,
                        };
                    }
                };
                let pretty =
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string());
                match catalog.validate(entry, &json) {
                    Ok(()) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid JSON Schema payload".to_string(),
                            can_send: true,
                        },
                        json: Some(pretty),
                    },
                    Err(error) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Error,
                            label: self.binding_label(binding),
                            message: error,
                            can_send: true,
                        },
                        json: Some(pretty),
                    },
                }
            }
        }
    }

    fn template_with_binding(&self, binding: &SchemaBinding) -> Result<String, String> {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return Err("Schema source is unavailable".to_string());
                };
                manager.json_template(type_name)
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return Err("Schema source is unavailable".to_string());
                };
                catalog.template(entry)
            }
        }
    }

    fn unavailable_outgoing(&self, binding: &SchemaBinding, payload_text: &str) -> OutgoingPayload {
        let status = self.unavailable_status(binding);
        let can_send = status.can_send;
        OutgoingPayload {
            payload: payload_text.as_bytes().to_vec(),
            status: Some(status),
            can_send,
        }
    }

    fn unavailable_status(&self, binding: &SchemaBinding) -> PayloadSchemaStatus {
        let can_send = binding.policy != ValidationPolicy::Block;
        PayloadSchemaStatus {
            level: if can_send {
                SchemaStatusLevel::Warning
            } else {
                SchemaStatusLevel::Error
            },
            label: self.binding_label(binding),
            message: "Schema source is unavailable".to_string(),
            can_send,
        }
    }

    fn invalid_outgoing(
        &self,
        binding: &SchemaBinding,
        payload_text: &str,
        error: String,
    ) -> OutgoingPayload {
        let can_send = binding.policy != ValidationPolicy::Block;
        let level = match binding.policy {
            ValidationPolicy::Inspect => SchemaStatusLevel::Info,
            ValidationPolicy::Warn => SchemaStatusLevel::Warning,
            ValidationPolicy::Block => SchemaStatusLevel::Error,
        };
        let status = PayloadSchemaStatus {
            level,
            label: self.binding_label(binding),
            message: error,
            can_send,
        };
        OutgoingPayload {
            payload: payload_text.as_bytes().to_vec(),
            status: Some(status),
            can_send,
        }
    }

    fn binding_label(&self, binding: &SchemaBinding) -> String {
        let source_name = self
            .config
            .sources
            .iter()
            .find(|source| source.id == binding.source_id)
            .map(|source| source.name.as_str())
            .unwrap_or("Unknown source");
        format!("{source_name} / {}", binding.selector.entry())
    }
}

struct JsonSchemaCatalog {
    validators: HashMap<String, jsonschema::Validator>,
    schemas: HashMap<String, serde_json::Value>,
}

impl JsonSchemaCatalog {
    fn entries(&self) -> Vec<String> {
        let mut entries: Vec<String> = self.validators.keys().cloned().collect();
        entries.sort();
        entries
    }

    fn validate(&self, entry: &str, value: &serde_json::Value) -> Result<(), String> {
        let validator = self
            .validators
            .get(entry)
            .ok_or_else(|| format!("Unknown JSON Schema entry: {entry}"))?;
        validator.validate(value).map_err(|error| error.to_string())
    }

    fn template(&self, entry: &str) -> Result<String, String> {
        let schema = self
            .schemas
            .get(entry)
            .ok_or_else(|| format!("Unknown JSON Schema entry: {entry}"))?;
        let template = json_schema_template(schema);
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("JSON template serialization failed: {error}"))
    }
}

fn load_json_schema_catalog(path: &Path) -> Result<JsonSchemaCatalog, String> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_json_files(path, &mut files)
            .map_err(|e| format!("Failed to scan directory: {e}"))?;
    } else {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if files.is_empty() {
        return Err(format!("No JSON schema files found in {}", path.display()));
    }

    let mut validators = HashMap::new();
    let mut schemas = HashMap::new();
    let mut used_entries = HashSet::new();
    for file in files {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read {}: {e}", file.display()))?;
        let schema = serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Failed to parse {}: {e}", file.display()))?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|e| format!("Failed to compile {}: {e}", file.display()))?;
        let mut entry = schema_entry_name(path, &file);
        if !used_entries.insert(entry.clone()) {
            entry = file.to_string_lossy().replace('\\', "/");
        }
        validators.insert(entry.clone(), validator);
        schemas.insert(entry, schema);
    }
    Ok(JsonSchemaCatalog {
        validators,
        schemas,
    })
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn schema_entry_name(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .ok()
        .filter(|relative| relative.components().count() > 0)
        .unwrap_or(file)
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

fn json_schema_template(schema: &serde_json::Value) -> serde_json::Value {
    json_schema_template_inner(schema, schema, &mut HashSet::new())
}

fn json_schema_template_inner(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> serde_json::Value {
    if let Some(reference) = schema["$ref"].as_str()
        && let Some(referenced_schema) = resolve_local_json_schema_ref(root, reference)
    {
        if !seen_refs.insert(reference.to_string()) {
            return serde_json::Value::Null;
        }
        let value = json_schema_template_inner(root, referenced_schema, seen_refs);
        seen_refs.remove(reference);
        return value;
    }

    for key in ["default", "const"] {
        if !schema[key].is_null() {
            return schema[key].clone();
        }
    }
    if let Some(example) = schema["examples"]
        .as_array()
        .and_then(|examples| examples.first())
    {
        return example.clone();
    }
    if let Some(value) = schema["enum"].as_array().and_then(|values| values.first()) {
        return value.clone();
    }
    if let Some(value) = json_schema_template_from_combinator(root, schema, seen_refs) {
        return value;
    }

    match json_schema_type(schema) {
        Some("object") => json_schema_object_template(root, schema, seen_refs),
        None if schema["properties"].is_object() => {
            json_schema_object_template(root, schema, seen_refs)
        }
        Some("array") => {
            let item = if schema["items"].is_object() {
                json_schema_template_inner(root, &schema["items"], seen_refs)
            } else {
                serde_json::Value::Null
            };
            serde_json::Value::Array(vec![item])
        }
        Some("integer") => schema["minimum"].clone().as_i64().map_or_else(
            || serde_json::json!(0),
            |minimum| serde_json::json!(minimum),
        ),
        Some("number") => schema["minimum"].clone().as_f64().map_or_else(
            || serde_json::json!(0.0),
            |minimum| serde_json::json!(minimum),
        ),
        Some("boolean") => serde_json::json!(false),
        Some("null") => serde_json::Value::Null,
        Some("string") => json_schema_string_template(schema),
        _ => json_schema_string_template(schema),
    }
}

fn json_schema_template_from_combinator(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> Option<serde_json::Value> {
    for key in ["oneOf", "anyOf"] {
        if let Some(first) = schema[key].as_array().and_then(|schemas| schemas.first()) {
            return Some(json_schema_template_inner(root, first, seen_refs));
        }
    }

    let schemas = schema["allOf"].as_array()?;
    let mut merged = serde_json::Map::new();
    for item in schemas {
        match json_schema_template_inner(root, item, seen_refs) {
            serde_json::Value::Object(object) => merged.extend(object),
            value if merged.is_empty() => return Some(value),
            _ => {}
        }
    }
    Some(serde_json::Value::Object(merged))
}

fn json_schema_object_template(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    seen_refs: &mut HashSet<String>,
) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    let Some(properties) = schema["properties"].as_object() else {
        return serde_json::Value::Object(object);
    };

    let mut keys = Vec::new();
    if let Some(required) = schema["required"].as_array() {
        for key in required.iter().filter_map(|key| key.as_str()) {
            if properties.contains_key(key) {
                keys.push(key.to_string());
            }
        }
    }
    for key in properties.keys() {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.clone());
        }
    }

    for key in keys {
        if let Some(property_schema) = properties.get(&key) {
            object.insert(
                key,
                json_schema_template_inner(root, property_schema, seen_refs),
            );
        }
    }
    serde_json::Value::Object(object)
}

fn json_schema_type(schema: &serde_json::Value) -> Option<&str> {
    if let Some(schema_type) = schema["type"].as_str() {
        return Some(schema_type);
    }
    schema["type"].as_array().and_then(|types| {
        types
            .iter()
            .filter_map(|schema_type| schema_type.as_str())
            .find(|schema_type| *schema_type != "null")
    })
}

fn json_schema_string_template(schema: &serde_json::Value) -> serde_json::Value {
    match schema["format"].as_str() {
        Some("date-time") => serde_json::json!("2026-01-01T00:00:00Z"),
        Some("date") => serde_json::json!("2026-01-01"),
        Some("email") => serde_json::json!("user@example.com"),
        Some("uri") | Some("url") => serde_json::json!("https://example.com"),
        _ => serde_json::json!(""),
    }
}

fn resolve_local_json_schema_ref<'a>(
    root: &'a serde_json::Value,
    reference: &str,
) -> Option<&'a serde_json::Value> {
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_pattern_matches_nats_wildcards() {
        let exact = SubjectPattern::parse("orders.created").unwrap();
        assert!(exact.matches("orders.created"));
        assert!(!exact.matches("orders.updated"));

        let one = SubjectPattern::parse("orders.*").unwrap();
        assert!(one.matches("orders.created"));
        assert!(!one.matches("orders.us.created"));

        let tail = SubjectPattern::parse("orders.>").unwrap();
        assert!(tail.matches("orders.created"));
        assert!(tail.matches("orders.us.created"));
        assert!(!tail.matches("orders"));
    }

    #[test]
    fn subject_pattern_rejects_invalid_tail_position() {
        assert!(SubjectPattern::parse("orders.>.created").is_err());
        assert!(SubjectPattern::parse("orders..created").is_err());
        assert!(SubjectPattern::parse("orders.created").is_ok());
    }

    #[test]
    fn resolver_prefers_connection_specific_and_specific_subjects() {
        let mut config = MessageSchemaConfig::default();
        let source_id = config.add_source(
            "schemas".to_string(),
            SchemaSourceKind::JsonSchema,
            "unused".to_string(),
        );
        config
            .add_binding(
                "all".to_string(),
                None,
                "orders.>".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "order".to_string(),
                },
                ValidationPolicy::Inspect,
            )
            .unwrap();
        let specific = config
            .add_binding(
                "specific".to_string(),
                Some(7),
                "orders.created".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "created".to_string(),
                },
                ValidationPolicy::Block,
            )
            .unwrap();
        let manager = MessageSchemaManager::from_config(config);

        match manager.resolve_binding(7, "orders.created") {
            BindingResolution::Match(binding) => assert_eq!(binding.id, specific),
            _ => panic!("expected matching binding"),
        }
    }

    #[test]
    fn resolver_uses_more_specific_subject_for_same_scope() {
        let mut config = MessageSchemaConfig::default();
        let source_id = config.add_source(
            "schemas".to_string(),
            SchemaSourceKind::JsonSchema,
            "unused".to_string(),
        );
        config
            .add_binding(
                "tail".to_string(),
                None,
                "orders.>".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "tail".to_string(),
                },
                ValidationPolicy::Inspect,
            )
            .unwrap();
        let exact = config
            .add_binding(
                "exact".to_string(),
                None,
                "orders.created".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "exact".to_string(),
                },
                ValidationPolicy::Inspect,
            )
            .unwrap();
        let manager = MessageSchemaManager::from_config(config);

        match manager.resolve_binding(1, "orders.created") {
            BindingResolution::Match(binding) => assert_eq!(binding.id, exact),
            _ => panic!("expected exact binding"),
        }
    }

    #[test]
    fn resolver_reports_ambiguous_same_rank_and_order() {
        let mut config = MessageSchemaConfig::default();
        let source_id = config.add_source(
            "schemas".to_string(),
            SchemaSourceKind::JsonSchema,
            "unused".to_string(),
        );
        let first = config
            .add_binding(
                "first".to_string(),
                None,
                "orders.*".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "first".to_string(),
                },
                ValidationPolicy::Inspect,
            )
            .unwrap();
        let second = config
            .add_binding(
                "second".to_string(),
                None,
                "orders.*".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "second".to_string(),
                },
                ValidationPolicy::Inspect,
            )
            .unwrap();
        for binding in &mut config.bindings {
            binding.order = 1;
        }
        let manager = MessageSchemaManager::from_config(config);

        match manager.resolve_binding(1, "orders.created") {
            BindingResolution::Ambiguous(bindings) => {
                let ids: Vec<u64> = bindings.iter().map(|binding| binding.id).collect();
                assert!(ids.contains(&first));
                assert!(ids.contains(&second));
            }
            _ => panic!("expected ambiguous bindings"),
        }
    }

    #[test]
    fn json_schema_validation_reports_invalid_payload() {
        let mut catalog = JsonSchemaCatalog {
            validators: HashMap::new(),
            schemas: HashMap::new(),
        };
        let schema = serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": { "id": { "type": "string" } }
        });
        catalog.validators.insert(
            "order".to_string(),
            jsonschema::validator_for(&schema).unwrap(),
        );
        catalog.schemas.insert("order".to_string(), schema);

        assert!(
            catalog
                .validate("order", &serde_json::json!({ "id": "A1" }))
                .is_ok()
        );
        assert!(
            catalog
                .validate("order", &serde_json::json!({ "id": 1 }))
                .is_err()
        );
    }

    #[test]
    fn json_schema_entry_name_uses_relative_directory_path() {
        let root = Path::new("D:/schemas/json");
        let direct = Path::new("D:/schemas/json/order-created.json");
        let nested = Path::new("D:/schemas/json/orders/created.schema.json");

        assert_eq!(schema_entry_name(root, direct), "order-created");
        assert_eq!(schema_entry_name(root, nested), "orders/created.schema");
    }

    #[test]
    fn json_schema_template_uses_schema_hints() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["id", "items"],
            "properties": {
                "id": { "type": "string", "default": "ORD-1" },
                "items": {
                    "type": "array",
                    "items": { "type": "string", "examples": ["sku-1"] }
                },
                "paid": { "type": "boolean" }
            }
        });

        assert_eq!(
            json_schema_template(&schema),
            serde_json::json!({
                "id": "ORD-1",
                "items": ["sku-1"],
                "paid": false
            })
        );
    }

    #[test]
    fn blocking_json_schema_binding_prevents_invalid_publish() {
        let mut config = MessageSchemaConfig::default();
        let source_id = config.add_source(
            "schemas".to_string(),
            SchemaSourceKind::JsonSchema,
            "unused".to_string(),
        );
        config
            .add_binding(
                "orders".to_string(),
                None,
                "orders.created".to_string(),
                source_id,
                SchemaSelector::JsonSchema {
                    entry: "order".to_string(),
                },
                ValidationPolicy::Block,
            )
            .unwrap();
        let mut manager = MessageSchemaManager::from_config(config);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": { "id": { "type": "string" } }
        });
        let mut validators = HashMap::new();
        validators.insert(
            "order".to_string(),
            jsonschema::validator_for(&schema).unwrap(),
        );
        manager.json_sources.insert(
            source_id,
            JsonSchemaCatalog {
                validators,
                schemas: HashMap::from([("order".to_string(), schema)]),
            },
        );
        manager.statuses.insert(
            source_id,
            SchemaSourceStatus::loaded(vec!["order".to_string()]),
        );

        let invalid = manager.prepare_outgoing(1, "orders.created", r#"{"id":1}"#);
        assert!(!invalid.can_send);
        assert_eq!(
            invalid.status.as_ref().map(|status| status.level),
            Some(SchemaStatusLevel::Error)
        );

        let valid = manager.prepare_outgoing(1, "orders.created", r#"{"id":"A1"}"#);
        assert!(valid.can_send);
        assert_eq!(valid.payload, br#"{"id":"A1"}"#);
    }

    #[test]
    fn protobuf_json_can_encode_and_decode_with_loaded_source() {
        let dir = unique_temp_dir("easy-nats-proto-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("order.proto"),
            r#"
                syntax = "proto3";
                package demo;
                message Order {
                    string id = 1;
                    int32 count = 2;
                }
            "#,
        )
        .unwrap();

        let mut manager = ProtoSchemaManager::default();
        manager.set_schema_dir(dir.clone());
        let bytes = manager
            .encode_json(r#"{"id":"A1","count":2}"#, "demo.Order")
            .unwrap();
        let json = manager.decode(&bytes, "demo.Order").unwrap();
        let template = manager.json_template("demo.Order").unwrap();

        assert!(json.contains(r#""id": "A1""#));
        assert!(json.contains(r#""count": 2"#));
        assert!(template.contains(r#""id": """#));
        assert!(template.contains(r#""count": 0"#));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_proto_dir_import_creates_unbound_source_once() {
        let mut config = MessageSchemaConfig::default();
        let first = config.import_legacy_proto_dir("C:/schemas");
        let second = config.import_legacy_proto_dir("C:/schemas");

        assert!(first.is_some());
        assert_eq!(second, None);
        assert_eq!(config.sources.len(), 1);
        assert!(config.bindings.is_empty());
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
