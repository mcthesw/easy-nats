#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use nats_backend::ProjectPaths;
use serde::{Deserialize, Serialize};

use super::{
    SchemaBinding, SchemaSelector, SchemaSource, SchemaSourceKind, SubjectPattern, ValidationPolicy,
};

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
    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Self {
        Self::default()
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
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
