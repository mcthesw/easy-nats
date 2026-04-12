use serde::{Deserialize, Serialize};

/// Configuration for a NATS connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: u64,
    pub name: String,
    pub urls: Vec<String>,
    pub auth: AuthMethod,
    pub tls_enabled: bool,
}

impl ConnectionConfig {
    pub fn new(id: u64, name: String, url: String) -> Self {
        Self {
            id,
            name,
            urls: vec![url],
            auth: AuthMethod::None,
            tls_enabled: false,
        }
    }
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
