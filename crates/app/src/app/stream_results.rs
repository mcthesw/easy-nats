use nats_backend::{BackendCommand, BackendOperation, ConsumerInfo, StreamInfo, StreamMessageInfo};

use crate::i18n::t;
use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_streams(&mut self, connection_id: u64, streams: Vec<StreamInfo>) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Stream {
                connection_id: cid,
                stream_name,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.info = streams
                    .iter()
                    .find(|info| info.name == *stream_name)
                    .cloned();
            }
        }
        self.stream_lists.insert(connection_id, streams);
    }

    pub(crate) fn apply_stream_changed(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        stream: StreamInfo,
    ) {
        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
        upsert_stream(
            self.stream_lists.entry(connection_id).or_default(),
            stream.clone(),
        );
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Stream {
                connection_id: cid,
                stream_name,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *stream_name == stream.name
            {
                state.info = Some(stream.clone());
            }
        }
        self.backend
            .send(BackendCommand::ListStreams { connection_id });
    }

    pub(crate) fn apply_stream_deleted(&mut self, connection_id: u64, name: String) {
        self.toasts
            .push(ToastLevel::Success, t("toast.stream_deleted").to_string());
        if let Some(streams) = self.stream_lists.get_mut(&connection_id) {
            streams.retain(|stream| stream.name != name);
        }
        self.remove_tabs_matching(|tab| {
            matches!(tab, TabKind::Stream { connection_id: cid, stream_name, .. }
                if *cid == connection_id && stream_name == &name)
        });
        self.backend
            .send(BackendCommand::ListStreams { connection_id });
    }

    pub(crate) fn apply_stream_purged(&mut self, _connection_id: u64, _name: String, purged: u64) {
        self.toasts
            .push(ToastLevel::Success, format!("Purged {purged} messages"));
    }

    pub(crate) fn apply_stream_messages(
        &mut self,
        connection_id: u64,
        stream_name: String,
        messages: Vec<StreamMessageInfo>,
    ) {
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
    }

    pub(crate) fn apply_consumers(
        &mut self,
        connection_id: u64,
        stream_name: String,
        consumers: Vec<ConsumerInfo>,
    ) {
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
    }

    pub(crate) fn apply_consumer_changed(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        stream_name: String,
    ) {
        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
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
        self.backend
            .send(BackendCommand::ListStreams { connection_id });
    }

    pub(crate) fn apply_stream_message_deleted(
        &mut self,
        _connection_id: u64,
        _stream: String,
        _sequence: u64,
    ) {
        self.toasts
            .push(ToastLevel::Success, t("toast.message_deleted").to_string());
    }

    pub(crate) fn apply_consumer_messages(
        &mut self,
        connection_id: u64,
        stream_name: String,
        consumer_name: String,
        messages: Vec<StreamMessageInfo>,
    ) {
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
    }
}

fn upsert_stream(streams: &mut Vec<StreamInfo>, stream: StreamInfo) {
    if let Some(existing) = streams.iter_mut().find(|info| info.name == stream.name) {
        *existing = stream;
    } else {
        streams.push(stream);
        streams.sort_by(|a, b| a.name.cmp(&b.name));
    }
}
