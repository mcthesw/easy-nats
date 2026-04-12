fn main() {
    // Initialize structured logging with RUST_LOG env filter
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
        Box::new(|_cc| Ok(Box::new(app::EasyNatsApp::new()))),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}

mod app {
    use eframe::egui;
    use nats_backend::{
        AppConfig, AuthMethod, BackendCommand, BackendEvent, BackendHandle, ConnectionConfig,
        ConnectionStatusKind,
    };
    use std::collections::HashMap;

    pub struct EasyNatsApp {
        backend: BackendHandle,
        config: AppConfig,
        /// Runtime connection statuses keyed by connection id.
        conn_statuses: HashMap<u64, ConnectionStatusKind>,
        /// Currently selected connection id for detail view.
        selected_conn: Option<u64>,
        /// State of the connection editor form.
        editor: ConnectionEditor,
    }

    #[derive(Default)]
    struct ConnectionEditor {
        visible: bool,
        /// If editing an existing connection, holds its id.
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
        /// Pending delete confirmation dialog.
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
        const ALL: [AuthKindSelection; 6] = [
            Self::None,
            Self::Token,
            Self::UserPassword,
            Self::NKey,
            Self::CredentialsFile,
            Self::TlsClientCert,
        ];

        fn label(self) -> &'static str {
            match self {
                Self::None => "No Auth",
                Self::Token => "Token",
                Self::UserPassword => "User / Password",
                Self::NKey => "NKey",
                Self::CredentialsFile => "Credentials File",
                Self::TlsClientCert => "TLS Client Certificate",
            }
        }
    }

    impl EasyNatsApp {
        pub fn new() -> Self {
            let config = AppConfig::load();
            Self {
                backend: BackendHandle::spawn(),
                config,
                conn_statuses: HashMap::new(),
                selected_conn: None,
                editor: ConnectionEditor::default(),
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
                        self.conn_statuses.insert(connection_id, status);
                    }
                    other => {
                        tracing::debug!(?other, "Received backend event");
                    }
                }
            }
            ctx.request_repaint();
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
                // Update existing
                if let Some(c) = self.config.connections.iter_mut().find(|c| c.id == id) {
                    c.name = self.editor.name.clone();
                    c.urls = vec![self.editor.url.clone()];
                    c.auth = auth;
                    c.tls_enabled = self.editor.tls_enabled;
                }
            } else {
                // Create new
                let id = self.config.next_connection_id();
                let cfg = ConnectionConfig {
                    id,
                    name: self.editor.name.clone(),
                    urls: vec![self.editor.url.clone()],
                    auth,
                    tls_enabled: self.editor.tls_enabled,
                };
                self.config.connections.push(cfg);
            }
            self.config.save();
            self.editor.visible = false;
        }

        fn delete_connection(&mut self, id: u64) {
            // Disconnect if connected
            self.disconnect(id);
            self.conn_statuses.remove(&id);
            self.config.connections.retain(|c| c.id != id);
            self.config.save();
            if self.selected_conn == Some(id) {
                self.selected_conn = None;
            }
        }
    }

    impl eframe::App for EasyNatsApp {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.handle_events(ctx);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            // Connection editor window
            let mut save_requested = false;
            if self.editor.visible {
                let title = if self.editor.editing_id.is_some() {
                    "Edit Connection"
                } else {
                    "New Connection"
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
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.editor.name);
                                ui.end_row();

                                ui.label("URL:");
                                ui.text_edit_singleline(&mut self.editor.url);
                                ui.end_row();

                                ui.label("Auth:");
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
                                        ui.label("Token:");
                                        ui.text_edit_singleline(&mut self.editor.token);
                                        ui.end_row();
                                    }
                                    AuthKindSelection::UserPassword => {
                                        ui.label("Username:");
                                        ui.text_edit_singleline(&mut self.editor.username);
                                        ui.end_row();
                                        ui.label("Password:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.editor.password)
                                                .password(true),
                                        );
                                        ui.end_row();
                                    }
                                    AuthKindSelection::NKey => {
                                        ui.label("NKey Seed:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.editor.nkey_seed)
                                                .password(true),
                                        );
                                        ui.end_row();
                                    }
                                    AuthKindSelection::CredentialsFile => {
                                        ui.label("Creds File:");
                                        ui.text_edit_singleline(&mut self.editor.creds_path);
                                        ui.end_row();
                                    }
                                    AuthKindSelection::TlsClientCert => {
                                        ui.label("Cert Path:");
                                        ui.text_edit_singleline(&mut self.editor.cert_path);
                                        ui.end_row();
                                        ui.label("Key Path:");
                                        ui.text_edit_singleline(&mut self.editor.key_path);
                                        ui.end_row();
                                    }
                                }

                                ui.label("TLS:");
                                ui.checkbox(&mut self.editor.tls_enabled, "Require TLS");
                                ui.end_row();
                            });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let name_ok = !self.editor.name.trim().is_empty();
                            let url_ok = !self.editor.url.trim().is_empty();
                            if ui
                                .add_enabled(name_ok && url_ok, egui::Button::new("Save"))
                                .clicked()
                            {
                                save_requested = true;
                            }
                            if ui.button("Cancel").clicked() {
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

            // Delete confirmation modal
            let mut do_delete: Option<u64> = None;
            if let Some(id) = self.editor.delete_confirm {
                let conn_name = self
                    .config
                    .connections
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                let mut still_open = true;
                egui::Window::new("Confirm Delete")
                    .open(&mut still_open)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Delete connection \"{}\"?", conn_name));
                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                do_delete = Some(id);
                            }
                            if ui.button("Cancel").clicked() {
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

            // ─── Sidebar ───
            egui::Panel::left("connections_panel")
                .default_size(220.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Connections");
                        if ui.button("＋").clicked() {
                            self.open_new_editor();
                        }
                    });
                    ui.separator();

                    // Collect data first to avoid borrow issues
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
                    for (id, name, status) in &conn_data {
                        let selected = self.selected_conn == Some(*id);
                        let status_icon = match status {
                            ConnectionStatusKind::Connected => "🟢",
                            ConnectionStatusKind::Connecting => "🟡",
                            ConnectionStatusKind::Disconnected => "⚪",
                            ConnectionStatusKind::Error(_) => "🔴",
                        };
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(selected, format!("{} {}", status_icon, name))
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
                                            .on_hover_text("Disconnect")
                                            .clicked()
                                        {
                                            action = Some(SidebarAction::Disconnect(*id));
                                        }
                                    }
                                    ConnectionStatusKind::Connecting => {}
                                    _ => {
                                        if ui.small_button("▶").on_hover_text("Connect").clicked()
                                        {
                                            action = Some(SidebarAction::Connect(*id));
                                        }
                                    }
                                },
                            );
                        });
                    }

                    // Apply action
                    match action {
                        Some(SidebarAction::Select(id)) => self.selected_conn = Some(id),
                        Some(SidebarAction::Connect(id)) => self.connect(id),
                        Some(SidebarAction::Disconnect(id)) => self.disconnect(id),
                        None => {}
                    }
                });

            // ─── Central Panel ───
            egui::CentralPanel::default().show_inside(ui, |ui| {
                if let Some(id) = self.selected_conn {
                    let status = self
                        .conn_statuses
                        .get(&id)
                        .cloned()
                        .unwrap_or(ConnectionStatusKind::Disconnected);
                    if let Some(cfg) = self.config.connections.iter().find(|c| c.id == id) {
                        let name = cfg.name.clone();
                        let urls = cfg.urls.join(", ");
                        let cfg_clone = cfg.clone();
                        ui.heading(&name);
                        ui.label(format!("URL: {urls}"));
                        ui.label(format!("Status: {status:?}"));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                self.open_edit_editor(&cfg_clone);
                            }
                            if ui.button("Delete").clicked() {
                                self.editor.delete_confirm = Some(id);
                            }
                        });
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select or create a connection to get started");
                    });
                }
            });
        }
    }

    enum SidebarAction {
        Select(u64),
        Connect(u64),
        Disconnect(u64),
    }
}
