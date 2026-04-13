use crate::i18n::t;

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
    pub(crate) nkey_seed: String,
    pub(crate) creds_path: String,
    pub(crate) cert_path: String,
    pub(crate) key_path: String,
    pub(crate) tls_enabled: bool,
    pub(crate) tls_first: bool,
    pub(crate) delete_confirm: Option<u64>,
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

pub(crate) struct ConsumerCreateEditor {
    pub(crate) visible: bool,
    pub(crate) connection_id: u64,
    pub(crate) stream_name: String,
    pub(crate) name: String,
    pub(crate) durable: bool,
    pub(crate) deliver_policy: DeliverPolicySelection,
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
}

impl DeliverPolicySelection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => t("consumer.policy_all"),
            Self::Last => t("consumer.policy_last"),
            Self::New => t("consumer.policy_new"),
        }
    }

    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Last => "Last",
            Self::New => "New",
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

    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            Self::Explicit => "Explicit",
            Self::All => "All",
            Self::None => "None",
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
