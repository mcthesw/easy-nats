use nats_backend::{
    ClientStatusDetail, ClientStatusPage, ClientStatusRequestError, MetricsSnapshot,
};

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

    pub(crate) fn apply_client_status_page(&mut self, connection_id: u64, page: ClientStatusPage) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Clients {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.apply_page(page.clone());
            }
        }
    }

    pub(crate) fn apply_client_status_detail(
        &mut self,
        connection_id: u64,
        detail: ClientStatusDetail,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Clients {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.apply_detail(detail.clone());
            }
        }
    }

    pub(crate) fn apply_client_status_error(
        &mut self,
        connection_id: u64,
        error: ClientStatusRequestError,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Clients {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.apply_error(error.clone());
            }
        }
    }
}
