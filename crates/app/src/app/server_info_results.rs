use crate::tabs::TabKind;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_server_info_operation(
        &mut self,
        connection_id: u64,
        operation: &str,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            "server_info" => {
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::ServerInfo {
                        connection_id: cid,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                    {
                        state.server_info = Some(data.clone());
                        state.loading = false;
                    }
                }
                true
            }
            "jetstream_account_info" => {
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::ServerInfo {
                        connection_id: cid,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                    {
                        state.account_info = Some(data.clone());
                        state.loading = false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn clear_server_info_loading_on_error(
        &mut self,
        connection_id: u64,
        operation: &str,
    ) {
        if matches!(operation, "server_info" | "jetstream_account_info") {
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
