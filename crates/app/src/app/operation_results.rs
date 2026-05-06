use nats_backend::{BackendErrorContext, BackendOperation};

use crate::i18n::t;
use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn handle_operation_result(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
    ) {
        if operation == BackendOperation::Publish {
            self.toasts.push(
                ToastLevel::Success,
                format!("Published to {}", self.conn_name(connection_id)),
            );
            return;
        }

        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
    }

    pub(crate) fn handle_error(
        &mut self,
        connection_id: Option<u64>,
        backend_id: Option<u64>,
        operation: BackendOperation,
        message: &str,
        context: Option<&BackendErrorContext>,
    ) {
        if operation == BackendOperation::Request
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

        if operation == BackendOperation::ListConsumers
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

        if operation == BackendOperation::FetchConsumerMessages
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
            self.clear_kv_loading_on_error(cid, operation, context);
            self.clear_obj_store_loading_on_error(cid, operation);
            self.clear_server_info_loading_on_error(cid, operation);
        }

        self.toasts.push(
            ToastLevel::Error,
            operation_error_message(operation, message, context),
        );
    }
}

fn operation_error_message(
    operation: BackendOperation,
    message: &str,
    context: Option<&BackendErrorContext>,
) -> String {
    if operation == BackendOperation::FetchConsumerMessages
        && matches!(
            context,
            Some(BackendErrorContext::WorkQueueConsumerPreview { reason, .. })
                if reason == "workqueue_inspector_not_supported"
        )
    {
        return format!(
            "{operation}: {}",
            t("consumer.workqueue_preview_unsupported")
        );
    }

    format!("{operation}: {message}")
}

#[cfg(test)]
mod tests {
    use nats_backend::{BackendErrorContext, BackendOperation};

    use super::*;

    #[test]
    fn workqueue_fetch_limitation_hides_raw_server_error() {
        let context = BackendErrorContext::WorkQueueConsumerPreview {
            stream: "ORDERS".to_string(),
            consumer: "worker-a".to_string(),
            reason: "workqueue_inspector_not_supported".to_string(),
        };

        let message = operation_error_message(
            BackendOperation::FetchConsumerMessages,
            "Failed to create inspector consumer: JetStream error: filtered consumer not unique on workqueue stream",
            Some(&context),
        );

        assert!(!message.contains("filtered consumer not unique"));
        assert!(!message.contains("Failed to create inspector consumer"));
        assert!(message.contains("fetch_consumer_messages"));
    }
}
