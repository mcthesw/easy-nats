use std::time::Duration;

use eframe::egui;
use nats_backend::{
    AuthMethod, BackendCommand, ConnectionConfig, ConsumerAckPolicyKind, ConsumerConfigInput,
    ConsumerDeliverPolicyKind, KvBucketConfigInput, MonitoringConfig, StorageKind,
    StreamConfigInput, StreamRetentionKind,
};
use tokio_util::sync::CancellationToken;

use crate::schema::kv_subject;
use crate::settings::PubSubTabMode;
use crate::tabs::{
    ClientStatusState, MetricsState, PublisherState, SubscriberState, TabGuard, TabKind,
    next_backend_id,
};

use super::{model::EasyNatsApp, util::same_tab};

/// (connection_id, backend_id, subject) for each active subscription needing cleanup.
type UnsubInfo = Vec<(u64, u64, String)>;

impl EasyNatsApp {
    pub(crate) fn connect(&mut self, id: u64) {
        if let Some(cfg) = self.config.connections.iter().find(|c| c.id == id) {
            self.user_wants_connected.insert(id);
            self.backend.send(BackendCommand::Connect {
                config: cfg.clone(),
            });
        }
    }

    pub(crate) fn disconnect(&mut self, id: u64) {
        self.user_wants_connected.remove(&id);
        self.backend.send(BackendCommand::Disconnect { id });
    }

    pub(crate) fn open_new_editor(&mut self) {
        self.editor = super::editors::ConnectionEditor {
            visible: true,
            editing_id: None,
            url: "nats://localhost:4222".to_string(),
            ..Default::default()
        };
    }

    pub(crate) fn open_edit_editor(&mut self, cfg: &ConnectionConfig) {
        let (auth_kind, token, username, password, nkey_seed, creds_path, cert_path, key_path) =
            match &cfg.auth {
                AuthMethod::None => (
                    super::editors::AuthKindSelection::None,
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                AuthMethod::Token { token } => (
                    super::editors::AuthKindSelection::Token,
                    token.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                AuthMethod::UserPassword { username, password } => (
                    super::editors::AuthKindSelection::UserPassword,
                    String::new(),
                    username.clone(),
                    password.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                AuthMethod::NKey { seed } => (
                    super::editors::AuthKindSelection::NKey,
                    String::new(),
                    String::new(),
                    String::new(),
                    seed.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
                AuthMethod::CredentialsFile { path } => (
                    super::editors::AuthKindSelection::CredentialsFile,
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    path.clone(),
                    String::new(),
                    String::new(),
                ),
                AuthMethod::TlsClientCert {
                    cert_path,
                    key_path,
                } => (
                    super::editors::AuthKindSelection::TlsClientCert,
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    cert_path.clone(),
                    key_path.clone(),
                ),
            };

        self.editor = super::editors::ConnectionEditor {
            visible: true,
            editing_id: Some(cfg.id),
            name: cfg.name.clone(),
            url: cfg.urls.first().cloned().unwrap_or_default(),
            auth_kind,
            token,
            username,
            password,
            metrics_endpoint: cfg.monitoring_endpoint().unwrap_or_default().to_string(),
            nkey_seed,
            creds_path,
            cert_path,
            key_path,
            tls_enabled: cfg.tls_enabled,
            tls_first: cfg.tls_first,
            delete_confirm: None,
        };
    }

    pub(crate) fn save_editor(&mut self) {
        let auth = match self.editor.auth_kind {
            super::editors::AuthKindSelection::None => AuthMethod::None,
            super::editors::AuthKindSelection::Token => AuthMethod::Token {
                token: self.editor.token.clone(),
            },
            super::editors::AuthKindSelection::UserPassword => AuthMethod::UserPassword {
                username: self.editor.username.clone(),
                password: self.editor.password.clone(),
            },
            super::editors::AuthKindSelection::NKey => AuthMethod::NKey {
                seed: self.editor.nkey_seed.clone(),
            },
            super::editors::AuthKindSelection::CredentialsFile => AuthMethod::CredentialsFile {
                path: self.editor.creds_path.clone(),
            },
            super::editors::AuthKindSelection::TlsClientCert => AuthMethod::TlsClientCert {
                cert_path: self.editor.cert_path.clone(),
                key_path: self.editor.key_path.clone(),
            },
        };

        if let Some(id) = self.editor.editing_id {
            if let Some(c) = self.config.connections.iter_mut().find(|c| c.id == id) {
                c.name = self.editor.name.clone();
                c.urls = vec![self.editor.url.clone()];
                c.auth = auth;
                c.tls_enabled = self.editor.tls_enabled;
                c.tls_first = self.editor.tls_first;
                c.monitoring = monitoring_from_editor(&self.editor);
            }
        } else {
            let id = self.config.next_connection_id();
            self.config.connections.push(ConnectionConfig {
                id,
                name: self.editor.name.clone(),
                urls: vec![self.editor.url.clone()],
                auth,
                tls_enabled: self.editor.tls_enabled,
                tls_first: self.editor.tls_first,
                monitoring: monitoring_from_editor(&self.editor),
            });
        }
        self.config.save();
        self.editor.visible = false;
    }

    pub(crate) fn delete_connection(&mut self, id: u64) {
        self.disconnect(id);
        self.conn_statuses.remove(&id);
        self.config.connections.retain(|c| c.id != id);
        self.config.save();
        if self.selected_conn == Some(id) {
            self.selected_conn = None;
        }
    }

    pub(crate) fn open_tab(&mut self, mut tab: TabKind) {
        if let Some(path) = self
            .dock_state
            .find_tab_from(|existing| same_tab(existing, &tab))
        {
            let _ = self.dock_state.set_active_tab(path);
            return;
        }

        if let TabKind::Stream {
            connection_id,
            stream_name,
            state,
            ..
        } = &mut tab
        {
            state.consumers_fetching = true;
            self.backend.send(BackendCommand::ListConsumers {
                connection_id: *connection_id,
                stream: stream_name.clone(),
            });
        }

        if let TabKind::KvBucket {
            connection_id,
            bucket_name,
            state,
            guard,
            ..
        } = &mut tab
        {
            let new_gen = crate::tabs::next_generation();
            state.loading_entries = true;
            state.load_generation = new_gen;
            state.keys.clear();
            state.fetched_values.clear();
            state.fetched_value_bytes.clear();
            state.invalidate_filtered_key_cache();
            state.value_search_cursor = 0;
            state.value_search_scanning = 0;
            state.value_search_pending.clear();
            state.search_generation = state.search_generation.wrapping_add(1);
            state.keys_complete = false;
            self.backend.send(BackendCommand::ListKvKeys {
                connection_id: *connection_id,
                bucket: bucket_name.clone(),
                cancel: guard.cancellation(),
                generation: new_gen,
            });
        }

        if let TabKind::ObjectStoreBucket {
            connection_id,
            bucket_name,
            state,
            ..
        } = &mut tab
        {
            state.loading_objects = true;
            self.backend.send(BackendCommand::ListObjects {
                connection_id: *connection_id,
                bucket: bucket_name.clone(),
            });
        }

        self.dock_state.push_to_focused_leaf(tab);
    }

    /// Open a singleton tab (Settings, LogViewer, etc.) or focus it if already open.
    pub(crate) fn open_or_focus_tab_kind(&mut self, tab: TabKind) {
        if let Some(path) = self
            .dock_state
            .find_tab_from(|existing| same_tab(existing, &tab))
        {
            let _ = self.dock_state.set_active_tab(path);
        } else {
            self.dock_state.push_to_focused_leaf(tab);
        }
    }

    pub(crate) fn open_or_focus_publisher_tab(&mut self, connection_id: u64) {
        self.open_or_focus_pubsub_tab(connection_id, PubSubTabKind::Publisher);
    }

    pub(crate) fn open_or_focus_subscriber_tab(&mut self, connection_id: u64) {
        self.open_or_focus_pubsub_tab(connection_id, PubSubTabKind::Subscriber);
    }

    pub(crate) fn open_or_focus_metrics_tab(&mut self, connection_id: u64) {
        let connection_name = self.conn_name(connection_id);
        let endpoint = self
            .connection_metrics_endpoint(connection_id)
            .unwrap_or_default();

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Metrics {
                connection_id: existing_id,
                connection_name: existing_name,
                state,
            } = tab
                && *existing_id == connection_id
            {
                *existing_name = connection_name.clone();
                state.set_endpoint(endpoint.clone());
            }
        }

        let tab = TabKind::Metrics {
            connection_id,
            connection_name,
            state: MetricsState::with_endpoint(endpoint),
        };
        self.open_or_focus_tab_kind(tab);
    }

    pub(crate) fn open_or_focus_clients_tab(&mut self, connection_id: u64) {
        let connection_name = self.conn_name(connection_id);
        let endpoint = self
            .connection_metrics_endpoint(connection_id)
            .unwrap_or_default();

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Clients {
                connection_id: existing_id,
                connection_name: existing_name,
                state,
            } = tab
                && *existing_id == connection_id
            {
                *existing_name = connection_name.clone();
                state.set_endpoint(endpoint.clone());
            }
        }

        let tab = TabKind::Clients {
            connection_id,
            connection_name,
            state: ClientStatusState::with_endpoint(endpoint),
        };
        self.open_or_focus_tab_kind(tab);
    }

    pub(crate) fn save_stream_editor(&mut self) {
        let subjects: Vec<String> = self
            .stream_editor
            .subjects
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        self.backend.send(BackendCommand::CreateStream {
            connection_id: self.stream_editor.connection_id,
            config: StreamConfigInput {
                name: self.stream_editor.name.trim().to_string(),
                subjects,
                storage: storage_kind(self.stream_editor.storage),
                retention: retention_kind(self.stream_editor.retention),
                max_messages: parse_optional(&self.stream_editor.max_messages),
                max_bytes: parse_optional(&self.stream_editor.max_bytes),
                max_age: parse_seconds(&self.stream_editor.max_age_secs),
                num_replicas: parse_optional(&self.stream_editor.num_replicas),
                description: trimmed_optional(&self.stream_editor.description),
            },
        });
        self.stream_editor.visible = false;
    }

    pub(crate) fn publish_stream_editor(&mut self) {
        let subject = self.stream_publish_editor.subject.trim().to_string();
        let outgoing = self.schema_manager.prepare_outgoing(
            self.stream_publish_editor.connection_id,
            &subject,
            &self.stream_publish_editor.payload,
        );
        if !outgoing.can_send {
            if let Some(status) = outgoing.status {
                self.toasts
                    .push(crate::toast::ToastLevel::Error, status.message);
            }
            return;
        }
        self.backend.send(BackendCommand::Publish {
            connection_id: self.stream_publish_editor.connection_id,
            subject,
            payload: outgoing.payload,
            headers: collect_non_empty_headers(&self.stream_publish_editor.headers),
        });
    }

    pub(crate) fn save_consumer_editor(&mut self) {
        let name = self.consumer_editor.name.trim().to_string();
        self.backend.send(BackendCommand::CreateConsumer {
            connection_id: self.consumer_editor.connection_id,
            stream: self.consumer_editor.stream_name.clone(),
            config: ConsumerConfigInput {
                name: name.clone(),
                durable_name: self.consumer_editor.durable.then_some(name),
                filter_subject: trimmed_optional(&self.consumer_editor.filter_subject),
                deliver_policy: deliver_policy_kind(&self.consumer_editor),
                ack_policy: ack_policy_kind(self.consumer_editor.ack_policy),
                max_deliver: parse_optional(&self.consumer_editor.max_deliver),
                max_ack_pending: parse_optional(&self.consumer_editor.max_ack_pending),
                description: trimmed_optional(&self.consumer_editor.description),
            },
        });
        self.consumer_editor.visible = false;
    }

    pub(crate) fn save_kv_bucket_editor(&mut self) {
        self.backend.send(BackendCommand::CreateKvBucket {
            connection_id: self.kv_bucket_editor.connection_id,
            config: KvBucketConfigInput {
                bucket: self.kv_bucket_editor.bucket.trim().to_string(),
                history: self.kv_bucket_editor.history.parse::<i64>().unwrap_or(1),
                storage: storage_kind(self.kv_bucket_editor.storage),
                max_value_size: parse_optional(&self.kv_bucket_editor.max_value_size),
                max_bytes: parse_optional(&self.kv_bucket_editor.max_bytes),
                max_age: parse_seconds(&self.kv_bucket_editor.max_age_secs),
                num_replicas: parse_optional(&self.kv_bucket_editor.num_replicas),
                description: trimmed_optional(&self.kv_bucket_editor.description),
            },
        });
        self.kv_bucket_editor.visible = false;
    }

    pub(crate) fn save_consumer_edit_editor(&mut self) {
        let original = &self.consumer_edit_editor.original_config;
        self.backend.send(BackendCommand::UpdateConsumer {
            connection_id: self.consumer_edit_editor.connection_id,
            stream: self.consumer_edit_editor.stream_name.clone(),
            config: ConsumerConfigInput {
                name: original.name.clone(),
                durable_name: original.durable_name.clone(),
                filter_subject: original.filter_subject.clone(),
                deliver_policy: original.deliver_policy.clone(),
                ack_policy: original.ack_policy,
                max_deliver: parse_optional(&self.consumer_edit_editor.max_deliver),
                max_ack_pending: parse_optional(&self.consumer_edit_editor.max_ack_pending),
                description: trimmed_optional(&self.consumer_edit_editor.description),
            },
        });
        self.consumer_edit_editor.visible = false;
    }

    pub(crate) fn save_kv_bucket_edit_editor(&mut self) {
        let ed = &self.kv_bucket_edit_editor;
        let connection_id = ed.connection_id;
        self.backend.send(BackendCommand::UpdateKvBucket {
            connection_id,
            config: KvBucketConfigInput {
                bucket: ed.bucket.clone(),
                history: ed.history.parse::<i64>().unwrap_or(1),
                storage: ed.storage,
                max_value_size: parse_optional(&ed.max_value_size),
                max_bytes: parse_optional(&ed.max_bytes),
                max_age: parse_seconds(&ed.max_age_secs),
                num_replicas: parse_optional(&ed.num_replicas),
                description: trimmed_optional(&ed.description),
            },
        });
        self.kv_bucket_edit_editor.visible = false;
    }

    pub(crate) fn save_kv_entry_create_editor(&mut self) {
        let subject = kv_subject(
            &self.kv_entry_create_editor.bucket_name,
            self.kv_entry_create_editor.key.trim(),
        );
        let outgoing = self.schema_manager.prepare_outgoing(
            self.kv_entry_create_editor.connection_id,
            &subject,
            &self.kv_entry_create_editor.value,
        );
        if !outgoing.can_send {
            if let Some(status) = outgoing.status {
                self.toasts
                    .push(crate::toast::ToastLevel::Error, status.message);
            }
            return;
        }
        self.backend.send(BackendCommand::PutKvEntry {
            connection_id: self.kv_entry_create_editor.connection_id,
            bucket: self.kv_entry_create_editor.bucket_name.clone(),
            key: self.kv_entry_create_editor.key.trim().to_string(),
            value: outgoing.payload,
        });
        self.kv_entry_create_editor.visible = false;
    }

    /// Remove all tabs matching a predicate (except Welcome).
    /// Sends Unsubscribe for any active subscriber tabs being removed.
    pub(crate) fn remove_tabs_matching(&mut self, mut predicate: impl FnMut(&TabKind) -> bool) {
        // Collect tab IDs + subscriber cleanup info
        let to_remove: Vec<(egui::Id, UnsubInfo)> = self
            .dock_state
            .iter_all_tabs()
            .filter(|(_, tab)| !matches!(tab, TabKind::Welcome) && predicate(tab))
            .map(|(_, tab)| {
                let unsubs = if let TabKind::Subscriber {
                    connection_id,
                    backend_id,
                    state,
                    ..
                } = tab
                {
                    state
                        .subscriptions
                        .iter()
                        .filter(|s| s.active)
                        .map(|s| (*connection_id, *backend_id, s.subject.clone()))
                        .collect()
                } else {
                    Vec::new()
                };
                (tab.tab_id(), unsubs)
            })
            .collect();
        for (tid, unsubs) in to_remove {
            for (connection_id, backend_id, subject) in unsubs {
                self.backend.send(BackendCommand::Unsubscribe {
                    connection_id,
                    backend_id,
                    subject,
                });
            }
            if let Some(pos) = self.dock_state.find_tab_from(|t| t.tab_id() == tid) {
                self.dock_state.remove_tab(pos);
            }
        }
    }

    pub(crate) fn close_other_tabs(&mut self, keep_tab_id: egui::Id) {
        self.remove_tabs_matching(|tab| tab.tab_id() != keep_tab_id);
    }

    pub(crate) fn close_all_tabs(&mut self) {
        self.remove_tabs_matching(|_| true);
    }

    pub(crate) fn close_tabs_to_right(&mut self, of_tab_id: egui::Id) {
        let all_info: Vec<(egui::Id, UnsubInfo)> = self
            .dock_state
            .iter_all_tabs()
            .map(|(_, tab)| {
                let unsubs = if let TabKind::Subscriber {
                    connection_id,
                    backend_id,
                    state,
                    ..
                } = tab
                {
                    state
                        .subscriptions
                        .iter()
                        .filter(|s| s.active)
                        .map(|s| (*connection_id, *backend_id, s.subject.clone()))
                        .collect()
                } else {
                    Vec::new()
                };
                (tab.tab_id(), unsubs)
            })
            .collect();
        let mut found = false;
        for (tid, unsubs) in all_info {
            if tid == of_tab_id {
                found = true;
                continue;
            }
            if found
                && let Some(pos) = self
                    .dock_state
                    .find_tab_from(|t| t.tab_id() == tid && !matches!(t, TabKind::Welcome))
            {
                for (connection_id, backend_id, subject) in unsubs {
                    self.backend.send(BackendCommand::Unsubscribe {
                        connection_id,
                        backend_id,
                        subject,
                    });
                }
                self.dock_state.remove_tab(pos);
            }
        }
    }
}

fn monitoring_from_editor(editor: &super::editors::ConnectionEditor) -> Option<MonitoringConfig> {
    let endpoint = editor.metrics_endpoint.trim();
    if endpoint.is_empty() {
        None
    } else {
        Some(MonitoringConfig {
            endpoint: endpoint.to_string(),
        })
    }
}

#[derive(Clone, Copy)]
enum PubSubTabKind {
    Publisher,
    Subscriber,
}

impl EasyNatsApp {
    fn open_or_focus_pubsub_tab(&mut self, connection_id: u64, tab_kind: PubSubTabKind) {
        if self.settings.pubsub_tab_mode == PubSubTabMode::ReuseExisting
            && let Some(path) = self
                .dock_state
                .find_tab_from(|tab| matches_pubsub_tab(tab, connection_id, tab_kind))
        {
            let _ = self.dock_state.set_active_tab(path);
            return;
        }

        let tab = self.new_pubsub_tab(connection_id, tab_kind);
        self.open_tab(tab);
    }

    fn new_pubsub_tab(&mut self, connection_id: u64, tab_kind: PubSubTabKind) -> TabKind {
        let connection_name = self.conn_name(connection_id);
        let (display_id, id_return) = self.tab_id_alloc.allocate();
        let guard = TabGuard::new(CancellationToken::new(), display_id, id_return);
        let backend_id = next_backend_id();

        match tab_kind {
            PubSubTabKind::Publisher => TabKind::Publisher {
                connection_id,
                connection_name,
                guard,
                backend_id,
                state: PublisherState::default(),
            },
            PubSubTabKind::Subscriber => TabKind::Subscriber {
                connection_id,
                connection_name,
                guard,
                backend_id,
                state: SubscriberState::default(),
            },
        }
    }
}

fn collect_non_empty_headers(headers: &[(String, String)]) -> Option<Vec<(String, String)>> {
    let non_empty: Vec<(String, String)> = headers
        .iter()
        .filter(|(k, v)| !k.trim().is_empty() || !v.trim().is_empty())
        .cloned()
        .collect();
    if non_empty.is_empty() {
        None
    } else {
        Some(non_empty)
    }
}

fn parse_optional<T: std::str::FromStr>(value: &str) -> Option<T> {
    value.trim().parse::<T>().ok()
}

fn parse_seconds(value: &str) -> Option<Duration> {
    parse_optional::<u64>(value).map(Duration::from_secs)
}

fn trimmed_optional(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn storage_kind(selection: super::editors::StorageSelection) -> StorageKind {
    match selection {
        super::editors::StorageSelection::File => StorageKind::File,
        super::editors::StorageSelection::Memory => StorageKind::Memory,
    }
}

fn retention_kind(selection: super::editors::RetentionSelection) -> StreamRetentionKind {
    match selection {
        super::editors::RetentionSelection::Limits => StreamRetentionKind::Limits,
        super::editors::RetentionSelection::Interest => StreamRetentionKind::Interest,
        super::editors::RetentionSelection::WorkQueue => StreamRetentionKind::WorkQueue,
    }
}

fn deliver_policy_kind(editor: &super::editors::ConsumerCreateEditor) -> ConsumerDeliverPolicyKind {
    match editor.deliver_policy {
        super::editors::DeliverPolicySelection::All => ConsumerDeliverPolicyKind::All,
        super::editors::DeliverPolicySelection::Last => ConsumerDeliverPolicyKind::Last,
        super::editors::DeliverPolicySelection::New => ConsumerDeliverPolicyKind::New,
        super::editors::DeliverPolicySelection::ByStartSequence => {
            ConsumerDeliverPolicyKind::ByStartSequence {
                start_sequence: editor.deliver_start_sequence.parse::<u64>().unwrap_or(1),
            }
        }
        super::editors::DeliverPolicySelection::ByStartTime => {
            ConsumerDeliverPolicyKind::ByStartTime {
                start_time: editor.deliver_start_time.trim().to_string(),
            }
        }
        super::editors::DeliverPolicySelection::LastPerSubject => {
            ConsumerDeliverPolicyKind::LastPerSubject
        }
    }
}

fn ack_policy_kind(selection: super::editors::AckPolicySelection) -> ConsumerAckPolicyKind {
    match selection {
        super::editors::AckPolicySelection::Explicit => ConsumerAckPolicyKind::Explicit,
        super::editors::AckPolicySelection::All => ConsumerAckPolicyKind::All,
        super::editors::AckPolicySelection::None => ConsumerAckPolicyKind::None,
    }
}

fn matches_pubsub_tab(tab: &TabKind, connection_id: u64, tab_kind: PubSubTabKind) -> bool {
    match (tab_kind, tab) {
        (
            PubSubTabKind::Publisher,
            TabKind::Publisher {
                connection_id: existing_id,
                ..
            },
        ) => *existing_id == connection_id,
        (
            PubSubTabKind::Subscriber,
            TabKind::Subscriber {
                connection_id: existing_id,
                ..
            },
        ) => *existing_id == connection_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
