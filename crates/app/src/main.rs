mod format;
mod tabs;
mod toast;
mod ui_strings;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let native_options = eframe::NativeOptions::default();
    if let Err(e) = eframe::run_native(
        "Easy NATS",
        native_options,
        Box::new(|cc| {
            // Detect system dark mode preference, default to dark
            let dark = cc
                .egui_ctx
                .system_theme()
                .map(|t| t == eframe::egui::Theme::Dark)
                .unwrap_or(true);
            if dark {
                cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            }
            Ok(Box::new(app::EasyNatsApp::new(dark)))
        }),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}

mod app {
    use eframe::egui;
    use egui_dock::{DockArea, DockState};
    use nats_backend::{
        AppConfig, AuthMethod, BackendCommand, BackendEvent, BackendHandle, ConnectionConfig,
        ConnectionStatusKind,
    };
    use std::collections::HashMap;

    use crate::tabs::{
        AppTabViewer, PublisherState, ReceivedMessage, ResponseData, SubscriberState, TabKind,
    };
    use crate::toast::{ToastLevel, Toasts};
    use crate::ui_strings as S;

    pub struct EasyNatsApp {
        backend: BackendHandle,
        config: AppConfig,
        conn_statuses: HashMap<u64, ConnectionStatusKind>,
        selected_conn: Option<u64>,
        editor: ConnectionEditor,
        dock_state: DockState<TabKind>,
        toasts: Toasts,
        dark_mode: bool,
    }

    #[derive(Default)]
    struct ConnectionEditor {
        visible: bool,
        editing_id: Option<u64>,
        name: String,
        url: String,
        auth_kind: AuthKindSelection,
        token: String,
        username: String,
        password: String,
        nkey_seed: String,
        creds_path: String,
        cert_path: String,
        key_path: String,
        tls_enabled: bool,
        delete_confirm: Option<u64>,
    }

    #[derive(Default, Debug, Clone, Copy, PartialEq)]
    enum AuthKindSelection {
        #[default]
        None,
        Token,
        UserPassword,
        NKey,
        CredentialsFile,
        TlsClientCert,
    }

    impl AuthKindSelection {
        const ALL: [Self; 6] = [
            Self::None,
            Self::Token,
            Self::UserPassword,
            Self::NKey,
            Self::CredentialsFile,
            Self::TlsClientCert,
        ];

        fn label(self) -> &'static str {
            match self {
                Self::None => S::AUTH_NONE,
                Self::Token => S::AUTH_TOKEN,
                Self::UserPassword => S::AUTH_USER_PASSWORD,
                Self::NKey => S::AUTH_NKEY,
                Self::CredentialsFile => S::AUTH_CREDENTIALS_FILE,
                Self::TlsClientCert => S::AUTH_TLS_CLIENT_CERT,
            }
        }
    }

    impl EasyNatsApp {
        pub fn new(dark_mode: bool) -> Self {
            let config = AppConfig::load();
            let dock_state = DockState::new(vec![TabKind::Welcome]);
            Self {
                backend: BackendHandle::spawn(),
                config,
                conn_statuses: HashMap::new(),
                selected_conn: None,
                editor: ConnectionEditor::default(),
                dock_state,
                toasts: Toasts::default(),
                dark_mode,
            }
        }

        fn handle_events(&mut self, ctx: &egui::Context) {
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
                        match &status {
                            ConnectionStatusKind::Connected => {
                                self.toasts.push(
                                    ToastLevel::Success,
                                    format!("Connected to {}", self.conn_name(connection_id)),
                                );
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
                        ..
                    } => {
                        if operation == "publish" {
                            self.toasts.push(
                                ToastLevel::Success,
                                format!("Published to {}", self.conn_name(connection_id)),
                            );
                        } else {
                            self.toasts
                                .push(ToastLevel::Success, format!("{operation} succeeded"));
                        }
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
                                && state.subscribed
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
                        // Clear waiting state on publisher tabs if request failed
                        if operation == "request"
                            && let Some(cid) = connection_id
                        {
                            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                                if let TabKind::Publisher {
                                    connection_id: tab_cid,
                                    state,
                                    ..
                                } = tab
                                    && *tab_cid == cid
                                {
                                    state.waiting = false;
                                }
                            }
                        }
                        self.toasts
                            .push(ToastLevel::Error, format!("{operation}: {message}"));
                    }
                }
            }
            ctx.request_repaint();
        }

        fn conn_name(&self, id: u64) -> String {
            self.config
                .connections
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("#{id}"))
        }

        fn connect(&mut self, id: u64) {
            if let Some(cfg) = self.config.connections.iter().find(|c| c.id == id) {
                self.backend.send(BackendCommand::Connect {
                    config: cfg.clone(),
                });
            }
        }

        fn disconnect(&mut self, id: u64) {
            self.backend.send(BackendCommand::Disconnect { id });
        }

        fn open_new_editor(&mut self) {
            self.editor = ConnectionEditor {
                visible: true,
                editing_id: None,
                url: "nats://localhost:4222".to_string(),
                ..Default::default()
            };
        }

        fn open_edit_editor(&mut self, cfg: &ConnectionConfig) {
            let (auth_kind, token, username, password, nkey_seed, creds_path, cert_path, key_path) =
                match &cfg.auth {
                    AuthMethod::None => (
                        AuthKindSelection::None,
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                    AuthMethod::Token { token } => (
                        AuthKindSelection::Token,
                        token.clone(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                    AuthMethod::UserPassword { username, password } => (
                        AuthKindSelection::UserPassword,
                        String::new(),
                        username.clone(),
                        password.clone(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                    AuthMethod::NKey { seed } => (
                        AuthKindSelection::NKey,
                        String::new(),
                        String::new(),
                        String::new(),
                        seed.clone(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                    AuthMethod::CredentialsFile { path } => (
                        AuthKindSelection::CredentialsFile,
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
                        AuthKindSelection::TlsClientCert,
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        cert_path.clone(),
                        key_path.clone(),
                    ),
                };
            self.editor = ConnectionEditor {
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

        fn save_editor(&mut self) {
            let auth = match self.editor.auth_kind {
                AuthKindSelection::None => AuthMethod::None,
                AuthKindSelection::Token => AuthMethod::Token {
                    token: self.editor.token.clone(),
                },
                AuthKindSelection::UserPassword => AuthMethod::UserPassword {
                    username: self.editor.username.clone(),
                    password: self.editor.password.clone(),
                },
                AuthKindSelection::NKey => AuthMethod::NKey {
                    seed: self.editor.nkey_seed.clone(),
                },
                AuthKindSelection::CredentialsFile => AuthMethod::CredentialsFile {
                    path: self.editor.creds_path.clone(),
                },
                AuthKindSelection::TlsClientCert => AuthMethod::TlsClientCert {
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

        fn delete_connection(&mut self, id: u64) {
            self.disconnect(id);
            self.conn_statuses.remove(&id);
            self.config.connections.retain(|c| c.id != id);
            self.config.save();
            if self.selected_conn == Some(id) {
                self.selected_conn = None;
            }
        }

        /// Open a tab in the dock, avoiding duplicates.
        fn open_tab(&mut self, tab: TabKind) {
            if self
                .dock_state
                .find_tab_from(|existing| same_tab(existing, &tab))
                .is_some()
            {
                return;
            }
            self.dock_state.push_to_focused_leaf(tab);
        }
    }

    /// Check if two tabs represent the same resource.
    fn same_tab(a: &TabKind, b: &TabKind) -> bool {
        match (a, b) {
            (TabKind::Welcome, TabKind::Welcome) => true,
            (
                TabKind::Publisher {
                    connection_id: a, ..
                },
                TabKind::Publisher {
                    connection_id: b, ..
                },
            ) => a == b,
            (
                TabKind::Subscriber {
                    connection_id: a, ..
                },
                TabKind::Subscriber {
                    connection_id: b, ..
                },
            ) => a == b,
            (
                TabKind::Stream {
                    connection_id: a1,
                    stream_name: s1,
                    ..
                },
                TabKind::Stream {
                    connection_id: a2,
                    stream_name: s2,
                    ..
                },
            ) => a1 == a2 && s1 == s2,
            (
                TabKind::KvBucket {
                    connection_id: a1,
                    bucket_name: b1,
                    ..
                },
                TabKind::KvBucket {
                    connection_id: a2,
                    bucket_name: b2,
                    ..
                },
            ) => a1 == a2 && b1 == b2,
            (
                TabKind::ObjectStoreBucket {
                    connection_id: a1,
                    bucket_name: b1,
                    ..
                },
                TabKind::ObjectStoreBucket {
                    connection_id: a2,
                    bucket_name: b2,
                    ..
                },
            ) => a1 == a2 && b1 == b2,
            _ => false,
        }
    }

    impl eframe::App for EasyNatsApp {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.handle_events(ctx);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            // ─── Floating windows (editor, delete confirm) ───
            let mut save_requested = false;
            if self.editor.visible {
                let title = if self.editor.editing_id.is_some() {
                    S::CONNECTION_EDIT
                } else {
                    S::CONNECTION_NEW
                };
                let mut open = true;
                egui::Window::new(title)
                    .open(&mut open)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Grid::new("conn_editor_grid")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(S::FIELD_NAME);
                                ui.text_edit_singleline(&mut self.editor.name);
                                ui.end_row();

                                ui.label(S::FIELD_URL);
                                ui.text_edit_singleline(&mut self.editor.url);
                                ui.end_row();

                                ui.label(S::FIELD_AUTH);
                                egui::ComboBox::from_id_salt("auth_kind")
                                    .selected_text(self.editor.auth_kind.label())
                                    .show_ui(ui, |ui| {
                                        for kind in AuthKindSelection::ALL {
                                            ui.selectable_value(
                                                &mut self.editor.auth_kind,
                                                kind,
                                                kind.label(),
                                            );
                                        }
                                    });
                                ui.end_row();

                                match self.editor.auth_kind {
                                    AuthKindSelection::None => {}
                                    AuthKindSelection::Token => {
                                        ui.label(S::FIELD_TOKEN);
                                        ui.text_edit_singleline(&mut self.editor.token);
                                        ui.end_row();
                                    }
                                    AuthKindSelection::UserPassword => {
                                        ui.label(S::FIELD_USERNAME);
                                        ui.text_edit_singleline(&mut self.editor.username);
                                        ui.end_row();
                                        ui.label(S::FIELD_PASSWORD);
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.editor.password)
                                                .password(true),
                                        );
                                        ui.end_row();
                                    }
                                    AuthKindSelection::NKey => {
                                        ui.label(S::FIELD_NKEY_SEED);
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.editor.nkey_seed)
                                                .password(true),
                                        );
                                        ui.end_row();
                                    }
                                    AuthKindSelection::CredentialsFile => {
                                        ui.label(S::FIELD_CREDS_FILE);
                                        ui.text_edit_singleline(&mut self.editor.creds_path);
                                        ui.end_row();
                                    }
                                    AuthKindSelection::TlsClientCert => {
                                        ui.label(S::FIELD_CERT_PATH);
                                        ui.text_edit_singleline(&mut self.editor.cert_path);
                                        ui.end_row();
                                        ui.label(S::FIELD_KEY_PATH);
                                        ui.text_edit_singleline(&mut self.editor.key_path);
                                        ui.end_row();
                                    }
                                }

                                ui.label(S::FIELD_TLS);
                                ui.checkbox(&mut self.editor.tls_enabled, S::REQUIRE_TLS);
                                ui.end_row();
                            });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let valid = !self.editor.name.trim().is_empty()
                                && !self.editor.url.trim().is_empty();
                            if ui.add_enabled(valid, egui::Button::new(S::SAVE)).clicked() {
                                save_requested = true;
                            }
                            if ui.button(S::CANCEL).clicked() {
                                self.editor.visible = false;
                            }
                        });
                    });
                if !open {
                    self.editor.visible = false;
                }
            }
            if save_requested {
                self.save_editor();
            }

            // Delete confirmation
            let mut do_delete: Option<u64> = None;
            if let Some(id) = self.editor.delete_confirm {
                let conn_name = self.conn_name(id);
                let mut still_open = true;
                egui::Window::new(S::CONNECTION_DELETE_CONFIRM_TITLE)
                    .open(&mut still_open)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label(format!(
                            "{} \"{}\"?",
                            S::CONNECTION_DELETE_PROMPT,
                            conn_name
                        ));
                        ui.horizontal(|ui| {
                            if ui.button(S::DELETE).clicked() {
                                do_delete = Some(id);
                            }
                            if ui.button(S::CANCEL).clicked() {
                                self.editor.delete_confirm = None;
                            }
                        });
                    });
                if !still_open {
                    self.editor.delete_confirm = None;
                }
            }
            if let Some(id) = do_delete {
                self.delete_connection(id);
                self.editor.delete_confirm = None;
            }

            // ─── Sidebar: connections + resource tree ───
            egui::Panel::left("sidebar_panel")
                .default_size(220.0)
                .show_inside(ui, |ui| {
                    // Theme toggle at top
                    ui.horizontal(|ui| {
                        ui.heading(S::CONNECTIONS_HEADING);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let icon = if self.dark_mode {
                                S::THEME_LIGHT
                            } else {
                                S::THEME_DARK
                            };
                            if ui.small_button(icon).clicked() {
                                self.dark_mode = !self.dark_mode;
                                if self.dark_mode {
                                    ui.ctx().set_visuals(egui::Visuals::dark());
                                } else {
                                    ui.ctx().set_visuals(egui::Visuals::light());
                                }
                            }
                            if ui
                                .small_button("＋")
                                .on_hover_text(S::CONNECTION_NEW)
                                .clicked()
                            {
                                self.open_new_editor();
                            }
                        });
                    });
                    ui.separator();

                    // Connection list with resource tree per connected profile
                    let conn_data: Vec<(u64, String, ConnectionStatusKind)> = self
                        .config
                        .connections
                        .iter()
                        .map(|c| {
                            let status = self
                                .conn_statuses
                                .get(&c.id)
                                .cloned()
                                .unwrap_or(ConnectionStatusKind::Disconnected);
                            (c.id, c.name.clone(), status)
                        })
                        .collect();

                    let mut action: Option<SidebarAction> = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (id, name, status) in &conn_data {
                            let selected = self.selected_conn == Some(*id);
                            let status_icon = match status {
                                ConnectionStatusKind::Connected => "🟢",
                                ConnectionStatusKind::Connecting => "🟡",
                                ConnectionStatusKind::Disconnected => "⚪",
                                ConnectionStatusKind::Error(_) => "🔴",
                            };

                            // Connection row
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(selected, format!("{status_icon} {name}"))
                                    .clicked()
                                {
                                    action = Some(SidebarAction::Select(*id));
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| match status {
                                        ConnectionStatusKind::Connected => {
                                            if ui
                                                .small_button("⏏")
                                                .on_hover_text(S::DISCONNECT)
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::Disconnect(*id));
                                            }
                                        }
                                        ConnectionStatusKind::Connecting => {}
                                        _ => {
                                            if ui
                                                .small_button("▶")
                                                .on_hover_text(S::CONNECT)
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::Connect(*id));
                                            }
                                        }
                                    },
                                );
                            });

                            // Resource tree (only for connected profiles)
                            if matches!(status, ConnectionStatusKind::Connected) {
                                ui.indent(format!("tree_{id}"), |ui| {
                                    // Pub/Sub section
                                    egui::CollapsingHeader::new(S::SECTION_PUBSUB)
                                        .id_salt(format!("pubsub_{id}"))
                                        .show(ui, |ui| {
                                            if ui
                                                .selectable_label(false, S::OPEN_PUBLISHER)
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::OpenTab(
                                                    TabKind::Publisher {
                                                        connection_id: *id,
                                                        connection_name: name.clone(),
                                                        state: PublisherState::default(),
                                                    },
                                                ));
                                            }
                                            if ui
                                                .selectable_label(false, S::OPEN_SUBSCRIBER)
                                                .clicked()
                                            {
                                                action = Some(SidebarAction::OpenTab(
                                                    TabKind::Subscriber {
                                                        connection_id: *id,
                                                        connection_name: name.clone(),
                                                        state: SubscriberState::default(),
                                                    },
                                                ));
                                            }
                                        });

                                    // Streams section
                                    egui::CollapsingHeader::new(S::SECTION_STREAMS)
                                        .id_salt(format!("streams_{id}"))
                                        .show(ui, |_ui| {
                                            // Streams will be listed here once discovered
                                        });

                                    // KV section
                                    egui::CollapsingHeader::new(S::SECTION_KV)
                                        .id_salt(format!("kv_{id}"))
                                        .show(ui, |_ui| {
                                            // KV buckets will be listed here
                                        });

                                    // Object Store section
                                    egui::CollapsingHeader::new(S::SECTION_OBJECT_STORE)
                                        .id_salt(format!("objstore_{id}"))
                                        .show(ui, |_ui| {
                                            // Objects will be listed here
                                        });
                                });
                            }
                        }
                    });

                    // Apply actions
                    match action {
                        Some(SidebarAction::Select(id)) => self.selected_conn = Some(id),
                        Some(SidebarAction::Connect(id)) => self.connect(id),
                        Some(SidebarAction::Disconnect(id)) => self.disconnect(id),
                        Some(SidebarAction::OpenTab(tab)) => self.open_tab(tab),
                        None => {}
                    }

                    // Edit/delete for selected connection
                    if let Some(id) = self.selected_conn {
                        ui.separator();
                        let cfg_clone =
                            self.config.connections.iter().find(|c| c.id == id).cloned();
                        ui.horizontal(|ui| {
                            if ui.small_button(S::EDIT).clicked()
                                && let Some(cfg) = &cfg_clone
                            {
                                self.open_edit_editor(cfg);
                            }
                            if ui.small_button(S::DELETE).clicked() {
                                self.editor.delete_confirm = Some(id);
                            }
                        });
                    }
                });

            // ─── Dock Area (central) ───
            DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(
                    ui,
                    &mut AppTabViewer {
                        backend: &self.backend,
                    },
                );

            // ─── Toast overlay ───
            self.toasts.show(ui.ctx());
        }
    }

    enum SidebarAction {
        Select(u64),
        Connect(u64),
        Disconnect(u64),
        OpenTab(TabKind),
    }
}
