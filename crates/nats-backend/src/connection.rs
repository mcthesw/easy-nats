use serde::{Deserialize, Serialize};

/// Configuration for a NATS connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: u64,
    pub name: String,
    pub urls: Vec<String>,
    pub auth: AuthMethod,
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default)]
    pub tls_first: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<MonitoringConfig>,
}

impl ConnectionConfig {
    pub fn new(id: u64, name: String, url: String) -> Self {
        Self {
            id,
            name,
            urls: vec![url],
            auth: AuthMethod::None,
            tls_enabled: false,
            tls_first: false,
            monitoring: None,
        }
    }

    pub fn monitoring_endpoint(&self) -> Option<&str> {
        self.monitoring
            .as_ref()
            .map(|monitoring| monitoring.endpoint.trim())
            .filter(|endpoint| !endpoint.is_empty())
    }
}

/// Optional monitoring configuration for a connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub endpoint: String,
}

/// Authentication method for a NATS connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    None,
    Token { token: String },
    UserPassword { username: String, password: String },
    NKey { seed: String },
    CredentialsFile { path: String },
    TlsClientCert { cert_path: String, key_path: String },
}

/// Runtime status of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}
