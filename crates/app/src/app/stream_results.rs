use nats_backend::{BackendCommand, BackendOperation};

use crate::i18n::t;
use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_stream_operation(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            BackendOperation::ListStreams => {
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
            BackendOperation::CreateStream | BackendOperation::UpdateStream => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                self.backend
                    .send(BackendCommand::ListStreams { connection_id });
                true
            }
            BackendOperation::DeleteStream => {
                self.toasts
                    .push(ToastLevel::Success, t("toast.stream_deleted").to_string());
                if let Some(name) = data["name"].as_str() {
                    self.remove_tabs_matching(|tab| {
                        matches!(tab, TabKind::Stream { connection_id: cid, stream_name, .. }
                            if *cid == connection_id && stream_name == name)
                    });
                }
                self.backend
                    .send(BackendCommand::ListStreams { connection_id });
                true
            }
            BackendOperation::PurgeStream => {
                let purged = data["purged"].as_u64().unwrap_or(0);
                self.toasts
                    .push(ToastLevel::Success, format!("Purged {purged} messages"));
                true
            }
            BackendOperation::GetStreamMessages => {
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
                        state.search_generation = state.search_generation.wrapping_add(1);
                        state.cached_filtered = None;
                    }
                }
                true
            }
            BackendOperation::ListConsumers => {
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
            BackendOperation::CreateConsumer
            | BackendOperation::DeleteConsumer
            | BackendOperation::UpdateConsumer => {
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
            BackendOperation::DeleteStreamMessage => {
                self.toasts
                    .push(ToastLevel::Success, t("toast.message_deleted").to_string());
                true
            }
            BackendOperation::FetchConsumerMessages => {
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
