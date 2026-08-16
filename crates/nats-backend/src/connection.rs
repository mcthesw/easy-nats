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
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    None,
    Token { token: String },
    UserPassword { username: String, password: String },
    NKey { seed: String },
    CredentialsFile { path: String },
    TlsClientCert { cert_path: String, key_path: String },
}

impl std::fmt::Debug for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const REDACTED: &str = "[REDACTED]";
        match self {
            Self::None => f.write_str("None"),
            Self::Token { .. } => f.debug_struct("Token").field("token", &REDACTED).finish(),
            Self::UserPassword { .. } => f
                .debug_struct("UserPassword")
                .field("username", &REDACTED)
                .field("password", &REDACTED)
                .finish(),
            Self::NKey { .. } => f.debug_struct("NKey").field("seed", &REDACTED).finish(),
            Self::CredentialsFile { .. } => f
                .debug_struct("CredentialsFile")
                .field("path", &REDACTED)
                .finish(),
            Self::TlsClientCert { .. } => f
                .debug_struct("TlsClientCert")
                .field("cert_path", &REDACTED)
                .field("key_path", &REDACTED)
                .finish(),
        }
    }
}

/// Runtime status of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::AuthMethod;

    #[test]
    fn auth_method_debug_redacts_field_values() {
        let methods = [
            AuthMethod::Token {
                token: "token-secret".into(),
            },
            AuthMethod::UserPassword {
                username: "username-secret".into(),
                password: "password-secret".into(),
            },
            AuthMethod::NKey {
                seed: "seed-secret".into(),
            },
            AuthMethod::CredentialsFile {
                path: "credentials-secret".into(),
            },
            AuthMethod::TlsClientCert {
                cert_path: "certificate-secret".into(),
                key_path: "key-secret".into(),
            },
        ];

        for method in methods {
            let debug = format!("{method:?}");
            assert!(debug.contains("[REDACTED]"));
            assert!(!debug.contains("secret"));
        }
    }
}
