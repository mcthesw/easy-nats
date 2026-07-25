use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{ConsumerConfigInput, ConsumerInfo, StreamConfigInput, StreamInfo};

use super::{DemoState, fixtures, subject_matches};

impl DemoState {
    pub(super) fn list_streams(&mut self, connection_id: u64) {
        self.push(BackendEvent::StreamsListed {
            connection_id,
            streams: self.streams.values().cloned().collect(),
        });
    }

    pub(super) fn upsert_stream(&mut self, connection_id: u64, config: StreamConfigInput) {
        let created = !self.streams.contains_key(&config.name);
        let stream = StreamInfo {
            name: config.name.clone(),
            subjects: config.subjects,
            storage: fixtures::storage_label(config.storage),
            retention: fixtures::retention_label(config.retention),
            messages: 0,
            bytes: 0,
            first_sequence: 0,
            last_sequence: 0,
            consumer_count: 0,
        };
        self.streams.insert(config.name, stream.clone());
        self.push(if created {
            BackendEvent::StreamCreated {
                connection_id,
                stream,
            }
        } else {
            BackendEvent::StreamUpdated {
                connection_id,
                stream,
            }
        });
    }

    pub(super) fn delete_stream(&mut self, connection_id: u64, name: String) {
        self.streams.remove(&name);
        self.stream_messages.remove(&name);
        self.consumers.remove(&name);
        self.push(BackendEvent::StreamDeleted {
            connection_id,
            name,
        });
    }

    pub(super) fn purge_stream(
        &mut self,
        connection_id: u64,
        name: String,
        filter: Option<String>,
    ) {
        let purged = {
            let messages = self.stream_messages.entry(name.clone()).or_default();
            let before = messages.len();
            if let Some(filter) = filter {
                messages.retain(|message| !subject_matches(&filter, &message.subject));
            } else {
                messages.clear();
            }
            (before - messages.len()) as u64
        };
        self.push(BackendEvent::StreamPurged {
            connection_id,
            name,
            purged,
        });
    }

    pub(super) fn get_stream_messages(
        &mut self,
        connection_id: u64,
        stream: String,
        start_sequence: Option<u64>,
        subject_filter: Option<String>,
        start_time: Option<String>,
        batch_size: u64,
    ) {
        let start_time = match start_time {
            Some(value) => match OffsetDateTime::parse(&value, &Rfc3339) {
                Ok(parsed) => Some(parsed),
                Err(error) => {
                    self.push(BackendEvent::Error {
                        connection_id: Some(connection_id),
                        backend_id: None,
                        operation: BackendOperation::GetStreamMessages,
                        message: format!("Invalid time format (use RFC3339): {error}"),
                        context: None,
                    });
                    return;
                }
            },
            None => None,
        };
        let messages = self
            .stream_messages
            .get(&stream)
            .into_iter()
            .flatten()
            .filter(|message| {
                if let Some(start_time) = start_time {
                    return OffsetDateTime::parse(&message.time, &Rfc3339)
                        .is_ok_and(|published| published >= start_time);
                }
                start_sequence.is_none_or(|start| message.sequence >= start)
            })
            .filter(|message| {
                subject_filter
                    .as_ref()
                    .is_none_or(|filter| subject_matches(filter, &message.subject))
            })
            .take(batch_size as usize)
            .cloned()
            .collect();
        self.push(BackendEvent::StreamMessagesFetched {
            connection_id,
            stream,
            messages,
        });
    }

    pub(super) fn delete_stream_message(
        &mut self,
        connection_id: u64,
        stream: String,
        sequence: u64,
    ) {
        if let Some(messages) = self.stream_messages.get_mut(&stream) {
            messages.retain(|message| message.sequence != sequence);
        }
        self.push(BackendEvent::StreamMessageDeleted {
            connection_id,
            stream,
            sequence,
        });
    }

    pub(super) fn list_consumers(&mut self, connection_id: u64, stream: String) {
        let consumers = self
            .consumers
            .get(&stream)
            .into_iter()
            .flat_map(|items| items.values().cloned())
            .collect();
        self.push(BackendEvent::ConsumersListed {
            connection_id,
            stream,
            consumers,
        });
    }

    pub(super) fn upsert_consumer(
        &mut self,
        connection_id: u64,
        stream: String,
        config: ConsumerConfigInput,
    ) {
        let items = self.consumers.entry(stream.clone()).or_default();
        let created = !items.contains_key(&config.name);
        let consumer = ConsumerInfo {
            name: config.name.clone(),
            stream_name: stream.clone(),
            durable_name: config.durable_name,
            filter_subject: config.filter_subject,
            deliver_policy: config.deliver_policy,
            ack_policy: fixtures::ack_label(config.ack_policy),
            max_deliver: config.max_deliver.unwrap_or(-1),
            max_ack_pending: config.max_ack_pending.unwrap_or(1_000),
            description: config.description,
            deliver_subject: None,
            num_pending: 0,
            num_ack_pending: 0,
            num_waiting: 0,
            num_redelivered: 0,
            push_bound: false,
        };
        items.insert(config.name, consumer.clone());
        self.push(if created {
            BackendEvent::ConsumerCreated {
                connection_id,
                stream,
                consumer,
            }
        } else {
            BackendEvent::ConsumerUpdated {
                connection_id,
                stream,
                consumer,
            }
        });
    }

    pub(super) fn delete_consumer(&mut self, connection_id: u64, stream: String, name: String) {
        if let Some(consumers) = self.consumers.get_mut(&stream) {
            consumers.remove(&name);
        }
        self.push(BackendEvent::ConsumerDeleted {
            connection_id,
            stream,
            name,
        });
    }

    pub(super) fn fetch_consumer_messages(
        &mut self,
        connection_id: u64,
        stream: String,
        consumer: String,
        batch: usize,
    ) {
        let messages = self
            .stream_messages
            .get(&stream)
            .into_iter()
            .flatten()
            .take(batch)
            .cloned()
            .collect();
        self.push(BackendEvent::ConsumerMessagesFetched {
            connection_id,
            stream,
            consumer,
            messages,
        });
    }
}
