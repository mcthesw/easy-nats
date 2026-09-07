use super::*;

fn app() -> EasyNatsApp {
    EasyNatsApp::new(
        Default::default(),
        crate::theme::ThemeId::EguiDark,
        Default::default(),
    )
}

#[test]
fn current_disconnected_tab_does_not_fall_back_to_another_connection() {
    let mut app = app();
    app.open_or_focus_publisher_tab(1);
    app.selected_conn = Some(2);
    app.conn_statuses.insert(2, ConnectionStatusKind::Connected);
    assert_eq!(app.command_connection(), Some(1));
    assert!(
        app.disabled_reason(&Context::default(), Command::Subscriber)
            .is_some()
    );
}

#[test]
fn close_stays_in_the_focused_split_and_keeps_welcome() {
    let mut app = app();
    app.dock_state = egui_dock::DockState::new(vec![TabKind::Welcome, TabKind::Settings]);
    let [_, right] = app.dock_state.main_surface_mut().split_right(
        egui_dock::NodeIndex::root(),
        0.5,
        vec![TabKind::LogViewer],
    );
    app.dock_state
        .set_focused_node_and_surface(egui_dock::NodePath::new(
            egui_dock::SurfaceIndex::main(),
            right,
        ));
    let ctx = Context::default();
    assert!(matches!(app.active_command_tab(), Some(TabKind::LogViewer)));
    app.execute_command(&ctx, Command::Close);
    assert_eq!(app.dock_state.iter_all_tabs().count(), 2);
    assert!(
        app.dock_state
            .iter_all_tabs()
            .any(|(_, t)| matches!(t, TabKind::Welcome))
    );
}

#[test]
fn command_search_accepts_both_languages_and_multiple_keywords() {
    let entry = Entry::Workspace(Command::Publisher);
    assert!(entry.matches("publish"));
    assert!(entry.matches("发布"));
    assert!(entry.matches("pub 发布"));
    assert!(!entry.matches("delete"));
}
#[test]
fn closing_palette_restores_the_original_text_field() {
    let ctx = Context::default();
    let mut app = app();
    let mut value = String::new();
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                keyboard::begin_frame(&ctx);
                {
                    let _form = keyboard::Form::new(ui, "palette_origin", false);
                    keyboard::text_edit(
                        ui,
                        egui::TextEdit::multiline(&mut value).id(Id::new("origin")),
                        true,
                    );
                    keyboard::primary_button(ui, true, "Save");
                }
                keyboard::end_frame(&ctx);
                app.render_command_palette(&ctx);
            },
        );
    };
    frame(vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("origin")));
    frame(vec![]);
    open_palette(&ctx);
    frame(vec![]);
    frame(vec![]);
    frame(vec![egui::Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    frame(vec![]);
    frame(vec![]);
    assert_eq!(ctx.memory(|m| m.focused()), Some(Id::new("origin")));
}
#[test]
fn palette_navigation_keeps_focus_in_destination_split() {
    let ctx = Context::default();
    let mut app = app();
    let [left, _] = app.dock_state.main_surface_mut().split_right(
        egui_dock::NodeIndex::root(),
        0.5,
        vec![TabKind::SearchWorkspace {
            state: Default::default(),
        }],
    );
    app.dock_state
        .set_focused_node_and_surface(egui_dock::NodePath::new(
            egui_dock::SurfaceIndex::main(),
            left,
        ));
    let mut origin = String::new();
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                keyboard::begin_frame(&ctx);
                ui.columns(2, |columns| {
                    {
                        let ui = &mut columns[0];
                        let _form = keyboard::Form::new(ui, "origin_form", false);
                        keyboard::text_edit(
                            ui,
                            egui::TextEdit::singleline(&mut origin).id(Id::new("origin")),
                            false,
                        );
                    }
                    let (_, TabKind::SearchWorkspace { state }) = app
                        .dock_state
                        .iter_all_tabs_mut()
                        .find(|(_, tab)| matches!(tab, TabKind::SearchWorkspace { .. }))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    crate::tabs::search_workspace_ui(&mut columns[1], state, &[], &mut Vec::new());
                });
                keyboard::end_frame(&ctx);
                app.render_command_palette(&ctx);
            },
        );
    };
    frame(vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("origin")));
    frame(vec![]);
    open_palette(&ctx);
    let mut p = palette(&ctx);
    p.query = "search workspace".into();
    save_palette(&ctx, p);
    frame(vec![]);
    frame(vec![]);
    frame(vec![egui::Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    frame(vec![]);
    frame(vec![]);
    frame(vec![egui::Event::Text("destination".into())]);
    let (_, TabKind::SearchWorkspace { state }) = app
        .dock_state
        .iter_all_tabs()
        .find(|(_, tab)| matches!(tab, TabKind::SearchWorkspace { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(state.query, "destination");
    assert!(origin.is_empty());
}
