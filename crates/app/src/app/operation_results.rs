use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn handle_operation_result(
        &mut self,
        connection_id: u64,
        operation: &str,
        data: serde_json::Value,
    ) {
        if operation == "publish" {
            self.toasts.push(
                ToastLevel::Success,
                format!("Published to {}", self.conn_name(connection_id)),
            );
            return;
        }

        if self.apply_stream_operation(connection_id, operation, &data)
            || self.apply_kv_operation(connection_id, operation, &data)
            || self.apply_obj_store_operation(connection_id, operation, &data)
            || self.apply_server_info_operation(connection_id, operation, &data)
        {
            return;
        }

        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
    }

    pub(crate) fn handle_error(
        &mut self,
        connection_id: Option<u64>,
        backend_id: Option<u64>,
        operation: &str,
        message: &str,
    ) {
        if operation == "request"
            && let Some(cid) = connection_id
        {
            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                if let TabKind::Publisher {
                    connection_id: tab_cid,
                    backend_id: tab_backend_id,
                    state,
                    ..
                } = tab
                    && *tab_cid == cid
                    && backend_id.is_none_or(|backend_id| *tab_backend_id == backend_id)
                {
                    state.waiting = false;
                }
            }
        }

        if operation == "list_consumers"
            && let Some(cid) = connection_id
        {
            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                if let TabKind::Stream {
                    connection_id: tab_cid,
                    state,
                    ..
                } = tab
                    && *tab_cid == cid
                {
                    state.consumers_fetching = false;
                }
            }
        }

        if operation == "fetch_consumer_messages"
            && let Some(cid) = connection_id
        {
            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                if let TabKind::Stream {
                    connection_id: tab_cid,
                    state,
                    ..
                } = tab
                    && *tab_cid == cid
                {
                    state.consumer_fetching.clear();
                }
            }
        }

        if let Some(cid) = connection_id {
            self.clear_kv_loading_on_error(cid, operation);
            self.clear_obj_store_loading_on_error(cid, operation);
            self.clear_server_info_loading_on_error(cid, operation);
        }

        self.toasts
            .push(ToastLevel::Error, format!("{operation}: {message}"));
    }
}
