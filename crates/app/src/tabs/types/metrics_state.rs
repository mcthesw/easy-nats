use std::collections::VecDeque;
use web_time::Instant;

use nats_backend::MetricsSnapshot;

use super::AutoRefresh;

#[derive(Debug, Default)]
pub struct MetricsState {
    endpoint: String,
    latest_attempt: Option<Box<MetricsSnapshot>>,
    history: VecDeque<MetricsSnapshot>,
    pub loading: bool,
    pub auto_refresh: AutoRefresh,
}

impl MetricsState {
    const MAX_SAMPLES: usize = 720;

    pub fn with_endpoint(endpoint: String) -> Self {
        let mut state = Self {
            endpoint,
            ..Default::default()
        };
        state.auto_refresh.enabled = true;
        state.auto_refresh.interval_secs = 5;
        state
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    pub fn endpoint_configured(&self) -> bool {
        !self.endpoint.trim().is_empty()
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        if self.endpoint == endpoint {
            return;
        }
        self.endpoint = endpoint;
        self.latest_attempt = None;
        self.history.clear();
        self.loading = false;
        self.auto_refresh.last_refresh = Instant::now();
    }

    pub fn begin_refresh(&mut self) {
        self.loading = true;
        self.auto_refresh.mark_refreshed();
    }

    pub fn apply_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.endpoint = snapshot.endpoint.clone();
        self.loading = false;
        if snapshot.has_any_data() {
            if self.history.len() >= Self::MAX_SAMPLES {
                self.history.pop_front();
            }
            self.history.push_back(snapshot.clone());
        }
        self.latest_attempt = Some(Box::new(snapshot));
    }

    pub fn has_never_loaded(&self) -> bool {
        self.latest_attempt.is_none() && self.history.is_empty()
    }

    pub fn latest_attempt(&self) -> Option<&MetricsSnapshot> {
        self.latest_attempt.as_deref()
    }

    pub fn latest_data(&self) -> Option<&MetricsSnapshot> {
        self.latest_attempt
            .as_deref()
            .filter(|snapshot| snapshot.has_any_data())
            .or_else(|| self.history.back())
    }

    pub fn history(&self) -> &VecDeque<MetricsSnapshot> {
        &self.history
    }

    pub fn is_stale(&self) -> bool {
        self.latest_attempt
            .as_deref()
            .is_some_and(|snapshot| !snapshot.has_any_data())
            && !self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use nats_backend::{MetricsSection, MetricsSectionError, MetricsSnapshot, VarzMetrics};

    use super::MetricsState;

    fn sample_snapshot(endpoint: &str, collected_at: SystemTime, in_msgs: u64) -> MetricsSnapshot {
        MetricsSnapshot {
            endpoint: endpoint.to_string(),
            collected_at,
            health: None,
            varz: Some(VarzMetrics {
                server_name: Some("local".to_string()),
                server_id: Some("server-1".to_string()),
                version: Some("2.11.0".to_string()),
                host: Some("127.0.0.1".to_string()),
                port: Some(4222),
                uptime: Some("1m".to_string()),
                mem_bytes: 1024,
                cpu_percent: 1.0,
                connections: 2,
                total_connections: 3,
                subscriptions: 4,
                slow_consumers: 0,
                in_msgs,
                out_msgs: in_msgs,
                in_bytes: 2048,
                out_bytes: 2048,
            }),
            connz: None,
            jsz: None,
            errors: Vec::new(),
        }
    }

    #[test]
    fn metrics_state_marks_last_good_sample_stale_after_failed_refresh() {
        let mut state = MetricsState::with_endpoint("http://localhost:8222".to_string());
        let first = sample_snapshot("http://localhost:8222", SystemTime::UNIX_EPOCH, 10);
        state.apply_snapshot(first);

        state.apply_snapshot(MetricsSnapshot {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            health: None,
            varz: None,
            connz: None,
            jsz: None,
            errors: vec![MetricsSectionError {
                section: MetricsSection::Varz,
                message: "timeout".to_string(),
            }],
        });

        assert!(state.is_stale());
        assert_eq!(state.history().len(), 1);
        assert_eq!(
            state.latest_data().unwrap().varz.as_ref().unwrap().in_msgs,
            10
        );
    }

    #[test]
    fn metrics_state_clears_history_when_endpoint_changes() {
        let mut state = MetricsState::with_endpoint("http://localhost:8222".to_string());
        state.apply_snapshot(sample_snapshot(
            "http://localhost:8222",
            SystemTime::UNIX_EPOCH,
            10,
        ));

        state.set_endpoint("http://localhost:9222".to_string());

        assert!(state.history().is_empty());
        assert!(state.latest_attempt().is_none());
        assert_eq!(state.endpoint(), "http://localhost:9222");
    }

    #[test]
    fn metrics_state_defaults_to_enabled_auto_refresh_every_five_seconds() {
        let state = MetricsState::with_endpoint("http://localhost:8222".to_string());

        assert!(state.auto_refresh.enabled);
        assert_eq!(state.auto_refresh.interval_secs, 5);
    }

    #[test]
    fn metrics_state_keeps_one_hour_at_default_refresh() {
        let mut state = MetricsState::with_endpoint("http://localhost:8222".to_string());
        for idx in 0..=MetricsState::MAX_SAMPLES {
            state.apply_snapshot(sample_snapshot(
                "http://localhost:8222",
                SystemTime::UNIX_EPOCH + Duration::from_secs(idx as u64 * 5),
                idx as u64,
            ));
        }

        assert_eq!(state.history().len(), MetricsState::MAX_SAMPLES);
        assert_eq!(
            state
                .history()
                .front()
                .unwrap()
                .varz
                .as_ref()
                .unwrap()
                .in_msgs,
            1
        );
    }
}
