//! App-level settings (UI preferences) persisted separately from connection config.

use std::path::PathBuf;

use nats_backend::ProjectPaths;
use serde::{Deserialize, Serialize};

use crate::i18n::Language;
use crate::theme::ThemeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub language: Language,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeId>,
    #[serde(default)]
    pub proto_schema_dir: Option<String>,
    #[serde(default)]
    pub topic_history: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StoredAppSettings {
    #[serde(default)]
    language: Language,
    #[serde(default)]
    theme: Option<ThemeId>,
    #[serde(default)]
    dark_mode: Option<bool>,
    #[serde(default)]
    proto_schema_dir: Option<String>,
    #[serde(default)]
    topic_history: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::En,
            theme: None,
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
                Ok(content) => match Self::parse(&content) {
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

    fn parse(content: &str) -> Result<Self, serde_json::Error> {
        let stored: StoredAppSettings = serde_json::from_str(content)?;
        Ok(Self {
            language: stored.language,
            theme: stored
                .theme
                .or_else(|| stored.dark_mode.map(ThemeId::from_legacy_dark_mode)),
            proto_schema_dir: stored.proto_schema_dir,
            topic_history: stored.topic_history,
        })
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
        ProjectPaths::resolve().config_file("settings.json")
    }

    pub fn resolved_theme(&self, system_prefers_dark: Option<bool>) -> ThemeId {
        crate::theme::resolve_theme(self.theme, system_prefers_dark)
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

#[cfg(test)]
mod tests {
    use super::AppSettings;
    use crate::theme::ThemeId;

    #[test]
    fn explicit_theme_wins_over_legacy_dark_mode() {
        let settings = AppSettings::parse(
            r#"{
                "theme": "catppuccin-macchiato",
                "dark_mode": false
            }"#,
        )
        .unwrap();

        assert_eq!(settings.theme, Some(ThemeId::CatppuccinMacchiato));
    }

    #[test]
    fn legacy_dark_mode_migrates_to_theme_id() {
        let settings = AppSettings::parse(r#"{ "dark_mode": false }"#).unwrap();

        assert_eq!(settings.theme, Some(ThemeId::EguiLight));
    }

    #[test]
    fn startup_resolution_prefers_saved_theme() {
        let settings = AppSettings {
            theme: Some(ThemeId::CatppuccinMocha),
            ..Default::default()
        };

        assert_eq!(
            settings.resolved_theme(Some(false)),
            ThemeId::CatppuccinMocha
        );
    }

    #[test]
    fn startup_resolution_falls_back_to_system_preference() {
        let settings = AppSettings::default();

        assert_eq!(settings.resolved_theme(Some(false)), ThemeId::EguiLight);
        assert_eq!(settings.resolved_theme(Some(true)), ThemeId::EguiDark);
        assert_eq!(settings.resolved_theme(None), ThemeId::EguiDark);
    }
}
