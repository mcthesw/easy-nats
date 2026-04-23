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
