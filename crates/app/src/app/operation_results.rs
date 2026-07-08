use nats_backend::{BackendErrorContext, BackendOperation};

use crate::i18n::t;
use crate::tabs::{PreviewFetchState, SearchSourceId, TabKind};
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
        _backend_id: Option<u64>,
        operation: BackendOperation,
        message: &str,
        context: Option<&BackendErrorContext>,
    ) {
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

        if operation == BackendOperation::GetKvEntry {
            self.mark_search_workspace_fetch_failed(connection_id, context, message);
        }

        self.toasts.push(
            ToastLevel::Error,
            operation_error_message(operation, message, context),
        );
    }

    fn mark_search_workspace_fetch_failed(
        &mut self,
        connection_id: Option<u64>,
        context: Option<&BackendErrorContext>,
        message: &str,
    ) {
        let (failed_bucket, failed_key) = match context {
            Some(BackendErrorContext::KvEntry { bucket, key }) => {
                (Some(bucket.as_str()), Some(key.as_str()))
            }
            _ => (None, None),
        };

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::SearchWorkspace { state } = tab {
                let fetch_state = state.preview_fetch.clone();
                if let PreviewFetchState::Loading(key) = &fetch_state {
                    let matches = match (&key.source_id, failed_bucket, failed_key) {
                        (
                            SearchSourceId::Kv {
                                connection_id: cid,
                                bucket_name,
                            },
                            Some(bucket),
                            Some(k),
                        ) => {
                            *cid == connection_id.unwrap_or(0)
                                && bucket_name == bucket
                                && key.item_id == k
                        }
                        (
                            SearchSourceId::Kv {
                                connection_id: cid, ..
                            },
                            None,
                            _,
                        ) => Some(*cid) == connection_id,
                        _ => false,
                    };
                    if matches {
                        state.preview_fetch = PreviewFetchState::Failed {
                            key: key.clone(),
                            message: message.to_string(),
                        };
                    }
                }
            }
        }
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
    use std::sync::{Arc, Mutex};

    use egui_dock::DockState;
    use nats_backend::{BackendErrorContext, BackendOperation};

    use super::*;
    use crate::log_layer::LogBuffer;
    use crate::settings::AppSettings;
    use crate::tabs::types::{SearchField, SearchResultKey};
    use crate::tabs::{SearchSourceId, SearchWorkspaceState};
    use crate::theme::ThemeId;

    fn test_app_with_search_workspace(preview_fetch: PreviewFetchState) -> EasyNatsApp {
        let mut app = EasyNatsApp::new(
            AppSettings::default(),
            ThemeId::EguiDark,
            Arc::new(Mutex::new(LogBuffer::default())),
        );
        app.dock_state = DockState::new(vec![TabKind::SearchWorkspace {
            state: SearchWorkspaceState {
                preview_fetch,
                ..Default::default()
            },
        }]);
        app
    }

    fn kv_loading_key(connection_id: u64, bucket: &str, key: &str) -> SearchResultKey {
        SearchResultKey {
            source_id: SearchSourceId::Kv {
                connection_id,
                bucket_name: bucket.to_string(),
            },
            field: SearchField::Key,
            item_id: key.to_string(),
        }
    }

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

    #[test]
    fn get_kv_entry_error_marks_matching_search_workspace_fetch_failed() {
        let key = kv_loading_key(7, "ORDERS", "orders/1");
        let mut app = test_app_with_search_workspace(PreviewFetchState::Loading(key.clone()));
        let context = BackendErrorContext::KvEntry {
            bucket: "ORDERS".to_string(),
            key: "orders/1".to_string(),
        };

        app.handle_error(
            Some(7),
            None,
            BackendOperation::GetKvEntry,
            "connection refused",
            Some(&context),
        );

        for (_, tab) in app.dock_state.iter_all_tabs() {
            if let TabKind::SearchWorkspace { state } = tab {
                match &state.preview_fetch {
                    PreviewFetchState::Failed { key: k, message } => {
                        assert_eq!(k, &key);
                        assert_eq!(message, "connection refused");
                    }
                    _ => panic!("expected Failed state, got {:?}", state.preview_fetch),
                }
                return;
            }
        }
        panic!("no SearchWorkspace tab found");
    }

    #[test]
    fn get_kv_entry_error_without_context_marks_connection_loading_failed() {
        let key = kv_loading_key(7, "ORDERS", "orders/1");
        let mut app = test_app_with_search_workspace(PreviewFetchState::Loading(key.clone()));

        app.handle_error(
            Some(7),
            None,
            BackendOperation::GetKvEntry,
            "not connected",
            None,
        );

        for (_, tab) in app.dock_state.iter_all_tabs() {
            if let TabKind::SearchWorkspace { state } = tab {
                match &state.preview_fetch {
                    PreviewFetchState::Failed { key: k, message } => {
                        assert_eq!(k, &key);
                        assert_eq!(message, "not connected");
                    }
                    _ => panic!("expected Failed state, got {:?}", state.preview_fetch),
                }
                return;
            }
        }
        panic!("no SearchWorkspace tab found");
    }

    #[test]
    fn get_kv_entry_error_does_not_mark_unrelated_search_workspace_fetch() {
        // Loading a different key — should not be marked as failed.
        let other_key = kv_loading_key(7, "ORDERS", "orders/2");
        let mut app = test_app_with_search_workspace(PreviewFetchState::Loading(other_key));
        let context = BackendErrorContext::KvEntry {
            bucket: "ORDERS".to_string(),
            key: "orders/1".to_string(),
        };

        app.handle_error(
            Some(7),
            None,
            BackendOperation::GetKvEntry,
            "connection refused",
            Some(&context),
        );

        for (_, tab) in app.dock_state.iter_all_tabs() {
            if let TabKind::SearchWorkspace { state } = tab {
                // Should still be Loading (not Failed) since the key doesn't match
                assert!(matches!(
                    &state.preview_fetch,
                    PreviewFetchState::Loading(_)
                ));
                return;
            }
        }
        panic!("no SearchWorkspace tab found");
    }

    #[test]
    fn get_kv_entry_error_does_not_mark_idle_search_workspace() {
        let mut app = test_app_with_search_workspace(PreviewFetchState::Idle);
        let context = BackendErrorContext::KvEntry {
            bucket: "ORDERS".to_string(),
            key: "orders/1".to_string(),
        };

        app.handle_error(
            Some(7),
            None,
            BackendOperation::GetKvEntry,
            "connection refused",
            Some(&context),
        );

        for (_, tab) in app.dock_state.iter_all_tabs() {
            if let TabKind::SearchWorkspace { state } = tab {
                assert!(matches!(state.preview_fetch, PreviewFetchState::Idle));
                return;
            }
        }
        panic!("no SearchWorkspace tab found");
    }
}
