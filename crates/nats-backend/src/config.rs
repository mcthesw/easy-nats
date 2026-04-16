use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::connection::ConnectionConfig;
use crate::paths::ProjectPaths;

/// Persisted application configuration (connection profiles + settings).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub connections: Vec<ConnectionConfig>,
    #[serde(default)]
    pub next_id: u64,
}

impl AppConfig {
    /// Load config from the platform config directory.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => {
                        tracing::info!(?path, "Loaded config");
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!(?path, %e, "Failed to parse config, using default");
                    }
                },
                Err(e) => {
                    tracing::warn!(?path, %e, "Failed to read config, using default");
                }
            }
        }
        Self::default()
    }

    /// Save config to the platform config directory.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::error!(?parent, %e, "Failed to create config directory");
            return;
        }
        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    tracing::error!(?path, %e, "Failed to write config");
                }
            }
            Err(e) => {
                tracing::error!(%e, "Failed to serialize config");
            }
        }
    }

    /// Generate the next unique connection ID.
    pub fn next_connection_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn config_path() -> PathBuf {
        ProjectPaths::resolve().config_file("config.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::AuthMethod;

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig {
            connections: vec![ConnectionConfig {
                id: 0,
                name: "local".to_string(),
                urls: vec!["nats://localhost:4222".to_string()],
                auth: AuthMethod::None,
                tls_enabled: false,
                tls_first: false,
            }],
            next_id: 1,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.connections.len(), 1);
        assert_eq!(loaded.connections[0].name, "local");
        assert_eq!(loaded.next_id, 1);
    }

    #[test]
    fn test_auth_method_serialization() {
        let methods = vec![
            AuthMethod::None,
            AuthMethod::Token {
                token: "secret".to_string(),
            },
            AuthMethod::UserPassword {
                username: "user".to_string(),
                password: "pass".to_string(),
            },
            AuthMethod::NKey {
                seed: "SUAM...".to_string(),
            },
            AuthMethod::CredentialsFile {
                path: "/path/to/creds".to_string(),
            },
            AuthMethod::TlsClientCert {
                cert_path: "/cert.pem".to_string(),
                key_path: "/key.pem".to_string(),
            },
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            let loaded: AuthMethod = serde_json::from_str(&json).unwrap();
            // Verify serialize/deserialize round-trips
            let json2 = serde_json::to_string(&loaded).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_next_connection_id() {
        let mut config = AppConfig::default();
        assert_eq!(config.next_connection_id(), 0);
        assert_eq!(config.next_connection_id(), 1);
        assert_eq!(config.next_connection_id(), 2);
    }
}
