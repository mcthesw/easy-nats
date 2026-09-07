use super::*;

fn app() -> EasyNatsApp {
    let mut app = EasyNatsApp::new(
        Default::default(),
        crate::theme::ThemeId::EguiDark,
        Default::default(),
    );
    app.config.connections = vec![
        nats_backend::ConnectionConfig::new(101, "local-test".into(), "invalid://test".into()),
        nats_backend::ConnectionConfig::new(102, "生产环境".into(), "invalid://test".into()),
    ];
    app
}

#[test]
fn saved_connections_are_searchable_by_name_and_bilingual_action() {
    let mut app = app();
    let entries: Vec<_> = app.connection_commands().collect();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].matches("open connection LOCAL-test"));
    assert!(entries[1].matches("打开 连接 生产"));
    assert!(!entries[0].matches("生产"));
    app.config.connections.clear();
    assert_eq!(app.connection_commands().count(), 0);
}

#[test]
fn opening_connection_selects_saved_profile_and_prevents_duplicate_attempts() {
    let mut app = app();
    let ctx = Context::default();
    app.execute_open_connection(&ctx, 102);
    assert_eq!(app.selected_conn, Some(102));
    assert!(app.user_wants_connected.contains(&102));
    assert!(!app.user_wants_connected.contains(&101));
    // A repeated invocation must not change selection or restart the connection.
    app.selected_conn = Some(101);
    app.execute_open_connection(&ctx, 102);
    assert_eq!(app.selected_conn, Some(101));
    assert!(app.open_connection_disabled_reason(&ctx, 102).is_some());
}

#[test]
fn unavailable_connections_and_active_windows_cannot_connect() {
    let mut app = app();
    let ctx = Context::default();
    for status in [
        ConnectionStatusKind::Connected,
        ConnectionStatusKind::Connecting,
    ] {
        app.conn_statuses.insert(101, status);
        app.execute_open_connection(&ctx, 101);
        assert!(app.user_wants_connected.is_empty());
    }
    app.conn_statuses
        .insert(101, ConnectionStatusKind::Error("test".into()));
    assert!(app.open_connection_disabled_reason(&ctx, 101).is_none());
    app.execute_open_connection(&ctx, 999);
    assert!(app.selected_conn.is_none());
    let _ = ctx.run_ui(Default::default(), |ui| {
        let _form = keyboard::Form::new(ui, "connection_editor", true);
        app.execute_open_connection(&ctx, 101);
        assert!(app.user_wants_connected.is_empty());
    });
}

#[test]
fn enter_runs_filtered_connection_and_closes_palette() {
    let mut app = app();
    let ctx = Context::default();
    open_palette(&ctx);
    let mut p = palette(&ctx);
    p.query = "local-test".into();
    save_palette(&ctx, p);
    for events in [
        vec![],
        vec![],
        vec![egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    ] {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |_| {
                app.render_command_palette(&ctx);
            },
        );
    }
    assert!(!palette(&ctx).open);
    assert!(!keyboard::palette_open(&ctx));
    assert_eq!(app.selected_conn, Some(101));
    assert!(app.user_wants_connected.contains(&101));
}
