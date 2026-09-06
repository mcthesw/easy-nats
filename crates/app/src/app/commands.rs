use eframe::egui::{self, Context, Id, Key, Modifiers};
use nats_backend::ConnectionStatusKind;

use super::model::EasyNatsApp;
use crate::{i18n::t, keyboard, tabs::TabKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Search,
    Connection,
    Publisher,
    Subscriber,
    Settings,
    Logs,
    Schemas,
    Close,
    Next,
    Previous,
}
impl Command {
    const ALL: [Self; 10] = [
        Self::Search,
        Self::Connection,
        Self::Publisher,
        Self::Subscriber,
        Self::Settings,
        Self::Logs,
        Self::Schemas,
        Self::Close,
        Self::Next,
        Self::Previous,
    ];
    fn label(self) -> &'static str {
        t(match self {
            Self::Search => "search_workspace.title",
            Self::Connection => "sidebar.connection_new",
            Self::Publisher => "keyboard.publisher",
            Self::Subscriber => "keyboard.subscriber",
            Self::Settings => "settings.title",
            Self::Logs => "log_viewer.title",
            Self::Schemas => "message_schema.title",
            Self::Close => "keyboard.close_tab",
            Self::Next => "keyboard.next_tab",
            Self::Previous => "keyboard.previous_tab",
        })
    }
    fn keywords(self) -> &'static str {
        match self {
            Self::Search => "search workspace 搜索 工作区",
            Self::Connection => "new connection 新建连接",
            Self::Publisher => "publisher publish 发布",
            Self::Subscriber => "subscriber subscribe 订阅",
            Self::Settings => "settings preferences 设置",
            Self::Logs => "logs 日志",
            Self::Schemas => "message schemas 消息模式",
            Self::Close => "close tab 关闭标签",
            Self::Next => "next tab 下一标签",
            Self::Previous => "previous tab 上一标签",
        }
    }
    fn shortcut(self) -> Option<egui::KeyboardShortcut> {
        Some(match self {
            Self::Search => keyboard::shortcut(Key::F, true),
            Self::Connection => keyboard::shortcut(Key::N, false),
            Self::Publisher => keyboard::shortcut(Key::P, false),
            Self::Subscriber => keyboard::shortcut(Key::S, true),
            Self::Close => keyboard::shortcut(Key::W, false),
            Self::Next | Self::Previous => egui::KeyboardShortcut::new(
                Modifiers {
                    ctrl: true,
                    shift: self == Self::Previous,
                    ..Modifiers::NONE
                },
                Key::Tab,
            ),
            _ => return None,
        })
    }
}

#[derive(Clone, Default)]
struct Palette {
    open: bool,
    query: String,
    selected: usize,
    restore: Option<Id>,
    just_opened: bool,
}
fn palette(ctx: &Context) -> Palette {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Palette>(Id::new("command_palette.state"))
            .clone()
    })
}
fn save_palette(ctx: &Context, value: Palette) {
    ctx.data_mut(|d| d.insert_temp(Id::new("command_palette.state"), value));
}

pub(crate) fn open_palette(ctx: &Context) {
    let mut p = palette(ctx);
    if !p.open {
        p = Palette {
            open: true,
            restore: ctx.memory(|m| m.focused()),
            just_opened: true,
            ..Default::default()
        };
        egui::Popup::close_all(ctx);
        keyboard::set_palette_open(ctx, true);
        save_palette(ctx, p);
    }
}

#[derive(Clone)]
enum Entry {
    Workspace(Command),
    Form(keyboard::FormAction),
}
impl Entry {
    fn label(&self) -> &str {
        match self {
            Self::Workspace(c) => c.label(),
            Self::Form(a) => &a.label,
        }
    }
    fn shortcut(&self) -> Option<egui::KeyboardShortcut> {
        match self {
            Self::Workspace(c) => c.shortcut(),
            Self::Form(a) => Some(keyboard::shortcut(Key::Enter, a.secondary)),
        }
    }
    fn matches(&self, query: &str) -> bool {
        let keywords = match self {
            Self::Workspace(c) => c.keywords(),
            Self::Form(a) if a.secondary => "request 请求",
            Self::Form(_) => "save submit publish reply add 保存 提交 发布 回复 添加",
        };
        let haystack = format!("{} {keywords}", self.label()).to_lowercase();
        query
            .split_whitespace()
            .all(|word| haystack.contains(&word.to_lowercase()))
    }
}

impl EasyNatsApp {
    fn command_connection(&self) -> Option<u64> {
        self.active_command_tab()
            .and_then(TabKind::connection_id)
            .or(self.selected_conn)
    }
    fn command_leaf(&self) -> Option<egui_dock::NodePath> {
        self.dock_state.focused_leaf().or_else(|| {
            let node = self
                .dock_state
                .main_surface()
                .focused_leaf()
                .unwrap_or(egui_dock::NodeIndex::root());
            let path = egui_dock::NodePath::new(egui_dock::SurfaceIndex::main(), node);
            self.dock_state
                .node(path)
                .ok()
                .filter(|node| matches!(node, egui_dock::Node::Leaf(_)))
                .map(|_| path)
        })
    }
    fn active_command_tab(&self) -> Option<&TabKind> {
        let path = self.command_leaf()?;
        let egui_dock::Node::Leaf(leaf) = &self.dock_state[path.surface][path.node] else {
            return None;
        };
        leaf.tabs.get(leaf.active.0)
    }
    fn disabled_reason(&self, ctx: &Context, command: Command) -> Option<&'static str> {
        if keyboard::window_active(ctx) {
            return Some(t("keyboard.finish_window"));
        }
        match command {
            Command::Publisher | Command::Subscriber => {
                if !self.command_connection().is_some_and(|id| {
                    matches!(
                        self.conn_statuses.get(&id),
                        Some(ConnectionStatusKind::Connected)
                    )
                }) {
                    return Some(t("keyboard.connection_required"));
                }
            }
            Command::Close
                if self
                    .active_command_tab()
                    .is_none_or(|tab| matches!(tab, TabKind::Welcome)) =>
            {
                return Some(t("keyboard.no_closable_tab"));
            }
            Command::Next | Command::Previous if self.active_command_tab().is_none() => {
                return Some(t("keyboard.no_active_tab"));
            }
            _ => {}
        }
        None
    }
    fn execute_command(&mut self, ctx: &Context, command: Command) {
        if self.disabled_reason(ctx, command).is_some() {
            return;
        }
        match command {
            Command::Search => {
                self.open_or_focus_search_workspace();
                ctx.data_mut(|d| d.insert_temp(Id::new("keyboard.focus_search"), true));
            }
            Command::Connection => self.open_new_editor(),
            Command::Publisher => {
                if let Some(id) = self.command_connection() {
                    self.open_or_focus_publisher_tab(id);
                    if let Some(tab) = self.active_command_tab() {
                        keyboard::request_tab_focus(ctx, tab.tab_id());
                    }
                }
            }
            Command::Subscriber => {
                if let Some(id) = self.command_connection() {
                    self.open_or_focus_subscriber_tab(id);
                    if let Some(tab) = self.active_command_tab() {
                        keyboard::request_tab_focus(ctx, tab.tab_id());
                    }
                }
            }
            Command::Settings => self.open_or_focus_tab_kind(TabKind::Settings),
            Command::Logs => self.open_or_focus_tab_kind(TabKind::LogViewer),
            Command::Schemas => self.open_or_focus_message_schemas(),
            Command::Close => {
                if let Some(id) = self.active_command_tab().map(TabKind::tab_id) {
                    self.remove_tabs_matching(|tab| tab.tab_id() == id);
                }
            }
            Command::Next | Command::Previous => {
                if let Some(path) = self.command_leaf()
                    && let egui_dock::Node::Leaf(leaf) =
                        &mut self.dock_state[path.surface][path.node]
                    && !leaf.tabs.is_empty()
                {
                    let delta = if command == Command::Next {
                        1
                    } else {
                        leaf.tabs.len() - 1
                    };
                    leaf.active = egui_dock::TabIndex((leaf.active.0 + delta) % leaf.tabs.len());
                }
            }
        }
    }
    pub(crate) fn handle_shortcuts(&mut self, ctx: &Context) {
        if !self.runtime_mode.supports_local_files() || keyboard::ime_blocked(ctx) {
            return;
        }
        if keyboard::take_key(ctx, keyboard::shortcut(Key::P, true).modifiers, Key::P) {
            open_palette(ctx);
        }
        if keyboard::palette_open(ctx) || egui::Popup::is_any_open(ctx) {
            return;
        }
        for command in Command::ALL {
            if let Some(shortcut) = command.shortcut()
                && keyboard::take_key(ctx, shortcut.modifiers, shortcut.logical_key)
            {
                self.execute_command(ctx, command);
                break;
            }
        }
    }
    pub(crate) fn render_command_palette(&mut self, ctx: &Context) {
        let mut p = palette(ctx);
        if !p.open {
            return;
        }
        let mut entries: Vec<Entry> = keyboard::current_actions(ctx)
            .into_iter()
            .map(Entry::Form)
            .collect();
        entries.extend(Command::ALL.into_iter().map(Entry::Workspace));
        let reason = |entry: &Entry| match entry {
            Entry::Workspace(c) => self.disabled_reason(ctx, *c),
            Entry::Form(a) => (!a.enabled).then(|| t("keyboard.form_invalid")),
        };
        let mut chosen = None;
        let escape = keyboard::take_key(ctx, Modifiers::NONE, Key::Escape);
        let enter = keyboard::take_key(ctx, Modifiers::NONE, Key::Enter);
        let up = keyboard::take_key(ctx, Modifiers::NONE, Key::ArrowUp);
        let down = keyboard::take_key(ctx, Modifiers::NONE, Key::ArrowDown);
        let response = egui::Modal::new(Id::new("command_palette")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.0).clamp(280.0, 540.0));
            ui.horizontal(|ui| {
                ui.weak(t("keyboard.commands"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("Esc");
                });
            });
            let search = ui.add(
                egui::TextEdit::singleline(&mut p.query)
                    .id(Id::new("command_palette.query"))
                    .hint_text(t("keyboard.search_commands"))
                    .desired_width(f32::INFINITY)
                    .return_key(None),
            );
            if p.just_opened {
                search.request_focus();
            }
            let results: Vec<&Entry> = entries
                .iter()
                .filter(|entry| entry.matches(&p.query))
                .collect();
            let available: Vec<usize> = results
                .iter()
                .enumerate()
                .filter(|(_, entry)| reason(entry).is_none())
                .map(|(i, _)| i)
                .collect();
            if p.just_opened || search.changed() || !available.contains(&p.selected) {
                p.selected = available.first().copied().unwrap_or(0);
            }
            if !available.is_empty() && (up || down) {
                let index = available.iter().position(|i| *i == p.selected).unwrap_or(0);
                p.selected = available
                    [(index + if down { 1 } else { available.len() - 1 }) % available.len()];
            }
            ui.add_space(6.0);
            ui.spacing_mut().scroll.floating = false;
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 150.0).clamp(180.0, 420.0))
                .show(ui, |ui| {
                    if results.is_empty() {
                        ui.add_space(12.0);
                        ui.weak(t("keyboard.no_commands"));
                    }
                    for (i, entry) in results.iter().enumerate() {
                        let disabled = reason(entry);
                        let text = if let Some(reason) = disabled {
                            format!("{}\n{reason}", entry.label())
                        } else {
                            entry.label().to_owned()
                        };
                        let hint = entry
                            .shortcut()
                            .map(|s| ctx.format_shortcut(&s))
                            .unwrap_or_default();
                        let row = ui.add_enabled(
                            disabled.is_none(),
                            egui::Button::new(text)
                                .selected(i == p.selected && disabled.is_none())
                                .shortcut_text(hint)
                                .min_size(egui::vec2(ui.available_width(), 32.0))
                                .frame(i == p.selected && disabled.is_none()),
                        );
                        if (up || down || search.changed()) && i == p.selected {
                            row.scroll_to_me(Some(egui::Align::Center));
                        }
                        if row.clicked() || (enter && i == p.selected && disabled.is_none()) {
                            chosen = Some((*entry).clone());
                        }
                    }
                });
            ui.add_space(6.0);
            ui.weak(t("keyboard.palette_help"));
        });
        if escape || response.backdrop_response.clicked() || chosen.is_some() {
            p.open = false;
            keyboard::set_palette_open(ctx, false);
            if let Some(id) = p.restore {
                keyboard::restore_focus(ctx, id);
            }
            if let Some(entry) = chosen {
                match entry {
                    Entry::Workspace(command) => self.execute_command(ctx, command),
                    Entry::Form(action) => keyboard::queue_action(ctx, &action),
                }
            }
        }
        p.just_opened = false;
        save_palette(ctx, p);
    }
}

#[cfg(test)]
mod tests {
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
    fn tab_shortcuts_stay_in_the_focused_split_and_keep_welcome() {
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
        app.execute_command(&ctx, Command::Next);
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
}
