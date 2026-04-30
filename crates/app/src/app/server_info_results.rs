use nats_backend::{BackendOperation, JetStreamAccountInfoSnapshot, ServerInfoSnapshot};

use crate::tabs::TabKind;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_server_info(&mut self, connection_id: u64, info: ServerInfoSnapshot) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::ServerInfo {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.server_info = Some(info.clone());
                state.loading = false;
            }
        }
    }

    pub(crate) fn apply_jetstream_account_info(
        &mut self,
        connection_id: u64,
        info: JetStreamAccountInfoSnapshot,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::ServerInfo {
                connection_id: cid,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.account_info = Some(info.clone());
                state.loading = false;
            }
        }
    }

    pub(crate) fn clear_server_info_loading_on_error(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
    ) {
        if matches!(
            operation,
            BackendOperation::ServerInfo | BackendOperation::JetStreamAccountInfo
        ) {
            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                if let TabKind::ServerInfo {
                    connection_id: cid,
                    state,
                    ..
                } = tab
                    && *cid == connection_id
                {
                    state.loading = false;
                }
            }
        }
    }
}
