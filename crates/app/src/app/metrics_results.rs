use nats_backend::MetricsSnapshot;

use crate::tabs::TabKind;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_metrics_snapshot(&mut self, connection_id: u64, snapshot: MetricsSnapshot) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Metrics {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.apply_snapshot(snapshot.clone());
            }
        }
    }
}
