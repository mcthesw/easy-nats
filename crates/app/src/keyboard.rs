//! Keyboard routing shared by forms, shortcuts, and the command palette.
//!
//! A command targets a stable form ID, never whichever button happens to render
//! first. Widgets register each frame; execution rechecks their current validity.
use eframe::egui::{self, Context, Event, Id, Key, Modifiers, Response, Ui};

use crate::i18n::t;

#[derive(Clone)]
pub(crate) struct FormAction {
    pub id: Id,
    pub form: Id,
    pub label: String,
    pub enabled: bool,
    pub secondary: bool,
}

#[derive(Clone)]
struct Field {
    id: Id,
    form: Id,
    multiline: bool,
}

#[derive(Clone)]
struct FormInfo {
    id: Id,
    layer: egui::LayerId,
    window: bool,
    restore: Option<Id>,
    connected: bool,
}

#[derive(Clone, Default)]
struct State {
    forms: Vec<FormInfo>,
    fields: Vec<Field>,
    actions: Vec<FormAction>,
    previous_forms: Vec<FormInfo>,
    previous_fields: Vec<Field>,
    previous_actions: Vec<FormAction>,
    stack: Vec<Id>,
    pending: Option<(Id, bool)>,
    focused_form: Option<Id>,
    active_window: Option<Id>,
    composing: bool,
    ime_blocked: bool,
    palette: bool,
    focus_first: Option<Id>,
    focus_tab: Option<Id>,
    focus_next: bool,
    restore_focus: Option<Id>,
    cancel: Option<Id>,
    connections: std::collections::HashSet<u64>,
}

fn state<R>(ctx: &Context, f: impl FnOnce(&mut State) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<State>(Id::new("keyboard.routing"))))
}

pub(crate) fn shortcut(key: Key, shift: bool) -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(
        Modifiers {
            shift,
            ..Modifiers::COMMAND
        },
        key,
    )
}

/// Unlike consume_key, extra Shift/Alt are not accepted and held keys do not
/// execute a command repeatedly. Repeats are still swallowed.
pub(crate) fn take_key(ctx: &Context, modifiers: Modifiers, key: Key) -> bool {
    ctx.input_mut(|input| {
        let mut pressed = false;
        input.events.retain(|event| {
            if let Event::Key {
                key: event_key,
                pressed: true,
                repeat,
                modifiers: actual,
                ..
            } = event
                && *event_key == key
                && actual.matches_exact(modifiers)
            {
                pressed |= !repeat;
                false
            } else {
                true
            }
        });
        pressed
    })
}

pub(crate) fn ime_blocked(ctx: &Context) -> bool {
    state(ctx, |s| s.ime_blocked)
}

pub(crate) fn begin_frame(ctx: &Context) {
    let events = ctx.input(|i| i.events.clone());
    let focus = ctx.memory(|m| m.focused());
    let focused_layer = focus
        .and_then(|id| ctx.read_response(id))
        .map(|r| r.layer_id);
    let clicked_layer = ctx
        .input(|i| {
            i.pointer
                .any_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        })
        .and_then(|pos| ctx.layer_id_at(pos));
    state(ctx, |s| {
        s.previous_forms = std::mem::take(&mut s.forms);
        s.previous_fields = std::mem::take(&mut s.fields);
        s.previous_actions = std::mem::take(&mut s.actions);
        s.stack.clear();
        s.cancel = None;
        s.ime_blocked = s.composing;
        for event in &events {
            if let Event::Ime(ime) = event {
                s.ime_blocked = true;
                match ime {
                    egui::ImeEvent::Preedit { text, .. } => s.composing = !text.is_empty(),
                    egui::ImeEvent::Commit(_) => s.composing = false,
                    #[allow(deprecated)]
                    egui::ImeEvent::Disabled => s.composing = false,
                    _ => {}
                }
            }
        }
        if events
            .iter()
            .any(|e| matches!(e, Event::WindowFocused(false)))
        {
            s.composing = false;
        }
        let previous_window = s.active_window;
        s.active_window = s
            .previous_forms
            .iter()
            .rev()
            .find(|f| f.window && Some(f.layer) == clicked_layer)
            .or_else(|| {
                s.previous_forms
                    .iter()
                    .rev()
                    .find(|f| f.window && Some(f.layer) == focused_layer)
            })
            .or_else(|| {
                s.previous_forms
                    .iter()
                    .find(|f| f.window && Some(f.id) == previous_window)
            })
            .or_else(|| s.previous_forms.iter().rev().find(|f| f.window))
            .map(|f| f.id);
        if !s.palette {
            s.focused_form = s
                .previous_fields
                .iter()
                .find(|f| Some(f.id) == focus)
                .map(|f| f.form)
                .or_else(|| {
                    s.previous_actions
                        .iter()
                        .find(|a| Some(a.id) == focus)
                        .map(|a| a.form)
                })
                .or_else(|| {
                    s.previous_forms
                        .iter()
                        .find(|f| Some(f.id) == s.active_window && Some(f.layer) == focused_layer)
                        .map(|f| f.id)
                });
        }
    });
    if ime_blocked(ctx) {
        // Text/IME events still reach TextEdit; only application-sensitive keys
        // are suppressed, including the Enter which commits the composition.
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: Key::Enter | Key::Escape | Key::Tab | Key::ArrowUp | Key::ArrowDown,
                        ..
                    }
                )
            })
        });
        return;
    }
    if palette_open(ctx) {
        return;
    }
    if egui::Popup::is_any_open(ctx) {
        return;
    }
    let secondary = take_key(ctx, shortcut(Key::Enter, true).modifiers, Key::Enter);
    let primary = take_key(ctx, Modifiers::COMMAND, Key::Enter);
    let singleline = state(ctx, |s| {
        s.previous_fields
            .iter()
            .any(|f| Some(f.id) == focus && !f.multiline)
    });
    let enter = singleline && take_key(ctx, Modifiers::NONE, Key::Enter);
    if secondary || primary || enter {
        queue_focused(ctx, secondary);
    }
    if state(ctx, |s| s.active_window.is_some()) && take_key(ctx, Modifiers::NONE, Key::Escape) {
        state(ctx, |s| s.cancel = s.active_window);
    }
}

pub(crate) fn end_frame(ctx: &Context) {
    // Restore after widgets have rendered. A just-closed modal can otherwise
    // make the target TextEdit surrender a focus request made at frame start.
    if !palette_open(ctx)
        && let Some(id) = state(ctx, |s| s.restore_focus)
    {
        if let Some(response) = ctx.read_response(id) {
            if response.enabled() && ctx.memory(|m| m.allows_interaction(response.layer_id)) {
                response.request_focus();
                state(ctx, |s| s.restore_focus = None);
            }
            ctx.request_repaint();
        } else {
            state(ctx, |s| s.restore_focus = None);
        }
    }

    let focus = ctx.memory(|m| m.focused());
    let restore = state(ctx, |s| {
        s.pending = None;
        if s.palette || s.restore_focus.is_some() {
            return None;
        }
        s.previous_forms
            .iter()
            .rev()
            .find(|old| {
                old.window
                    && !s.forms.iter().any(|current| current.id == old.id)
                    && (focus.is_none()
                        || s.previous_fields
                            .iter()
                            .any(|f| f.form == old.id && Some(f.id) == focus)
                        || s.previous_actions
                            .iter()
                            .any(|a| a.form == old.id && Some(a.id) == focus))
            })
            .and_then(|old| old.restore)
    });
    if let Some(id) = restore {
        restore_focus(ctx, id);
    }
}

pub(crate) fn palette_open(ctx: &Context) -> bool {
    state(ctx, |s| s.palette)
}
pub(crate) fn set_palette_open(ctx: &Context, open: bool) {
    state(ctx, |s| s.palette = open);
}
pub(crate) fn window_active(ctx: &Context) -> bool {
    state(ctx, |s| {
        s.active_window.is_some() || s.forms.iter().any(|f| f.window)
    })
}

pub(crate) fn current_actions(ctx: &Context) -> Vec<FormAction> {
    state(ctx, |s| {
        s.actions
            .iter()
            .filter(|a| {
                Some(a.form) == s.focused_form && s.active_window.is_none_or(|w| w == a.form)
            })
            .cloned()
            .collect()
    })
}

pub(crate) fn queue_action(ctx: &Context, action: &FormAction) {
    state(ctx, |s| s.pending = Some((action.form, action.secondary)));
}

pub(crate) fn queue_focused(ctx: &Context, secondary: bool) {
    state(ctx, |s| {
        if let Some(form) = s.focused_form
            && s.active_window.is_none_or(|w| w == form)
        {
            s.pending = Some((form, secondary));
        }
    });
}

/// RAII keeps nested and adjacent forms independent even when they early-return.
pub(crate) struct Form {
    ctx: Context,
    id: Id,
}
impl Form {
    pub(crate) fn new(ui: &Ui, salt: impl egui::AsIdSalt, window: bool) -> Self {
        let ctx = ui.ctx().clone();
        let id = ui.id().with(salt);
        let focus = ctx.memory(|m| m.focused());
        let restore = state(&ctx, |s| {
            s.previous_forms
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.restore)
        });
        state(&ctx, |s| {
            if ((window && restore.is_none()) || s.focus_next) && !s.palette {
                s.focus_next = false;
                s.focus_first = Some(id);
                if window {
                    s.active_window = Some(id);
                }
                s.focused_form = Some(id);
            }
            s.forms.push(FormInfo {
                id,
                window,
                layer: ui.layer_id(),
                restore: restore.unwrap_or(focus),
                connected: true,
            });
            s.stack.push(id);
        });
        Self { ctx, id }
    }
}
impl Form {
    pub(crate) fn connected(
        ui: &Ui,
        salt: impl egui::AsIdSalt,
        window: bool,
        connection: u64,
    ) -> Self {
        let form = Self::new(ui, salt, window);
        state(ui.ctx(), |s| {
            let connected = s.connections.contains(&connection);
            if let Some(info) = s.forms.iter_mut().find(|info| info.id == form.id) {
                info.connected = connected;
            }
        });
        form
    }
}
pub(crate) fn restore_focus(ctx: &Context, id: Id) {
    state(ctx, |s| s.restore_focus = Some(id));
    ctx.request_repaint();
}
pub(crate) fn request_tab_focus(ctx: &Context, id: Id) {
    state(ctx, |s| s.focus_tab = Some(id));
}
pub(crate) fn enter_tab(ctx: &Context, id: Id) {
    state(ctx, |s| {
        if s.focus_tab == Some(id) {
            s.focus_next = true;
            s.focus_tab = None;
        }
    });
}
pub(crate) fn set_connections(ctx: &Context, connections: impl Iterator<Item = u64>) {
    state(ctx, |s| s.connections = connections.collect());
}

impl Drop for Form {
    fn drop(&mut self) {
        state(&self.ctx, |s| {
            debug_assert_eq!(s.stack.pop(), Some(self.id));
        });
    }
}

pub(crate) fn text_edit(ui: &mut Ui, edit: egui::TextEdit<'_>, multiline: bool) -> Response {
    let response = ui.add(edit);
    register_field(ui, &response, multiline);
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    tab: false,
                    escape: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                },
            )
        });
    }
    response
}

pub(crate) fn combo_box<R>(
    ui: &mut Ui,
    combo: egui::ComboBox,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let response = combo.show_ui(ui, contents);
    register_field(ui, &response.response, true);
    response
}

pub(crate) fn singleline(ui: &mut Ui, value: &mut String) -> Response {
    text_edit(ui, egui::TextEdit::singleline(value), false)
}

pub(crate) fn register_field(ui: &Ui, response: &Response, multiline: bool) {
    let has_focus = response.has_focus();
    let first = state(ui.ctx(), |s| {
        let Some(&form) = s.stack.last() else {
            return false;
        };
        s.fields.push(Field {
            id: response.id,
            form,
            multiline,
        });
        if has_focus && !s.palette {
            s.focused_form = Some(form);
        }
        if s.focus_first == Some(form) && response.enabled() {
            s.focus_first = None;
            true
        } else {
            false
        }
    });
    if first {
        response.request_focus();
    }
}

pub(crate) fn primary_button(ui: &mut Ui, enabled: bool, label: &str) -> bool {
    action_button(ui, enabled, label, false)
}

pub(crate) fn action_button(ui: &mut Ui, enabled: bool, label: &str, secondary: bool) -> bool {
    let enabled = enabled
        && state(ui.ctx(), |s| {
            s.forms
                .iter()
                .find(|f| Some(&f.id) == s.stack.last())
                .is_none_or(|f| f.connected)
        });
    let hint = ui.ctx().format_shortcut(&shortcut(Key::Enter, secondary));
    let response = ui
        .add_enabled(enabled, egui::Button::new(label))
        .on_hover_text(hint);
    let execute = state(ui.ctx(), |s| {
        let Some(&form) = s.stack.last() else {
            return false;
        };
        s.actions.push(FormAction {
            id: response.id,
            form,
            label: label.to_owned(),
            enabled,
            secondary,
        });
        if s.pending == Some((form, secondary)) && !s.palette {
            s.pending = None;
            return enabled && !s.ime_blocked && s.active_window.is_none_or(|w| w == form);
        }
        false
    });
    response.clicked() || execute
}

pub(crate) fn cancel_button(ui: &mut Ui) -> bool {
    let response = ui.button(t("common.cancel")).on_hover_text("Esc");
    register_field(ui, &response, true);
    let cancel = state(ui.ctx(), |s| {
        s.stack.last().is_some_and(|id| s.cancel == Some(*id))
    });
    if response.clicked() || cancel {
        let restore = state(ui.ctx(), |s| {
            s.forms
                .iter()
                .find(|f| Some(&f.id) == s.stack.last())
                .and_then(|f| f.restore)
        });
        if let Some(id) = restore {
            restore_focus(ui.ctx(), id);
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests;
