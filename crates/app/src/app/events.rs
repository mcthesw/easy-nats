use eframe::egui;
use nats_backend::{BackendCommand, BackendEvent, ConnectionStatusKind};

use crate::tabs::{ReceivedMessage, ResponseData, TabKind};
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn handle_events(&mut self, ctx: &egui::Context) {
        let events = self.backend.drain_events();
        if events.is_empty() {
            return;
        }

        for event in events {
            match event {
                BackendEvent::ConnectionStatus {
                    connection_id,
                    status,
                } => {
                    tracing::info!(connection_id, ?status, "Connection status changed");
                    let prev = self.conn_statuses.get(&connection_id).cloned();
                    match &status {
                        ConnectionStatusKind::Connected => {
                            // Guard against duplicate Connected events from the NATS event_callback
                            if !matches!(prev, Some(ConnectionStatusKind::Connected)) {
                                self.toasts.push(
                                    ToastLevel::Success,
                                    format!("Connected to {}", self.conn_name(connection_id)),
                                );
                                self.backend
                                    .send(BackendCommand::ListStreams { connection_id });
                                self.backend
                                    .send(BackendCommand::ListKvBuckets { connection_id });
                                self.backend
                                    .send(BackendCommand::ListObjectStoreBuckets { connection_id });
                            }
                        }
                        ConnectionStatusKind::Disconnected => {
                            self.stream_lists.remove(&connection_id);
                            self.kv_lists.remove(&connection_id);
                            self.obj_store_lists.remove(&connection_id);
                        }
                        ConnectionStatusKind::Error(msg) => {
                            self.toasts.push(
                                ToastLevel::Error,
                                format!("{}: {}", self.conn_name(connection_id), msg),
                            );
                        }
                        _ => {}
                    }
                    self.conn_statuses.insert(connection_id, status);
                }
                BackendEvent::OperationResult {
                    connection_id,
                    operation,
                    data,
                } => {
                    self.handle_operation_result(connection_id, &operation, data);
                }
                BackendEvent::RequestResponse {
                    connection_id,
                    payload,
                    headers,
                } => {
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::Publisher {
                            connection_id: cid,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                        {
                            state.response = Some(ResponseData {
                                payload: payload.clone(),
                                headers: headers.clone(),
                            });
                            state.waiting = false;
                        }
                    }
                }
                BackendEvent::MessageReceived {
                    connection_id,
                    subject,
                    reply,
                    headers,
                    payload,
                    timestamp,
                } => {
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::Subscriber {
                            connection_id: cid,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                            && state.has_active_subscription()
                        {
                            state.push_message(ReceivedMessage {
                                subject: subject.clone(),
                                reply: reply.clone(),
                                headers: headers.clone(),
                                payload: payload.clone(),
                                timestamp,
                            });
                        }
                    }
                }
                BackendEvent::Error {
                    connection_id,
                    operation,
                    message,
                } => {
                    self.handle_error(connection_id, &operation, &message);
                }
            }
        }

        ctx.request_repaint();
    }
}
