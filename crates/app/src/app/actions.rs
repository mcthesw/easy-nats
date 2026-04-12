use nats_backend::{AuthMethod, BackendCommand, ConnectionConfig};

use crate::tabs::TabKind;

use super::{model::EasyNatsApp, util::same_tab};

impl EasyNatsApp {
    pub(crate) fn connect(&mut self, id: u64) {
        if let Some(cfg) = self.config.connections.iter().find(|c| c.id == id) {
            self.backend.send(BackendCommand::Connect {
                config: cfg.clone(),
            });
        }
    }

    pub(crate) fn disconnect(&mut self, id: u64) {
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
            nkey_seed,
            creds_path,
            cert_path,
            key_path,
            tls_enabled: cfg.tls_enabled,
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
            }
        } else {
            let id = self.config.next_connection_id();
            self.config.connections.push(ConnectionConfig {
                id,
                name: self.editor.name.clone(),
                urls: vec![self.editor.url.clone()],
                auth,
                tls_enabled: self.editor.tls_enabled,
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
        if self
            .dock_state
            .find_tab_from(|existing| same_tab(existing, &tab))
            .is_some()
        {
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
            ..
        } = &mut tab
        {
            state.loading_entries = true;
            self.backend.send(BackendCommand::ListKvKeys {
                connection_id: *connection_id,
                bucket: bucket_name.clone(),
            });
        }

        self.dock_state.push_to_focused_leaf(tab);
    }

    pub(crate) fn save_stream_editor(&mut self) {
        let subjects: Vec<String> = self
            .stream_editor
            .subjects
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let storage = match self.stream_editor.storage {
            super::editors::StorageSelection::File => "file",
            super::editors::StorageSelection::Memory => "memory",
        };
        let retention = match self.stream_editor.retention {
            super::editors::RetentionSelection::Limits => "limits",
            super::editors::RetentionSelection::Interest => "interest",
            super::editors::RetentionSelection::WorkQueue => "workqueue",
        };

        let mut config = serde_json::json!({
            "name": self.stream_editor.name.trim(),
            "subjects": subjects,
            "storage": storage,
            "retention": retention,
        });

        if let Ok(v) = self.stream_editor.max_messages.parse::<i64>() {
            config["max_msgs"] = serde_json::json!(v);
        }
        if let Ok(v) = self.stream_editor.max_bytes.parse::<i64>() {
            config["max_bytes"] = serde_json::json!(v);
        }
        if let Ok(secs) = self.stream_editor.max_age_secs.parse::<u64>() {
            config["max_age"] = serde_json::json!(secs * 1_000_000_000_u64);
        }
        if let Ok(v) = self.stream_editor.num_replicas.parse::<usize>() {
            config["num_replicas"] = serde_json::json!(v);
        }
        if !self.stream_editor.description.trim().is_empty() {
            config["description"] = serde_json::json!(self.stream_editor.description.trim());
        }

        self.backend.send(BackendCommand::CreateStream {
            connection_id: self.stream_editor.connection_id,
            config,
        });
        self.stream_editor.visible = false;
    }

    pub(crate) fn save_consumer_editor(&mut self) {
        let mut config = serde_json::json!({
            "name": self.consumer_editor.name.trim(),
            "deliver_policy": self.consumer_editor.deliver_policy.as_wire(),
            "ack_policy": self.consumer_editor.ack_policy.as_wire(),
        });

        if self.consumer_editor.durable {
            config["durable_name"] = serde_json::json!(self.consumer_editor.name.trim());
        }
        if !self.consumer_editor.filter_subject.trim().is_empty() {
            config["filter_subject"] =
                serde_json::json!(self.consumer_editor.filter_subject.trim());
        }
        if let Ok(v) = self.consumer_editor.max_deliver.parse::<i64>() {
            config["max_deliver"] = serde_json::json!(v);
        }
        if let Ok(v) = self.consumer_editor.max_ack_pending.parse::<i64>() {
            config["max_ack_pending"] = serde_json::json!(v);
        }
        if !self.consumer_editor.description.trim().is_empty() {
            config["description"] = serde_json::json!(self.consumer_editor.description.trim());
        }

        self.backend.send(BackendCommand::CreateConsumer {
            connection_id: self.consumer_editor.connection_id,
            stream: self.consumer_editor.stream_name.clone(),
            config,
        });
        self.consumer_editor.visible = false;
    }

    pub(crate) fn save_kv_bucket_editor(&mut self) {
        let storage = match self.kv_bucket_editor.storage {
            super::editors::StorageSelection::File => "file",
            super::editors::StorageSelection::Memory => "memory",
        };

        let mut config = serde_json::json!({
            "bucket": self.kv_bucket_editor.bucket.trim(),
            "history": self.kv_bucket_editor.history.parse::<i64>().unwrap_or(1),
            "storage": storage,
        });

        if let Ok(v) = self.kv_bucket_editor.max_value_size.parse::<i32>() {
            config["max_value_size"] = serde_json::json!(v);
        }
        if let Ok(v) = self.kv_bucket_editor.max_bytes.parse::<i64>() {
            config["max_bytes"] = serde_json::json!(v);
        }
        if let Ok(secs) = self.kv_bucket_editor.max_age_secs.parse::<u64>() {
            config["max_age"] = serde_json::json!(secs * 1_000_000_000_u64);
        }
        if let Ok(v) = self.kv_bucket_editor.num_replicas.parse::<usize>() {
            config["num_replicas"] = serde_json::json!(v);
        }
        if !self.kv_bucket_editor.description.trim().is_empty() {
            config["description"] = serde_json::json!(self.kv_bucket_editor.description.trim());
        }

        self.backend.send(BackendCommand::CreateKvBucket {
            connection_id: self.kv_bucket_editor.connection_id,
            config,
        });
        self.kv_bucket_editor.visible = false;
    }

    pub(crate) fn close_other_tabs(&mut self, keep_title: &str) {
        let to_remove: Vec<_> = self
            .dock_state
            .iter_all_tabs()
            .filter(|(_, tab)| tab.title() != keep_title && !matches!(tab, TabKind::Welcome))
            .map(|(surface, tab)| (surface, tab.title()))
            .collect();
        for (_, title) in to_remove {
            if let Some(pos) = self.dock_state.find_tab_from(|t| t.title() == title) {
                self.dock_state.remove_tab(pos);
            }
        }
    }

    pub(crate) fn close_all_tabs(&mut self) {
        let to_remove: Vec<String> = self
            .dock_state
            .iter_all_tabs()
            .filter(|(_, tab)| !matches!(tab, TabKind::Welcome))
            .map(|(_, tab)| tab.title())
            .collect();
        for title in to_remove {
            if let Some(pos) = self.dock_state.find_tab_from(|t| t.title() == title) {
                self.dock_state.remove_tab(pos);
            }
        }
    }

    pub(crate) fn close_tabs_to_right(&mut self, of_title: &str) {
        let all_titles: Vec<String> = self
            .dock_state
            .iter_all_tabs()
            .map(|(_, tab)| tab.title())
            .collect();
        let mut found = false;
        for title in all_titles {
            if title == of_title {
                found = true;
                continue;
            }
            if found {
                if let Some(pos) = self
                    .dock_state
                    .find_tab_from(|t| t.title() == title && !matches!(t, TabKind::Welcome))
                {
                    self.dock_state.remove_tab(pos);
                }
            }
        }
    }
}
