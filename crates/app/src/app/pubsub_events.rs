use nats_backend::{MessageData, RequestFailureKind};

use crate::tabs::{ReceivedMessage, ResponseData, TabKind};

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_publisher_request_response(
        &mut self,
        connection_id: u64,
        response_backend_id: u64,
        request_id: u64,
        subject: Option<String>,
        payload: Vec<u8>,
        headers: Vec<(String, String)>,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Publisher {
                connection_id: cid,
                backend_id,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *backend_id == response_backend_id
            {
                state.apply_request_response(
                    request_id,
                    ResponseData {
                        subject,
                        payload,
                        headers,
                    },
                );
                break;
            }
        }
    }

    pub(crate) fn apply_publisher_request_failed(
        &mut self,
        connection_id: u64,
        response_backend_id: u64,
        request_id: u64,
        kind: RequestFailureKind,
        message: String,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Publisher {
                connection_id: cid,
                backend_id,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *backend_id == response_backend_id
            {
                state.apply_request_failure(request_id, kind, message);
                break;
            }
        }
    }

    pub(crate) fn apply_subscriber_reply_success(
        &mut self,
        connection_id: u64,
        reply_backend_id: u64,
        reply_id: u64,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Subscriber {
                connection_id: cid,
                backend_id,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *backend_id == reply_backend_id
            {
                state.apply_reply_success(reply_id);
                break;
            }
        }
    }

    pub(crate) fn apply_subscriber_reply_failed(
        &mut self,
        connection_id: u64,
        reply_backend_id: u64,
        reply_id: u64,
        message: String,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Subscriber {
                connection_id: cid,
                backend_id,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *backend_id == reply_backend_id
            {
                state.apply_reply_failure(reply_id, message);
                break;
            }
        }
    }

    pub(crate) fn apply_subscriber_message_batch(
        &mut self,
        connection_id: u64,
        msg_backend_id: u64,
        messages: Vec<MessageData>,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Subscriber {
                connection_id: cid,
                backend_id,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *backend_id == msg_backend_id
            {
                state.push_messages(messages.into_iter().map(|message| {
                    ReceivedMessage::new(
                        message.subject,
                        message.reply,
                        message.headers,
                        message.payload,
                        message.timestamp,
                    )
                }));
                break;
            }
        }
    }
}
