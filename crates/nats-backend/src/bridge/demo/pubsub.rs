use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use web_time::Instant;

use crate::event::{
    BackendEvent, BackendOperation, ConnectionStatusKind, MessageData, RequestFailureKind,
};
use crate::models::StreamMessageInfo;

use super::fixtures::{self, DEMO_CONNECTION_ID, DEMO_TIME};
use super::{DemoState, STREAM_MESSAGE_LIMIT, SYNTHETIC_INTERVAL, subject_matches};

impl DemoState {
    pub(super) fn connect(&mut self, connection_id: u64) {
        self.push(BackendEvent::ConnectionStatus {
            connection_id,
            status: ConnectionStatusKind::Connected,
        });
    }

    pub(super) fn disconnect(&mut self, connection_id: u64) {
        self.push(BackendEvent::ConnectionStatus {
            connection_id,
            status: ConnectionStatusKind::Disconnected,
        });
    }

    pub(super) fn publish(
        &mut self,
        connection_id: u64,
        subject: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
    ) {
        self.publish_message(connection_id, subject, payload, headers);
        self.succeeded(connection_id, BackendOperation::Publish);
    }

    pub(super) fn subscribe(&mut self, connection_id: u64, backend_id: u64, subject: String) {
        self.subscriptions
            .entry((connection_id, backend_id))
            .or_default()
            .insert(subject);
        self.succeeded(connection_id, BackendOperation::Subscribe);
    }

    pub(super) fn unsubscribe(&mut self, connection_id: u64, backend_id: u64, subject: String) {
        if let Some(subjects) = self.subscriptions.get_mut(&(connection_id, backend_id)) {
            subjects.remove(&subject);
        }
        self.succeeded(connection_id, BackendOperation::Unsubscribe);
    }

    pub(super) fn request(
        &mut self,
        connection_id: u64,
        backend_id: u64,
        request_id: u64,
        subject: String,
        payload: Vec<u8>,
    ) {
        if subject.trim().is_empty() {
            self.push(BackendEvent::RequestFailed {
                connection_id,
                backend_id,
                request_id,
                message: "Subject is required".into(),
                kind: RequestFailureKind::Other,
            });
            return;
        }

        let request = String::from_utf8_lossy(&payload);
        self.push(BackendEvent::RequestResponse {
            connection_id,
            backend_id,
            request_id,
            subject: Some(subject),
            payload: format!(
                r#"{{"ok":true,"echo":{},"source":"interactive-demo"}}"#,
                serde_json::to_string(request.as_ref()).unwrap_or_default()
            )
            .into_bytes(),
            headers: vec![("content-type".into(), "application/json".into())],
        });
    }

    pub(super) fn reply(
        &mut self,
        connection_id: u64,
        backend_id: u64,
        reply_id: u64,
        reply_to: String,
    ) {
        self.push(BackendEvent::Replied {
            connection_id,
            backend_id,
            reply_id,
            subject: reply_to,
        });
    }

    pub(super) fn enqueue_synthetic_message(&mut self) {
        let now = Instant::now();
        if now < self.next_synthetic {
            return;
        }
        self.next_synthetic = now + SYNTHETIC_INTERVAL;
        self.synthetic_count += 1;
        let sequence = self.synthetic_count;
        let (subject, payload) = match (sequence as usize - 1) % 3 {
            0 => (
                "orders.created",
                format!(
                    r#"{{"order_id":"ord-live-{sequence:04}","customer":"demo","total":{}.00,"status":"created"}}"#,
                    100 + sequence * 7
                ),
            ),
            1 => (
                "audit.order.created",
                format!(
                    r#"{{"order_id":"ord-live-{sequence:04}","actor":"demo-generator","action":"created"}}"#
                ),
            ),
            _ => (
                "telemetry.orders-api.latency",
                format!(
                    r#"{{"service":"orders-api","metric":"latency_ms","value":{},"status":"ok"}}"#,
                    35 + sequence % 20
                ),
            ),
        };
        self.publish_message(
            DEMO_CONNECTION_ID,
            subject.into(),
            payload.into_bytes(),
            None,
        );
    }

    fn publish_message(
        &mut self,
        connection_id: u64,
        subject: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
    ) {
        let message = MessageData {
            subject: subject.clone(),
            reply: None,
            headers: headers
                .unwrap_or_else(|| vec![("content-type".into(), "application/json".into())]),
            payload: payload.clone(),
            timestamp: fixtures::system_time(),
        };
        let recipients: Vec<u64> = self
            .subscriptions
            .iter()
            .filter(|((id, _), patterns)| {
                *id == connection_id
                    && patterns
                        .iter()
                        .any(|pattern| subject_matches(pattern, &subject))
            })
            .map(|((_, backend_id), _)| *backend_id)
            .collect();
        for backend_id in recipients {
            self.push(BackendEvent::MessageBatch {
                connection_id,
                backend_id,
                messages: vec![message.clone()],
            });
        }

        let stream_names: Vec<String> = self
            .streams
            .iter()
            .filter(|(_, stream)| {
                stream
                    .subjects
                    .iter()
                    .any(|pattern| subject_matches(pattern, &subject))
            })
            .map(|(name, _)| name.clone())
            .collect();
        let stored_at = OffsetDateTime::from(message.timestamp)
            .format(&Rfc3339)
            .unwrap_or_else(|_| DEMO_TIME.into());
        for stream_name in stream_names {
            let sequence = self
                .streams
                .get(&stream_name)
                .map_or(1, |stream| stream.last_sequence.saturating_add(1));
            let messages = self.stream_messages.entry(stream_name.clone()).or_default();
            messages.push(StreamMessageInfo {
                sequence,
                subject: subject.clone(),
                payload: payload.clone(),
                headers: message.headers.clone(),
                time: stored_at.clone(),
            });
            if messages.len() > STREAM_MESSAGE_LIMIT {
                messages.remove(0);
            }
            if let Some(stream) = self.streams.get_mut(&stream_name) {
                stream.messages = stream.messages.saturating_add(1);
                stream.bytes = stream.bytes.saturating_add(payload.len() as u64);
                if stream.first_sequence == 0 {
                    stream.first_sequence = sequence;
                }
                stream.last_sequence = sequence;
            }
        }
    }
}
