use crate::i18n::t;
use nats_backend::{
    AuthMethod, ConsumerAckPolicyKind, ConsumerDeliverPolicyKind, ConsumerInfo, KvBucketInfo,
    StorageKind,
};

use crate::schema::PayloadInputFormat;

#[derive(Default)]
pub(crate) struct ConnectionEditor {
    pub(crate) visible: bool,
    pub(crate) editing_id: Option<u64>,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) auth_kind: AuthKindSelection,
    pub(crate) token: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) metrics_endpoint: String,
    pub(crate) nkey_seed: String,
    pub(crate) creds_path: String,
    pub(crate) cert_path: String,
    pub(crate) key_path: String,
    pub(crate) tls_enabled: bool,
    pub(crate) tls_first: bool,
    pub(crate) delete_confirm: Option<u64>,
    pub(crate) test_state: ConnectionTestState,
}

impl ConnectionEditor {
    pub(crate) fn auth_method(&self) -> AuthMethod {
        match self.auth_kind {
            AuthKindSelection::None => AuthMethod::None,
            AuthKindSelection::Token => AuthMethod::Token {
                token: self.token.clone(),
            },
            AuthKindSelection::UserPassword => AuthMethod::UserPassword {
                username: self.username.clone(),
                password: self.password.clone(),
            },
            AuthKindSelection::NKey => AuthMethod::NKey {
                seed: self.nkey_seed.clone(),
            },
            AuthKindSelection::CredentialsFile => AuthMethod::CredentialsFile {
                path: self.creds_path.clone(),
            },
            AuthKindSelection::TlsClientCert => AuthMethod::TlsClientCert {
                cert_path: self.cert_path.clone(),
                key_path: self.key_path.clone(),
            },
        }
    }

    pub(crate) fn start_test(&mut self, request_id: u64) {
        self.test_state = ConnectionTestState::Pending { request_id };
    }

    pub(crate) fn invalidate_test(&mut self) {
        self.test_state = ConnectionTestState::Idle;
    }

    pub(crate) fn complete_test(&mut self, request_id: u64, result: Result<(), String>) -> bool {
        if !matches!(
            &self.test_state,
            ConnectionTestState::Pending {
                request_id: pending
            } if *pending == request_id
        ) {
            return false;
        }

        self.test_state = match result {
            Ok(()) => ConnectionTestState::Succeeded,
            Err(message) => ConnectionTestState::Failed(message),
        };
        true
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum ConnectionTestState {
    #[default]
    Idle,
    Pending {
        request_id: u64,
    },
    Succeeded,
    Failed(String),
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub(crate) enum AuthKindSelection {
    #[default]
    None,
    Token,
    UserPassword,
    NKey,
    CredentialsFile,
    TlsClientCert,
}

impl AuthKindSelection {
    pub(crate) const ALL: [Self; 6] = [
        Self::None,
        Self::Token,
        Self::UserPassword,
        Self::NKey,
        Self::CredentialsFile,
        Self::TlsClientCert,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::None => t("connection.auth_none"),
            Self::Token => t("connection.auth_token"),
            Self::UserPassword => t("connection.auth_user_password"),
            Self::NKey => t("connection.auth_nkey"),
            Self::CredentialsFile => t("connection.auth_credentials_file"),
            Self::TlsClientCert => t("connection.auth_tls_client_cert"),
        }
    }
}

pub(crate) struct StreamCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) name: String,
    pub(crate) subjects: String,
    pub(crate) storage: StorageSelection,
    pub(crate) retention: RetentionSelection,
    pub(crate) max_messages: String,
    pub(crate) max_bytes: String,
    pub(crate) max_age_secs: String,
    pub(crate) num_replicas: String,
    pub(crate) description: String,
}

impl Default for StreamCreateEditor {
    fn default() -> Self {
        Self {
            visible: false,
            connection_id: 0,
            name: String::new(),
            subjects: String::new(),
            storage: StorageSelection::File,
            retention: RetentionSelection::Limits,
            max_messages: String::new(),
            max_bytes: String::new(),
            max_age_secs: String::new(),
            num_replicas: "1".to_string(),
            description: String::new(),
        }
    }
}

#[derive(Default)]
pub(crate) struct StreamPublishEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) stream_name: String,
    pub(crate) subject: String,
    pub(crate) payload: String,
    pub(crate) payload_input_format: PayloadInputFormat,
    pub(crate) headers: Vec<(String, String)>,
}

impl StreamPublishEditor {
    pub(crate) fn for_stream(connection_id: u64, stream_name: String, subject: String) -> Self {
        Self {
            visible: true,
            connection_id,
            stream_name,
            subject,
            payload: String::new(),
            payload_input_format: PayloadInputFormat::Text,
            headers: Vec::new(),
        }
    }
}

pub(crate) struct ConsumerCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) stream_name: String,
    pub(crate) name: String,
    pub(crate) durable: bool,
    pub(crate) deliver_policy: DeliverPolicySelection,
    pub(crate) deliver_start_sequence: String,
    pub(crate) deliver_start_time: String,
    pub(crate) ack_policy: AckPolicySelection,
    pub(crate) filter_subject: String,
    pub(crate) max_deliver: String,
    pub(crate) max_ack_pending: String,
    pub(crate) description: String,
}

impl Default for ConsumerCreateEditor {
    fn default() -> Self {
        Self {
            visible: false,
            connection_id: 0,
            stream_name: String::new(),
            name: String::new(),
            durable: true,
            deliver_policy: DeliverPolicySelection::All,
            deliver_start_sequence: "1".to_string(),
            deliver_start_time: String::new(),
            ack_policy: AckPolicySelection::Explicit,
            filter_subject: String::new(),
            max_deliver: String::new(),
            max_ack_pending: String::new(),
            description: String::new(),
        }
    }
}

pub(crate) struct KvBucketCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) bucket: String,
    pub(crate) history: String,
    pub(crate) max_age_secs: String,
    pub(crate) max_value_size: String,
    pub(crate) max_bytes: String,
    pub(crate) storage: StorageSelection,
    pub(crate) num_replicas: String,
    pub(crate) description: String,
}

impl Default for KvBucketCreateEditor {
    fn default() -> Self {
        Self {
            visible: false,
            connection_id: 0,
            bucket: String::new(),
            history: "1".to_string(),
            max_age_secs: String::new(),
            max_value_size: String::new(),
            max_bytes: String::new(),
            storage: StorageSelection::File,
            num_replicas: "1".to_string(),
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliverPolicySelection {
    All,
    Last,
    New,
    ByStartSequence,
    ByStartTime,
    LastPerSubject,
}

impl DeliverPolicySelection {
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Last,
        Self::New,
        Self::ByStartSequence,
        Self::ByStartTime,
        Self::LastPerSubject,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => t("consumer.policy_all"),
            Self::Last => t("consumer.policy_last"),
            Self::New => t("consumer.policy_new"),
            Self::ByStartSequence => t("consumer.policy_by_start_sequence"),
            Self::ByStartTime => t("consumer.policy_by_start_time"),
            Self::LastPerSubject => t("consumer.policy_last_per_subject"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AckPolicySelection {
    Explicit,
    All,
    None,
}

impl AckPolicySelection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Explicit => t("consumer.ack_explicit"),
            Self::All => t("consumer.ack_all"),
            Self::None => t("consumer.ack_none"),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub(crate) enum StorageSelection {
    #[default]
    File,
    Memory,
}

impl StorageSelection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::File => t("common.storage_file"),
            Self::Memory => t("common.storage_memory"),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub(crate) enum RetentionSelection {
    #[default]
    Limits,
    Interest,
    WorkQueue,
}

impl RetentionSelection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Limits => t("common.retention_limits"),
            Self::Interest => t("common.retention_interest"),
            Self::WorkQueue => t("common.retention_work_queue"),
        }
    }
}

#[derive(Default)]
pub(crate) struct ConsumerEditEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) stream_name: String,
    pub(crate) consumer_name: String,
    pub(crate) description: String,
    pub(crate) max_deliver: String,
    pub(crate) max_ack_pending: String,
    pub(crate) original_config: ConsumerEditableConfig,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct ConsumerEditableConfig {
    pub(crate) name: String,
    pub(crate) durable_name: Option<String>,
    pub(crate) filter_subject: Option<String>,
    pub(crate) deliver_policy: ConsumerDeliverPolicyKind,
    pub(crate) ack_policy: ConsumerAckPolicyKind,
}

impl ConsumerEditEditor {
    pub(crate) fn from_info(connection_id: u64, stream_name: String, info: &ConsumerInfo) -> Self {
        let name = info
            .durable_name
            .clone()
            .unwrap_or_else(|| info.name.clone());
        Self {
            visible: true,
            connection_id,
            stream_name,
            consumer_name: name.clone(),
            description: info.description.clone().unwrap_or_default(),
            max_deliver: info.max_deliver.to_string(),
            max_ack_pending: info.max_ack_pending.to_string(),
            original_config: ConsumerEditableConfig {
                name,
                durable_name: info.durable_name.clone(),
                filter_subject: info.filter_subject.clone(),
                deliver_policy: info.deliver_policy.clone(),
                ack_policy: ConsumerAckPolicyKind::from_display(&info.ack_policy),
            },
        }
    }
}

#[derive(Default)]
pub(crate) struct KvBucketEditEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) bucket: String,
    pub(crate) description: String,
    pub(crate) history: String,
    pub(crate) max_age_secs: String,
    pub(crate) max_value_size: String,
    pub(crate) max_bytes: String,
    pub(crate) storage: StorageKind,
    pub(crate) num_replicas: String,
}

impl KvBucketEditEditor {
    pub(crate) fn from_info(connection_id: u64, info: &KvBucketInfo) -> Self {
        let max_age = if info.max_age_secs > 0 {
            info.max_age_secs.to_string()
        } else {
            String::new()
        };
        let max_value_size = if info.max_value_size > 0 {
            info.max_value_size.to_string()
        } else {
            String::new()
        };
        let max_bytes = if info.max_bytes > 0 {
            info.max_bytes.to_string()
        } else {
            String::new()
        };
        Self {
            visible: true,
            connection_id,
            bucket: info.bucket.clone(),
            description: info.description.clone(),
            history: info.history_depth.to_string(),
            max_age_secs: max_age,
            max_value_size,
            max_bytes,
            storage: StorageKind::from_display(&info.storage),
            num_replicas: info.num_replicas.to_string(),
        }
    }
}

#[derive(Default)]
pub(crate) struct KvEntryCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) bucket_name: String,
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) value_input_format: PayloadInputFormat,
}

impl KvEntryCreateEditor {
    pub(crate) fn for_bucket(connection_id: u64, bucket_name: String, initial_key: String) -> Self {
        Self {
            visible: true,
            connection_id,
            bucket_name,
            key: initial_key,
            value: String::new(),
            value_input_format: PayloadInputFormat::Text,
        }
    }
}

pub(crate) struct ObjStoreBucketCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) bucket: String,
    pub(crate) max_bytes: String,
    pub(crate) storage: StorageSelection,
    pub(crate) num_replicas: String,
    pub(crate) description: String,
}

impl Default for ObjStoreBucketCreateEditor {
    fn default() -> Self {
        Self {
            visible: false,
            connection_id: 0,
            bucket: String::new(),
            max_bytes: String::new(),
            storage: StorageSelection::File,
            num_replicas: "1".to_string(),
            description: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectionEditor, ConnectionTestState};

    #[test]
    fn connection_test_ignores_stale_results() {
        let mut editor = ConnectionEditor::default();
        editor.start_test(2);

        assert!(!editor.complete_test(1, Ok(())));
        assert_eq!(
            editor.test_state,
            ConnectionTestState::Pending { request_id: 2 }
        );
    }

    #[test]
    fn invalidated_connection_test_ignores_its_result() {
        let mut editor = ConnectionEditor::default();
        editor.start_test(1);
        editor.invalidate_test();

        assert!(!editor.complete_test(1, Ok(())));
        assert_eq!(editor.test_state, ConnectionTestState::Idle);
    }
}
