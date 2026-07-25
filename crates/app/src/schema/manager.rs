#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::proto::ProtoSchemaManager;

use super::json_schema::{JsonSchemaCatalog, load_json_schema_catalog};
use super::{
    BindingResolution, MessageSchemaConfig, SchemaBinding, SchemaSelector, SchemaSourceKind,
    SchemaSourceStatus, SubjectPattern, ValidationPolicy,
};

#[derive(Default)]
pub struct MessageSchemaManager {
    pub(super) config: MessageSchemaConfig,
    pub(super) proto_sources: BTreeMap<u64, ProtoSchemaManager>,
    pub(super) json_sources: HashMap<u64, JsonSchemaCatalog>,
    pub(super) statuses: HashMap<u64, SchemaSourceStatus>,
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
            proto_sources: BTreeMap::new(),
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
}
