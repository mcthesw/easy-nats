use eframe::egui;
use nats_backend::{BackendCommand, BackendEvent, ConnectionStatusKind};

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
                        ConnectionStatusKind::Connected
                            if !matches!(prev, Some(ConnectionStatusKind::Connected)) =>
                        {
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
                        ConnectionStatusKind::Disconnected
                            if !self.user_wants_connected.contains(&connection_id) =>
                        {
                            self.stream_lists.remove(&connection_id);
                            self.kv_lists.remove(&connection_id);
                            self.obj_store_lists.remove(&connection_id);
                        }
                        ConnectionStatusKind::Error(msg) => {
                            if self.user_wants_connected.contains(&connection_id) {
                                self.toasts.push(
                                    ToastLevel::Info,
                                    format!("{}: {}", self.conn_name(connection_id), msg),
                                );
                            } else {
                                self.toasts.push(
                                    ToastLevel::Error,
                                    format!("{}: {}", self.conn_name(connection_id), msg),
                                );
                            }
                        }
                        _ => {}
                    }
                    self.conn_statuses.insert(connection_id, status);
                }
                BackendEvent::OperationSucceeded {
                    connection_id,
                    operation,
                } => {
                    self.handle_operation_result(connection_id, operation);
                }
                BackendEvent::RequestResponse {
                    connection_id,
                    backend_id: response_backend_id,
                    request_id,
                    subject,
                    payload,
                    headers,
                } => {
                    self.apply_publisher_request_response(
                        connection_id,
                        response_backend_id,
                        request_id,
                        subject,
                        payload,
                        headers,
                    );
                }
                BackendEvent::RequestFailed {
                    connection_id,
                    backend_id: response_backend_id,
                    request_id,
                    message,
                    kind,
                } => {
                    self.apply_publisher_request_failed(
                        connection_id,
                        response_backend_id,
                        request_id,
                        kind,
                        message,
                    );
                }
                BackendEvent::Replied {
                    connection_id,
                    backend_id: reply_backend_id,
                    reply_id,
                    ..
                } => {
                    self.apply_subscriber_reply_success(connection_id, reply_backend_id, reply_id);
                }
                BackendEvent::ReplyFailed {
                    connection_id,
                    backend_id: reply_backend_id,
                    reply_id,
                    message,
                } => {
                    self.apply_subscriber_reply_failed(
                        connection_id,
                        reply_backend_id,
                        reply_id,
                        message,
                    );
                }
                BackendEvent::MessageBatch {
                    connection_id,
                    backend_id: msg_backend_id,
                    messages,
                } => {
                    self.apply_subscriber_message_batch(connection_id, msg_backend_id, messages);
                }
                BackendEvent::MetricsSnapshot {
                    connection_id,
                    snapshot,
                } => {
                    self.apply_metrics_snapshot(connection_id, *snapshot);
                }
                BackendEvent::ClientStatusPageLoaded {
                    connection_id,
                    page,
                } => {
                    self.apply_client_status_page(connection_id, *page);
                }
                BackendEvent::ClientStatusDetailLoaded {
                    connection_id,
                    detail,
                } => {
                    self.apply_client_status_detail(connection_id, *detail);
                }
                BackendEvent::ClientStatusError {
                    connection_id,
                    error,
                } => {
                    self.apply_client_status_error(connection_id, *error);
                }
                BackendEvent::KvBucketsListed {
                    connection_id,
                    buckets,
                } => {
                    self.apply_kv_buckets(connection_id, buckets);
                }
                BackendEvent::KvBucketCreated {
                    connection_id,
                    bucket,
                } => {
                    self.apply_kv_bucket_changed(
                        connection_id,
                        nats_backend::BackendOperation::CreateKvBucket,
                        bucket,
                    );
                }
                BackendEvent::KvBucketUpdated {
                    connection_id,
                    bucket,
                } => {
                    self.apply_kv_bucket_changed(
                        connection_id,
                        nats_backend::BackendOperation::UpdateKvBucket,
                        bucket,
                    );
                }
                BackendEvent::KvBucketDeleted {
                    connection_id,
                    bucket,
                } => {
                    self.apply_kv_bucket_deleted(connection_id, bucket);
                }
                BackendEvent::KvKeysListed {
                    connection_id,
                    batch,
                } => {
                    self.apply_kv_key_batch(connection_id, batch);
                }
                BackendEvent::KvEntryFetched {
                    connection_id,
                    entry,
                } => {
                    self.apply_kv_entry(connection_id, entry);
                }
                BackendEvent::KvHistoryFetched {
                    connection_id,
                    bucket,
                    key,
                    history,
                } => {
                    self.apply_kv_history(connection_id, bucket, key, history);
                }
                BackendEvent::KvEntryMutated {
                    connection_id,
                    operation,
                    bucket,
                    key,
                } => {
                    self.apply_kv_entry_mutation(connection_id, operation, bucket, key);
                }
                BackendEvent::ObjectStoreBucketsListed {
                    connection_id,
                    buckets,
                } => {
                    self.apply_obj_store_buckets(connection_id, buckets);
                }
                BackendEvent::ObjectStoreBucketCreated {
                    connection_id,
                    bucket,
                } => {
                    self.apply_obj_store_bucket_created(connection_id, bucket);
                }
                BackendEvent::ObjectStoreBucketDeleted {
                    connection_id,
                    bucket,
                } => {
                    self.apply_obj_store_bucket_deleted(connection_id, bucket);
                }
                BackendEvent::ObjectStoreObjectsListed {
                    connection_id,
                    bucket,
                    objects,
                } => {
                    self.apply_obj_store_objects(connection_id, bucket, objects);
                }
                BackendEvent::ObjectStoreObjectUploaded {
                    connection_id,
                    object,
                } => {
                    self.apply_obj_store_uploaded(connection_id, object);
                }
                BackendEvent::ObjectStoreObjectDownloaded {
                    connection_id,
                    result,
                } => {
                    self.apply_obj_store_downloaded(connection_id, result);
                }
                BackendEvent::ObjectStoreObjectDeleted {
                    connection_id,
                    bucket,
                    name,
                } => {
                    self.apply_obj_store_deleted(connection_id, bucket, name);
                }
                BackendEvent::ServerInfoLoaded {
                    connection_id,
                    info,
                } => {
                    self.apply_server_info(connection_id, info);
                }
                BackendEvent::JetStreamAccountInfoLoaded {
                    connection_id,
                    info,
                } => {
                    self.apply_jetstream_account_info(connection_id, info);
                }
                BackendEvent::StreamsListed {
                    connection_id,
                    streams,
                } => {
                    self.apply_streams(connection_id, streams);
                }
                BackendEvent::StreamCreated {
                    connection_id,
                    stream,
                } => {
                    self.apply_stream_changed(
                        connection_id,
                        nats_backend::BackendOperation::CreateStream,
                        stream,
                    );
                }
                BackendEvent::StreamUpdated {
                    connection_id,
                    stream,
                } => {
                    self.apply_stream_changed(
                        connection_id,
                        nats_backend::BackendOperation::UpdateStream,
                        stream,
                    );
                }
                BackendEvent::StreamDeleted {
                    connection_id,
                    name,
                } => {
                    self.apply_stream_deleted(connection_id, name);
                }
                BackendEvent::StreamPurged {
                    connection_id,
                    name,
                    purged,
                } => {
                    self.apply_stream_purged(connection_id, name, purged);
                }
                BackendEvent::StreamMessagesFetched {
                    connection_id,
                    stream,
                    messages,
                } => {
                    self.apply_stream_messages(connection_id, stream, messages);
                }
                BackendEvent::StreamMessageDeleted {
                    connection_id,
                    stream,
                    sequence,
                } => {
                    self.apply_stream_message_deleted(connection_id, stream, sequence);
                }
                BackendEvent::ConsumersListed {
                    connection_id,
                    stream,
                    consumers,
                } => {
                    self.apply_consumers(connection_id, stream, consumers);
                }
                BackendEvent::ConsumerCreated {
                    connection_id,
                    stream,
                    consumer: _,
                } => {
                    self.apply_consumer_changed(
                        connection_id,
                        nats_backend::BackendOperation::CreateConsumer,
                        stream,
                    );
                }
                BackendEvent::ConsumerUpdated {
                    connection_id,
                    stream,
                    consumer: _,
                } => {
                    self.apply_consumer_changed(
                        connection_id,
                        nats_backend::BackendOperation::UpdateConsumer,
                        stream,
                    );
                }
                BackendEvent::ConsumerDeleted {
                    connection_id,
                    stream,
                    name: _,
                } => {
                    self.apply_consumer_changed(
                        connection_id,
                        nats_backend::BackendOperation::DeleteConsumer,
                        stream,
                    );
                }
                BackendEvent::ConsumerMessagesFetched {
                    connection_id,
                    stream,
                    consumer,
                    messages,
                } => {
                    self.apply_consumer_messages(connection_id, stream, consumer, messages);
                }
                BackendEvent::Error {
                    connection_id,
                    backend_id,
                    operation,
                    message,
                    context,
                } => {
                    self.handle_error(
                        connection_id,
                        backend_id,
                        operation,
                        &message,
                        context.as_ref(),
                    );
                }
            }
        }

        ctx.request_repaint();
    }
}
