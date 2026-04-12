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
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::En,
            dark_mode: true,
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
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!(?parent, %e, "Failed to create settings directory");
                return;
            }
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
}
