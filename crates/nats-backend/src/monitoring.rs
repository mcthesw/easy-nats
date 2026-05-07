use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricsSection {
    Health,
    Varz,
    Connz,
    Jsz,
}

impl MetricsSection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Health => "healthz",
            Self::Varz => "varz",
            Self::Connz => "connz",
            Self::Jsz => "jsz",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSectionError {
    pub section: MetricsSection,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsHealth {
    pub ok: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarzMetrics {
    pub server_name: Option<String>,
    pub server_id: Option<String>,
    pub version: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub uptime: Option<String>,
    pub mem_bytes: u64,
    pub cpu_percent: f64,
    pub connections: u64,
    pub total_connections: u64,
    pub subscriptions: u64,
    pub slow_consumers: u64,
    pub in_msgs: u64,
    pub out_msgs: u64,
    pub in_bytes: u64,
    pub out_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnzMetrics {
    pub open_connections: u64,
    pub total_connections: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientConnectionState {
    Open,
    Closed,
    Any,
}

impl ClientConnectionState {
    pub const fn as_connz_param(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientStatusSort {
    Cid,
    Start,
    Subscriptions,
    PendingBytes,
    InMessages,
    OutMessages,
    InBytes,
    OutBytes,
    LastActivity,
    Idle,
    Uptime,
    Stop,
    Reason,
}

impl ClientStatusSort {
    pub const ALL: [Self; 13] = [
        Self::Cid,
        Self::Start,
        Self::Subscriptions,
        Self::PendingBytes,
        Self::InMessages,
        Self::OutMessages,
        Self::InBytes,
        Self::OutBytes,
        Self::LastActivity,
        Self::Idle,
        Self::Uptime,
        Self::Stop,
        Self::Reason,
    ];

    pub const fn as_connz_param(self) -> &'static str {
        match self {
            Self::Cid => "cid",
            Self::Start => "start",
            Self::Subscriptions => "subs",
            Self::PendingBytes => "pending",
            Self::InMessages => "msgs_from",
            Self::OutMessages => "msgs_to",
            Self::InBytes => "bytes_from",
            Self::OutBytes => "bytes_to",
            Self::LastActivity => "last",
            Self::Idle => "idle",
            Self::Uptime => "uptime",
            Self::Stop => "stop",
            Self::Reason => "reason",
        }
    }

    pub const fn is_allowed_for_state(self, state: ClientConnectionState) -> bool {
        match self {
            Self::Stop | Self::Reason => matches!(state, ClientConnectionState::Closed),
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStatusQuery {
    pub state: ClientConnectionState,
    pub sort: ClientStatusSort,
    pub page_size: usize,
    pub offset: usize,
    pub client_id: Option<u64>,
    pub include_subscriptions: bool,
    pub include_auth: bool,
}

impl Default for ClientStatusQuery {
    fn default() -> Self {
        Self {
            state: ClientConnectionState::Open,
            sort: ClientStatusSort::Cid,
            page_size: 100,
            offset: 0,
            client_id: None,
            include_subscriptions: false,
            include_auth: true,
        }
    }
}

impl ClientStatusQuery {
    pub const PAGE_SIZE_OPTIONS: [usize; 4] = [50, 100, 250, 500];
    pub const MAX_PAGE_SIZE: usize = 500;

    pub fn with_page(mut self, page_size: usize, offset: usize) -> Self {
        self.page_size = page_size.clamp(1, Self::MAX_PAGE_SIZE);
        self.offset = offset;
        self
    }

    pub fn detail(client_id: u64) -> Self {
        Self {
            client_id: Some(client_id),
            include_subscriptions: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStatusSubscription {
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStatusRow {
    pub client_id: u64,
    pub state: ClientConnectionState,
    pub name: Option<String>,
    pub account: Option<String>,
    pub user: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub uptime: Option<String>,
    pub idle: Option<String>,
    pub last_activity: Option<String>,
    pub rtt: Option<String>,
    pub subscriptions: Option<u64>,
    pub pending_bytes: Option<u64>,
    pub in_msgs: Option<u64>,
    pub out_msgs: Option<u64>,
    pub in_bytes: Option<u64>,
    pub out_bytes: Option<u64>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub closed_at: Option<String>,
    pub closed_reason: Option<String>,
    pub subscription_details: Vec<ClientStatusSubscription>,
}

impl ClientStatusRow {
    pub fn remote_address(&self) -> Option<String> {
        match (&self.ip, self.port) {
            (Some(ip), Some(port)) => Some(format!("{ip}:{port}")),
            (Some(ip), None) => Some(ip.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientStatusPage {
    pub endpoint: String,
    pub collected_at: SystemTime,
    pub query: ClientStatusQuery,
    pub total: u64,
    pub offset: usize,
    pub limit: usize,
    pub clients: Vec<ClientStatusRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientStatusDetail {
    pub endpoint: String,
    pub collected_at: SystemTime,
    pub query: ClientStatusQuery,
    pub client: ClientStatusRow,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientStatusRequestError {
    pub endpoint: String,
    pub collected_at: SystemTime,
    pub query: ClientStatusQuery,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetStreamMetrics {
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub streams: u64,
    pub consumers: u64,
    pub total_messages: u64,
    pub total_message_bytes: u64,
    pub api_total: u64,
    pub api_errors: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
    pub endpoint: String,
    pub collected_at: SystemTime,
    pub health: Option<MetricsHealth>,
    pub varz: Option<VarzMetrics>,
    pub connz: Option<ConnzMetrics>,
    pub jsz: Option<JetStreamMetrics>,
    pub errors: Vec<MetricsSectionError>,
}

impl MetricsSnapshot {
    pub fn has_any_data(&self) -> bool {
        self.health.is_some() || self.varz.is_some() || self.connz.is_some() || self.jsz.is_some()
    }

    pub fn is_partial(&self) -> bool {
        self.has_any_data() && !self.errors.is_empty()
    }
}
