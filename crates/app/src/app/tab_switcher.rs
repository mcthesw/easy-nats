use eframe::egui::{self, Context, Id, Key, Modifiers};

use super::model::EasyNatsApp;
use crate::{i18n::t, keyboard, tabs::TabKind};

#[derive(Clone, Default)]
struct Switcher {
    open: bool,
    held_ctrl: bool,
    leaf: Option<egui_dock::NodePath>,
    tabs: Vec<Id>,
    selected: usize,
    query: String,
    just_opened: bool,
    restore: Option<Id>,
    navigate: i8,
    confirm: bool,
}

fn state(ctx: &Context) -> Switcher {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Switcher>(Id::new("tab_switcher"))
            .clone()
    })
}
fn save(ctx: &Context, state: Switcher) {
    ctx.data_mut(|d| d.insert_temp(Id::new("tab_switcher"), state));
}

fn navigation_key(ctx: &Context, key: Key, held_ctrl: bool) -> bool {
    let plain = keyboard::take_key(ctx, Modifiers::NONE, key);
    let held = held_ctrl && keyboard::take_key(ctx, Modifiers::CTRL, key);
    let reverse = held_ctrl && keyboard::take_key(ctx, Modifiers::CTRL | Modifiers::SHIFT, key);
    plain || held || reverse
}

fn cycle(selected: usize, count: usize, backwards: bool) -> usize {
    if count == 0 {
        0
    } else {
        (selected + if backwards { count - 1 } else { 1 }) % count
    }
}

fn tab_label(tab: &TabKind) -> String {
    let title = tab.title(true);
    let resource = match tab {
        TabKind::Publisher { state, .. } => state.subject.clone(),
        TabKind::Subscriber { state, .. } => {
            if state.subscriptions.is_empty() {
                state.subject_input.clone()
            } else {
                state
                    .subscriptions
                    .iter()
                    .map(|s| s.subject.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        TabKind::KvBucket { .. } => "KV".into(),
        TabKind::ObjectStoreBucket { .. } => t("common.tab_object_store").into(),
        _ => String::new(),
    };
    if resource.is_empty() {
        title
    } else {
        format!("{title} · {resource}")
    }
}

impl EasyNatsApp {
    pub(crate) fn record_tab_visit(&self, ctx: &Context) {
        if state(ctx).open {
            return;
        }
        let live: Vec<Id> = self
            .dock_state
            .iter_all_tabs()
            .map(|(_, t)| t.tab_id())
            .collect();
        let active = self.active_command_tab().map(TabKind::tab_id);
        ctx.data_mut(|d| {
            let history = d.get_temp_mut_or_default::<Vec<Id>>(Id::new("tab_history"));
            history.retain(|id| live.contains(id));
            if let Some(active) = active {
                history.retain(|id| *id != active);
                history.insert(0, active);
            }
        });
    }

    fn switcher_tabs(&self, ctx: &Context, leaf: Option<egui_dock::NodePath>) -> Vec<Id> {
        let history = ctx
            .data_mut(|d| d.get_temp::<Vec<Id>>(Id::new("tab_history")))
            .unwrap_or_default();
        let mut tabs: Vec<Id> = self
            .dock_state
            .iter_all_tabs()
            .filter(|(path, _)| leaf.is_none_or(|leaf| path.node_path() == leaf))
            .map(|(_, tab)| tab.tab_id())
            .collect();
        tabs.sort_by_key(|id| history.iter().position(|h| h == id).unwrap_or(usize::MAX));
        tabs
    }

    pub(crate) fn open_tab_switcher(&self, ctx: &Context, held_ctrl: bool, backwards: bool) {
        if keyboard::window_active(ctx) {
            return;
        }
        self.record_tab_visit(ctx);
        let leaf = if held_ctrl { self.command_leaf() } else { None };
        if held_ctrl && leaf.is_none() {
            return;
        }
        let tabs = self.switcher_tabs(ctx, leaf);
        if tabs.is_empty() {
            return;
        }
        let selected = if held_ctrl {
            cycle(0, tabs.len(), backwards)
        } else {
            0
        };
        save(
            ctx,
            Switcher {
                open: true,
                held_ctrl,
                leaf,
                tabs,
                selected,
                restore: ctx.memory(|m| m.focused()),
                just_opened: true,
                ..Default::default()
            },
        );
        keyboard::set_palette_open(ctx, true);
        ctx.request_repaint();
    }

    pub(super) fn inherit_switcher_focus(&self, ctx: &Context, restore: Option<Id>) {
        let mut s = state(ctx);
        s.restore = restore;
        save(ctx, s);
    }

    fn finish_tab_switcher(&mut self, ctx: &Context, s: &Switcher, selected: Option<Id>) {
        save(ctx, Switcher::default());
        keyboard::set_palette_open(ctx, false);
        if let Some(id) = selected
            && let Some(path) = self.dock_state.find_tab_from(|tab| tab.tab_id() == id)
            && s.leaf.is_none_or(|leaf| path.node_path() == leaf)
        {
            let _ = self.dock_state.set_active_tab(path);
            self.dock_state
                .set_focused_node_and_surface(path.node_path());
            keyboard::request_tab_focus(ctx, id);
        } else if let Some(id) = s.restore {
            keyboard::restore_focus(ctx, id);
        }
        self.record_tab_visit(ctx);
    }

    // Consume navigation before the underlying dock or text fields see it.
    pub(crate) fn handle_tab_switcher_input(&mut self, ctx: &Context) -> bool {
        let mut s = state(ctx);
        if !s.open {
            return false;
        }
        let selected = s.tabs.get(s.selected).copied();
        s.tabs.retain(|id| {
            self.dock_state
                .find_tab_from(|tab| tab.tab_id() == *id)
                .is_some_and(|path| s.leaf.is_none_or(|leaf| path.node_path() == leaf))
        });
        s.selected = selected
            .and_then(|id| s.tabs.iter().position(|t| *t == id))
            .unwrap_or(0);
        if keyboard::ime_blocked(ctx) {
            save(ctx, s);
            return true;
        }
        let lost_focus = ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false)))
        });
        if lost_focus || navigation_key(ctx, Key::Escape, s.held_ctrl) || s.tabs.is_empty() {
            self.finish_tab_switcher(ctx, &s, None);
            return true;
        }
        let next = keyboard::take_key(ctx, Modifiers::CTRL, Key::Tab);
        let previous = keyboard::take_key(ctx, Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
        let down = navigation_key(ctx, Key::ArrowDown, s.held_ctrl);
        let up = navigation_key(ctx, Key::ArrowUp, s.held_ctrl);
        let enter = navigation_key(ctx, Key::Enter, s.held_ctrl);
        if !s.held_ctrl {
            s.navigate = if previous || up {
                -1
            } else if next || down {
                1
            } else {
                0
            };
            s.confirm = enter;
            save(ctx, s);
            return true;
        }
        let results = self.filtered_tabs(&s);
        if next || previous || down || up {
            let index = results
                .iter()
                .position(|(i, _)| *i == s.selected)
                .unwrap_or(0);
            if !results.is_empty() {
                s.selected = results[cycle(index, results.len(), previous || up)].0;
            }
        }
        if enter || (s.held_ctrl && !ctx.input(|i| i.modifiers.ctrl)) {
            let selected = results
                .iter()
                .find(|(i, _)| *i == s.selected)
                .map(|(i, _)| s.tabs[*i]);
            self.finish_tab_switcher(ctx, &s, selected);
        } else {
            save(ctx, s);
        }
        true
    }

    fn filtered_tabs(&self, s: &Switcher) -> Vec<(usize, String)> {
        s.tabs
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let (_, tab) = self
                    .dock_state
                    .iter_all_tabs()
                    .find(|(_, tab)| tab.tab_id() == *id)?;
                let label = tab_label(tab);
                let lower = label.to_lowercase();
                s.query
                    .split_whitespace()
                    .all(|word| lower.contains(&word.to_lowercase()))
                    .then_some((index, label))
            })
            .collect()
    }

    pub(crate) fn render_tab_switcher(&mut self, ctx: &Context) {
        let mut s = state(ctx);
        if !s.open {
            return;
        }
        let mut chosen = None;
        let id = Id::new("tab_switcher.modal");
        let modal = egui::Modal::new(id).area(egui::Modal::default_area(id).fade_in(false));
        let response = modal.show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.0).clamp(280.0, 600.0));
            ui.strong(t(if s.held_ctrl {
                "keyboard.recent_tabs"
            } else {
                "keyboard.switch_tab"
            }));
            ui.weak(t(if s.held_ctrl {
                "keyboard.current_split"
            } else {
                "keyboard.all_splits"
            }));
            if !s.held_ctrl {
                let search = ui.add(
                    egui::TextEdit::singleline(&mut s.query)
                        .id(Id::new("tab_switcher.search"))
                        .hint_text(t("keyboard.search_tabs"))
                        .desired_width(f32::INFINITY)
                        .return_key(None),
                );
                if s.just_opened {
                    search.request_focus();
                }
                if search.changed() {
                    s.selected = self.filtered_tabs(&s).first().map_or(0, |(i, _)| *i);
                }
            }
            let results = self.filtered_tabs(&s);
            if !results.iter().any(|(i, _)| *i == s.selected) {
                s.selected = results.first().map_or(0, |(i, _)| *i);
            }
            if s.navigate != 0 && !results.is_empty() {
                let index = results
                    .iter()
                    .position(|(i, _)| *i == s.selected)
                    .unwrap_or(0);
                s.selected = results[cycle(index, results.len(), s.navigate < 0)].0;
            }
            if s.confirm && results.iter().any(|(i, _)| *i == s.selected) {
                chosen = s.tabs.get(s.selected).copied();
            }
            s.navigate = 0;
            s.confirm = false;
            ui.add_space(6.0);
            ui.spacing_mut().scroll.floating = false;
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 180.0).clamp(140.0, 300.0))
                .show(ui, |ui| {
                    let results = self.filtered_tabs(&s);
                    if results.is_empty() {
                        ui.label(t("keyboard.no_tabs"));
                    }
                    for (index, label) in results {
                        let selected = index == s.selected;
                        let row = ui.add(
                            egui::Button::new(&label)
                                .selected(selected)
                                .frame(selected)
                                .min_size(egui::vec2(ui.available_width(), 30.0)),
                        );
                        if selected {
                            row.scroll_to_me(Some(egui::Align::Center));
                        }
                        if row.clicked() {
                            chosen = Some(s.tabs[index]);
                        }
                        row.on_hover_text(label);
                    }
                });
            ui.separator();
            ui.label(t(if s.held_ctrl {
                "keyboard.switcher_hold_help"
            } else {
                "keyboard.switcher_help"
            }));
        });
        if chosen.is_some() || response.backdrop_response.clicked() {
            self.finish_tab_switcher(ctx, &s, chosen);
        } else {
            s.just_opened = ctx.will_discard();
            save(ctx, s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> EasyNatsApp {
        let mut app = EasyNatsApp::new(
            Default::default(),
            crate::theme::ThemeId::EguiDark,
            Default::default(),
        );
        app.dock_state = egui_dock::DockState::new(vec![
            TabKind::Welcome,
            TabKind::Settings,
            TabKind::LogViewer,
        ]);
        app
    }
    fn focus(app: &mut EasyNatsApp, tab: TabKind) {
        let path = app
            .dock_state
            .find_tab_from(|t| t.tab_id() == tab.tab_id())
            .unwrap();
        app.dock_state.set_active_tab(path).unwrap();
        app.dock_state
            .set_focused_node_and_surface(path.node_path());
    }
    fn key(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    fn frame(app: &mut EasyNatsApp, ctx: &Context, modifiers: Modifiers, events: Vec<egui::Event>) {
        let _ = ctx.run_ui(
            egui::RawInput {
                modifiers,
                events,
                ..Default::default()
            },
            |_| {
                keyboard::begin_frame(ctx);
                app.handle_shortcuts(ctx);
                app.render_tab_switcher(ctx);
            },
        );
    }

    #[test]
    fn release_confirms_and_two_quick_switches_return_to_origin() {
        let mut app = app();
        let ctx = Context::default();
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::LogViewer);
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::Settings);
        app.record_tab_visit(&ctx);
        for expected in [TabKind::LogViewer, TabKind::Settings] {
            frame(
                &mut app,
                &ctx,
                Modifiers::CTRL,
                vec![key(Key::Tab, Modifiers::CTRL)],
            );
            assert!(state(&ctx).open);
            frame(
                &mut app,
                &ctx,
                Modifiers::NONE,
                vec![egui::Event::Key {
                    key: Key::Tab,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
            assert_eq!(
                app.active_command_tab().unwrap().tab_id(),
                expected.tab_id()
            );
            assert!(!state(&ctx).open);
        }
    }

    #[test]
    fn escape_keeps_origin_and_does_not_change_history() {
        let mut app = app();
        let ctx = Context::default();
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::Settings);
        app.record_tab_visit(&ctx);
        frame(
            &mut app,
            &ctx,
            Modifiers::CTRL,
            vec![key(Key::Tab, Modifiers::CTRL)],
        );
        frame(
            &mut app,
            &ctx,
            Modifiers::NONE,
            vec![key(Key::Escape, Modifiers::NONE)],
        );
        assert_eq!(
            app.active_command_tab().unwrap().tab_id(),
            TabKind::Settings.tab_id()
        );
        assert_eq!(app.switcher_tabs(&ctx, None)[0], TabKind::Settings.tab_id());
        assert!(!keyboard::palette_open(&ctx));
    }

    #[test]
    fn mru_is_limited_to_current_split_but_search_lists_all_splits() {
        let mut app = app();
        let [_, right] = app.dock_state.main_surface_mut().split_right(
            egui_dock::NodeIndex::root(),
            0.5,
            vec![TabKind::SearchWorkspace {
                state: Default::default(),
            }],
        );
        let ctx = Context::default();
        app.dock_state
            .set_focused_node_and_surface(egui_dock::NodePath::new(
                egui_dock::SurfaceIndex::main(),
                right,
            ));
        app.open_tab_switcher(&ctx, true, false);
        assert_eq!(state(&ctx).tabs.len(), 1);
        app.finish_tab_switcher(&ctx, &state(&ctx), None);
        app.open_tab_switcher(&ctx, false, false);
        assert_eq!(state(&ctx).tabs.len(), 4);
    }

    #[test]
    fn closed_selection_is_pruned_before_confirmation() {
        let mut app = app();
        let ctx = Context::default();
        focus(&mut app, TabKind::Settings);
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::LogViewer);
        app.open_tab_switcher(&ctx, true, false);
        assert_eq!(state(&ctx).tabs[1], TabKind::Settings.tab_id());
        app.remove_tabs_matching(|tab| matches!(tab, TabKind::Settings));
        frame(&mut app, &ctx, Modifiers::NONE, vec![]);
        assert!(!state(&ctx).open);
        assert!(
            app.dock_state
                .iter_all_tabs()
                .all(|(_, tab)| !matches!(tab, TabKind::Settings))
        );
    }

    #[test]
    fn search_and_enter_in_one_frame_use_the_new_query() {
        let mut app = app();
        app.open_or_focus_publisher_tab(42);
        let (_, publisher) = app
            .dock_state
            .iter_all_tabs_mut()
            .find(|(_, t)| matches!(t, TabKind::Publisher { .. }))
            .unwrap();
        let expected = publisher.tab_id();
        if let TabKind::Publisher { state, .. } = publisher {
            state.subject = "orders.created".into();
        }
        focus(&mut app, TabKind::Welcome);
        let ctx = Context::default();
        app.open_tab_switcher(&ctx, false, false);
        frame(&mut app, &ctx, Modifiers::NONE, vec![]);
        frame(&mut app, &ctx, Modifiers::NONE, vec![]);
        frame(
            &mut app,
            &ctx,
            Modifiers::NONE,
            vec![
                egui::Event::Text("orders.created".into()),
                key(Key::Enter, Modifiers::NONE),
            ],
        );
        assert_eq!(app.active_command_tab().unwrap().tab_id(), expected);
    }

    #[test]
    fn tab_search_matches_connection_and_subject() {
        let mut app = app();
        app.open_or_focus_publisher_tab(42);
        if let TabKind::Publisher {
            state,
            connection_name,
            ..
        } = app
            .dock_state
            .iter_all_tabs_mut()
            .find(|(_, t)| matches!(t, TabKind::Publisher { .. }))
            .unwrap()
            .1
        {
            state.subject = "orders.created".into();
            *connection_name = "Production".into();
        }
        let ctx = Context::default();
        app.open_tab_switcher(&ctx, false, false);
        let mut s = state(&ctx);
        s.query = "production orders".into();
        assert_eq!(app.filtered_tabs(&s).len(), 1);
        s.query = "missing".into();
        assert!(app.filtered_tabs(&s).is_empty());
    }
    #[test]
    fn reverse_cycles_mru_and_escape_works_while_ctrl_is_held() {
        let mut app = app();
        let ctx = Context::default();
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::LogViewer);
        app.record_tab_visit(&ctx);
        focus(&mut app, TabKind::Settings);
        frame(
            &mut app,
            &ctx,
            Modifiers::CTRL | Modifiers::SHIFT,
            vec![key(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT)],
        );
        let s = state(&ctx);
        assert_eq!(s.tabs[s.selected], TabKind::Welcome.tab_id());
        frame(
            &mut app,
            &ctx,
            Modifiers::CTRL,
            vec![key(Key::Escape, Modifiers::CTRL)],
        );
        assert!(!state(&ctx).open);
        assert_eq!(
            app.active_command_tab().unwrap().tab_id(),
            TabKind::Settings.tab_id()
        );
    }

    #[test]
    fn losing_app_focus_cancels_instead_of_switching() {
        let mut app = app();
        let ctx = Context::default();
        app.open_tab_switcher(&ctx, true, false);
        frame(
            &mut app,
            &ctx,
            Modifiers::NONE,
            vec![egui::Event::WindowFocused(false)],
        );
        assert!(!state(&ctx).open);
        assert_eq!(
            app.active_command_tab().unwrap().tab_id(),
            TabKind::Welcome.tab_id()
        );
    }
}
