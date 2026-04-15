//! App-level settings (UI preferences) persisted separately from connection config.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub language: Language,
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    #[serde(default)]
    pub proto_schema_dir: Option<String>,
    #[serde(default)]
    pub topic_history: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::En,
            dark_mode: true,
            proto_schema_dir: None,
            topic_history: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(s) => {
                        tracing::info!(?path, "Loaded app settings");
                        return s;
                    }
                    Err(e) => {
                        tracing::warn!(?path, %e, "Failed to parse settings, using defaults");
                    }
                },
                Err(e) => {
                    tracing::warn!(?path, %e, "Failed to read settings, using defaults");
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
            tracing::error!(?parent, %e, "Failed to create settings directory");
            return;
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    tracing::error!(?path, %e, "Failed to write settings");
                }
            }
            Err(e) => {
                tracing::error!(%e, "Failed to serialize settings");
            }
        }
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("easy-nats")
            .join("settings.json")
    }

    const MAX_TOPIC_HISTORY: usize = 50;

    /// Record a topic as most-recently-used. Deduplicates and caps at MAX_TOPIC_HISTORY.
    pub fn record_topic(&mut self, topic: &str) {
        let topic = topic.trim().to_string();
        if topic.is_empty() {
            return;
        }
        self.topic_history.retain(|t| t != &topic);
        self.topic_history.insert(0, topic);
        self.topic_history.truncate(Self::MAX_TOPIC_HISTORY);
    }

    /// Return topics matching a prefix (MRU order).
    pub fn topic_suggestions(&self, prefix: &str) -> Vec<&str> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return self.topic_history.iter().map(|s| s.as_str()).collect();
        }
        self.topic_history
            .iter()
            .filter(|t| t.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }
}
