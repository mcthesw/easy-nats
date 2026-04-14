use nats_backend::BackendCommand;

use crate::i18n::t;
use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_stream_operation(
        &mut self,
        connection_id: u64,
        operation: &str,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            "list_streams" => {
                if let Some(arr) = data.as_array() {
                    let infos = arr.clone();
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::Stream {
                            connection_id: cid,
                            stream_name,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                        {
                            state.info = infos
                                .iter()
                                .find(|i| {
                                    i["config"]["name"].as_str() == Some(stream_name.as_str())
                                })
                                .cloned();
                        }
                    }
                    self.stream_lists.insert(connection_id, infos);
                }
                true
            }
            "create_stream" | "update_stream" => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                self.backend
                    .send(BackendCommand::ListStreams { connection_id });
                true
            }
            "delete_stream" => {
                self.toasts
                    .push(ToastLevel::Success, t("toast.stream_deleted").to_string());
                self.backend
                    .send(BackendCommand::ListStreams { connection_id });
                true
            }
            "purge_stream" => {
                let purged = data["purged"].as_u64().unwrap_or(0);
                self.toasts
                    .push(ToastLevel::Success, format!("Purged {purged} messages"));
                true
            }
            "get_stream_messages" => {
                let stream_name = data["stream"].as_str().unwrap_or("").to_string();
                let messages = data["messages"].as_array().cloned().unwrap_or_default();
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::Stream {
                        connection_id: cid,
                        stream_name: sname,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *sname == stream_name
                    {
                        state.messages = messages.clone();
                        state.fetching = false;
                        state.selected_msg = None;
                    }
                }
                true
            }
            "list_consumers" => {
                let stream_name = data["stream"].as_str().unwrap_or("").to_string();
                let consumers = data["consumers"].as_array().cloned().unwrap_or_default();
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::Stream {
                        connection_id: cid,
                        stream_name: sname,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *sname == stream_name
                    {
                        state.consumers = consumers.clone();
                        state.consumers_fetching = false;
                    }
                }
                true
            }
            "create_consumer" | "delete_consumer" | "update_consumer" => {
                let stream_name = data["stream"]
                    .as_str()
                    .or_else(|| data["stream_name"].as_str())
                    .unwrap_or("")
                    .to_string();
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if !stream_name.is_empty() {
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::Stream {
                            connection_id: cid,
                            stream_name: sname,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                            && *sname == stream_name
                        {
                            state.consumers_fetching = true;
                        }
                    }
                    self.backend.send(BackendCommand::ListConsumers {
                        connection_id,
                        stream: stream_name,
                    });
                }
                self.backend
                    .send(BackendCommand::ListStreams { connection_id });
                true
            }
            "delete_stream_message" => {
                self.toasts
                    .push(ToastLevel::Success, t("toast.message_deleted").to_string());
                true
            }
            "fetch_consumer_messages" => {
                let stream_name = data["stream"].as_str().unwrap_or("").to_string();
                let consumer_name = data["consumer"].as_str().unwrap_or("").to_string();
                let messages = data["messages"].as_array().cloned().unwrap_or_default();
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::Stream {
                        connection_id: cid,
                        stream_name: sname,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *sname == stream_name
                    {
                        state.consumer_fetching.remove(&consumer_name);
                        state
                            .consumer_fetched
                            .insert(consumer_name.clone(), messages.clone());
                    }
                }
                true
            }
            _ => false,
        }
    }
}
