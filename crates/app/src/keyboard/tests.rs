use super::*;

fn key(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

fn released(events: Vec<Event>) -> Vec<Event> {
    events
        .into_iter()
        .flat_map(|event| {
            let mut pair = vec![event.clone()];
            if let Event::Key {
                key,
                physical_key,
                modifiers,
                ..
            } = event
            {
                pair.push(Event::Key {
                    key,
                    physical_key,
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
            pair
        })
        .collect()
}

#[derive(Default)]
struct Harness {
    first: String,
    second: String,
    sent: usize,
    requested: usize,
    other: usize,
}
impl Harness {
    fn frame(&mut self, ctx: &Context, events: Vec<Event>) {
        self.frame_raw(ctx, released(events));
    }
    fn frame_raw(&mut self, ctx: &Context, events: Vec<Event>) {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                begin_frame(ctx);
                {
                    let _form = Form::new(ui, "first", false);
                    text_edit(
                        ui,
                        egui::TextEdit::singleline(&mut self.first).id(Id::new("single")),
                        false,
                    );
                    text_edit(
                        ui,
                        egui::TextEdit::multiline(&mut self.second).id(Id::new("multi")),
                        true,
                    );
                    self.sent += usize::from(primary_button(ui, !self.first.is_empty(), "Send"));
                    self.requested += usize::from(action_button(ui, true, "Request", true));
                }
                {
                    let _form = Form::new(ui, "second", false);
                    self.other += usize::from(primary_button(ui, true, "Other"));
                }
                end_frame(ctx);
            },
        );
    }
}

#[test]
fn submit_is_scoped_and_request_does_not_publish() {
    let ctx = Context::default();
    let mut h = Harness {
        first: "orders.created".into(),
        ..Default::default()
    };
    h.frame(&ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("single")));
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::NONE, false)]);
    assert_eq!((h.sent, h.requested, h.other), (1, 0, 0));
    h.frame(
        &ctx,
        vec![key(Key::Enter, shortcut(Key::Enter, true).modifiers, false)],
    );
    assert_eq!((h.sent, h.requested, h.other), (1, 1, 0));
    h.frame_raw(&ctx, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    h.frame_raw(&ctx, vec![key(Key::Enter, Modifiers::COMMAND, true)]);
    assert_eq!((h.sent, h.requested, h.other), (2, 1, 0));
}

#[test]
fn multiline_enter_edits_and_modified_enter_submits() {
    let ctx = Context::default();
    let mut h = Harness {
        first: "orders.created".into(),
        ..Default::default()
    };
    h.frame(&ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("multi")));
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::NONE, false)]);
    assert_eq!(h.second, "\n");
    assert_eq!(h.sent, 0);
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    assert_eq!(h.second, "\n");
    assert_eq!(h.sent, 1);
}

#[test]
fn invalid_forms_and_unregistered_focus_cannot_submit() {
    let ctx = Context::default();
    let mut h = Harness::default();
    h.frame(&ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("single")));
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    assert_eq!(h.sent, 0);
    h.first = "valid".into();
    ctx.memory_mut(|m| m.request_focus(Id::new("search_or_purge")));
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    assert_eq!(h.sent, 0);
}

#[test]
fn ime_commit_enter_never_submits() {
    let ctx = Context::default();
    let mut h = Harness {
        first: "valid".into(),
        ..Default::default()
    };
    h.frame(&ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("single")));
    h.frame(
        &ctx,
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "zhong".into(),
                active_range_chars: None,
            }),
            key(Key::Enter, Modifiers::NONE, false),
        ],
    );
    h.frame(
        &ctx,
        vec![
            Event::Ime(egui::ImeEvent::Commit("中".into())),
            key(Key::Enter, Modifiers::NONE, false),
        ],
    );
    assert_eq!(h.sent, 0);
    h.frame(&ctx, vec![key(Key::Enter, Modifiers::NONE, false)]);
    assert_eq!(h.sent, 1);
}

#[test]
fn suggestion_acceptance_is_separate_from_submission_and_escape_stays_closed() {
    let ctx = Context::default();
    let mut value = "orders".to_owned();
    let mut selected = None;
    let mut input = None;
    let mut sent = 0;
    let mut frame = |events: Vec<Event>| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                let _form = Form::new(ui, "suggestions", false);
                let response = crate::tabs::common::topic_history_text_edit(
                    ui,
                    "history",
                    &mut value,
                    &mut selected,
                    &["orders.created", "orders.updated"],
                );
                if input.is_none() {
                    response.request_focus();
                    input = Some(response.id);
                }
                sent += usize::from(primary_button(ui, true, "Submit"));
                end_frame(&ctx);
            },
        );
        (
            value.clone(),
            selected,
            sent,
            egui::Popup::is_any_open(&ctx),
        )
    };
    frame(vec![]);
    assert_eq!(frame(vec![]).1, None);
    frame(vec![key(Key::ArrowDown, Modifiers::NONE, false)]);
    let accepted = frame(vec![key(Key::Enter, Modifiers::NONE, false)]);
    assert_eq!((accepted.0.as_str(), accepted.2), ("orders.created", 0));
    assert_eq!(frame(vec![key(Key::Enter, Modifiers::NONE, false)]).2, 1);
}

#[test]
fn palette_queues_the_original_form_and_revalidates_it() {
    let ctx = Context::default();
    let mut h = Harness {
        first: "orders.created".into(),
        ..Default::default()
    };
    h.frame(&ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("single")));
    h.frame(&ctx, vec![]);
    let action = current_actions(&ctx)
        .into_iter()
        .find(|a| !a.secondary)
        .unwrap();
    set_palette_open(&ctx, true);
    ctx.memory_mut(|m| m.request_focus(Id::new("palette.search")));
    h.frame(&ctx, vec![]);
    assert_eq!(current_actions(&ctx)[0].form, action.form);
    set_palette_open(&ctx, false);
    queue_action(&ctx, &action);
    h.frame(&ctx, vec![]);
    assert_eq!((h.sent, h.other), (1, 0));
    h.first.clear();
    queue_action(&ctx, &action);
    h.frame(&ctx, vec![]);
    assert_eq!((h.sent, h.other), (1, 0));
}

#[test]
fn escape_dismisses_suggestions_until_input_changes_and_tab_leaves() {
    let ctx = Context::default();
    let mut value = "orders".to_owned();
    let mut selected = None;
    let mut first_id = None;
    let mut other = String::new();
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                let _form = Form::new(ui, "completion", false);
                let response = crate::tabs::common::topic_history_text_edit(
                    ui,
                    "history",
                    &mut value,
                    &mut selected,
                    &["orders.created"],
                );
                if first_id.is_none() {
                    first_id = Some(response.id);
                    response.request_focus();
                }
                text_edit(
                    ui,
                    egui::TextEdit::singleline(&mut other).id(Id::new("next_field")),
                    false,
                );
                end_frame(&ctx);
            },
        );
        (egui::Popup::is_any_open(&ctx), ctx.memory(|m| m.focused()))
    };
    frame(vec![]);
    assert!(frame(vec![]).0);
    assert!(!frame(vec![key(Key::Escape, Modifiers::NONE, false)]).0);
    assert!(!frame(vec![]).0);
    assert!(frame(vec![key(Key::ArrowDown, Modifiers::NONE, false)]).0);
    frame(vec![key(Key::Tab, Modifiers::NONE, false)]);
    let after_tab = frame(vec![]);
    assert!(!after_tab.0);
    assert_eq!(after_tab.1, Some(Id::new("next_field")));
}

#[test]
fn new_window_focus_survives_sizing_and_escape_cancels_only_that_window() {
    let ctx = Context::default();
    let mut visible = true;
    let mut value = String::new();
    let mut cancelled = 0;
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                if visible {
                    egui::Window::new("Editor").show(ui.ctx(), |ui| {
                        let _form = Form::new(ui, "window", true);
                        text_edit(
                            ui,
                            egui::TextEdit::singleline(&mut value).id(Id::new("window_field")),
                            false,
                        );
                        if cancel_button(ui) {
                            visible = false;
                            cancelled += 1;
                        }
                    });
                }
                end_frame(&ctx);
            },
        );
        ctx.memory(|m| m.focused())
    };
    frame(vec![]);
    frame(vec![]);
    assert_eq!(frame(vec![]), Some(Id::new("window_field")));
    frame(vec![key(Key::Escape, Modifiers::NONE, false)]);
    assert!(!visible);
    assert_eq!(cancelled, 1);
}

#[test]
fn connection_loss_disables_previously_available_form_action() {
    let ctx = Context::default();
    let mut value = "orders.created".to_owned();
    let mut sent = 0;
    let mut frame = |connected: bool, events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                set_connections(&ctx, connected.then_some(1).into_iter());
                let _form = Form::connected(ui, "connection_form", false, 1);
                text_edit(
                    ui,
                    egui::TextEdit::singleline(&mut value).id(Id::new("connected_field")),
                    false,
                );
                sent += usize::from(primary_button(ui, true, "Send"));
                end_frame(&ctx);
            },
        );
    };
    frame(true, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("connected_field")));
    frame(false, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    assert_eq!(sent, 0);
}

#[test]
fn typing_and_enter_in_one_frame_does_not_accept_a_stale_candidate() {
    let ctx = Context::default();
    let mut value = "orders".to_owned();
    let mut selected = None;
    let mut input = None;
    let mut submitted = Vec::new();
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                let _form = Form::new(ui, "same_frame", false);
                let response = crate::tabs::common::topic_history_text_edit(
                    ui,
                    "history",
                    &mut value,
                    &mut selected,
                    &["orders.created"],
                );
                if input.is_none() {
                    input = Some(response.id);
                    response.request_focus();
                }
                if primary_button(ui, true, "Submit") {
                    submitted.push(value.clone());
                }
                end_frame(&ctx);
            },
        );
    };
    frame(vec![]);
    frame(vec![]);
    frame(vec![key(Key::ArrowDown, Modifiers::NONE, false)]);
    frame(vec![
        Event::Text(".custom".into()),
        key(Key::Enter, Modifiers::NONE, false),
    ]);
    assert_eq!(submitted, ["orders.custom"]);
}

#[test]
fn background_search_cannot_submit_a_nonmodal_window() {
    let ctx = Context::default();
    let mut editor = "valid".to_owned();
    let mut query = String::new();
    let mut saved = 0;
    let mut frame = |events| {
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                egui::Window::new("Connection").show(ui.ctx(), |ui| {
                    let _form = Form::new(ui, "editor", true);
                    singleline(ui, &mut editor);
                    saved += usize::from(primary_button(ui, true, "Save"));
                });
                ui.add(egui::TextEdit::singleline(&mut query).id(Id::new("background_search")));
                end_frame(&ctx);
            },
        );
    };
    frame(vec![]);
    frame(vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("background_search")));
    frame(vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    assert_eq!(saved, 0);
}

#[test]
fn successful_window_submission_restores_its_opener() {
    let ctx = Context::default();
    let mut visible = false;
    let mut opener = String::new();
    let mut value = "valid".to_owned();
    let mut frame = |show: bool, events| {
        visible |= show;
        let _ = ctx.run_ui(
            egui::RawInput {
                events: released(events),
                ..Default::default()
            },
            |ui| {
                begin_frame(&ctx);
                ui.add(egui::TextEdit::singleline(&mut opener).id(Id::new("opener")));
                if visible {
                    egui::Window::new("Editor").show(ui.ctx(), |ui| {
                        let _form = Form::new(ui, "successful_editor", true);
                        singleline(ui, &mut value);
                        if primary_button(ui, true, "Save") {
                            visible = false;
                        }
                    });
                }
                end_frame(&ctx);
            },
        );
        ctx.memory(|m| m.focused())
    };
    frame(false, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("opener")));
    frame(true, vec![]);
    frame(false, vec![]);
    frame(false, vec![key(Key::Enter, Modifiers::COMMAND, false)]);
    frame(false, vec![]);
    assert_eq!(frame(false, vec![]), Some(Id::new("opener")));
}
