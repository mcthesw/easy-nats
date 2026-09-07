use eframe::egui::{self, Context, Id, Key, Modifiers};
use nats_backend::ConnectionStatusKind;

use super::model::EasyNatsApp;
use crate::{i18n::t, keyboard, tabs::TabKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    SwitchTab,
    Search,
    Connection,
    Publisher,
    Subscriber,
    Settings,
    Logs,
    Schemas,
    Close,
}
impl Command {
    const ALL: [Self; 9] = [
        Self::SwitchTab,
        Self::Search,
        Self::Connection,
        Self::Publisher,
        Self::Subscriber,
        Self::Settings,
        Self::Logs,
        Self::Schemas,
        Self::Close,
    ];
    fn label(self) -> &'static str {
        t(match self {
            Self::SwitchTab => "keyboard.switch_tab",
            Self::Search => "search_workspace.title",
            Self::Connection => "sidebar.connection_new",
            Self::Publisher => "keyboard.publisher",
            Self::Subscriber => "keyboard.subscriber",
            Self::Settings => "settings.title",
            Self::Logs => "log_viewer.title",
            Self::Schemas => "message_schema.title",
            Self::Close => "keyboard.close_tab",
        })
    }
    fn keywords(self) -> &'static str {
        match self {
            Self::SwitchTab => "switch tabs recent 切换 标签 最近",
            Self::Search => "search workspace 搜索 工作区",
            Self::Connection => "new connection 新建连接",
            Self::Publisher => "publisher publish 发布",
            Self::Subscriber => "subscriber subscribe 订阅",
            Self::Settings => "settings preferences 设置",
            Self::Logs => "logs 日志",
            Self::Schemas => "message schemas 消息模式",
            Self::Close => "close tab 关闭标签",
        }
    }
    fn shortcut(self) -> Option<egui::KeyboardShortcut> {
        Some(match self {
            Self::SwitchTab => egui::KeyboardShortcut::new(Modifiers::CTRL, Key::Tab),
            Self::Search => keyboard::shortcut(Key::F, true),
            Self::Connection => keyboard::shortcut(Key::N, false),
            Self::Close => keyboard::shortcut(Key::W, false),
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
    OpenConnection { id: u64, label: String },
}
impl Entry {
    fn group(&self) -> u8 {
        match self {
            Self::Form(_) => 0,
            Self::OpenConnection { .. } => 3,
            Self::Workspace(
                Command::Connection | Command::Settings | Command::Logs | Command::Schemas,
            ) => 2,
            Self::Workspace(_) => 1,
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::Workspace(c) => c.label(),
            Self::Form(a) => &a.label,
            Self::OpenConnection { label, .. } => label,
        }
    }
    fn shortcut(&self) -> Option<egui::KeyboardShortcut> {
        match self {
            Self::Workspace(c) => c.shortcut(),
            Self::Form(a) => Some(keyboard::shortcut(Key::Enter, a.secondary)),
            Self::OpenConnection { .. } => None,
        }
    }
    fn matches(&self, query: &str) -> bool {
        let keywords = match self {
            Self::Workspace(c) => c.keywords(),
            Self::OpenConnection { .. } => "open connect connection 打开 连接",
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
    fn connection_commands(&self) -> impl Iterator<Item = Entry> + '_ {
        self.config
            .connections
            .iter()
            .map(|connection| Entry::OpenConnection {
                id: connection.id,
                label: format!("{}: {}", t("keyboard.open_connection"), connection.name),
            })
    }

    fn open_connection_disabled_reason(&self, ctx: &Context, id: u64) -> Option<&'static str> {
        if keyboard::window_active(ctx) {
            Some(t("keyboard.finish_window"))
        } else if !self
            .config
            .connections
            .iter()
            .any(|connection| connection.id == id)
        {
            Some(t("keyboard.connection_unavailable"))
        } else if self.user_wants_connected.contains(&id)
            || matches!(
                self.conn_statuses.get(&id),
                Some(ConnectionStatusKind::Connected | ConnectionStatusKind::Connecting)
            )
        {
            Some(t("keyboard.connection_already_open"))
        } else {
            None
        }
    }

    fn execute_open_connection(&mut self, ctx: &Context, id: u64) {
        if self.open_connection_disabled_reason(ctx, id).is_none() {
            self.selected_conn = Some(id);
            self.connect(id);
        }
    }

    fn command_connection(&self) -> Option<u64> {
        self.active_command_tab()
            .and_then(TabKind::connection_id)
            .or(self.selected_conn)
    }
    pub(super) fn command_leaf(&self) -> Option<egui_dock::NodePath> {
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
    pub(super) fn active_command_tab(&self) -> Option<&TabKind> {
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
            Command::SwitchTab if self.active_command_tab().is_none() => {
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
            Command::SwitchTab => self.open_tab_switcher(ctx, false, false),
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
        }
    }
    pub(crate) fn handle_shortcuts(&mut self, ctx: &Context) {
        self.record_tab_visit(ctx);
        if self.handle_tab_switcher_input(ctx) {
            return;
        }
        if !self.runtime_mode.supports_local_files() || keyboard::ime_blocked(ctx) {
            return;
        }
        if keyboard::take_key(ctx, keyboard::shortcut(Key::P, true).modifiers, Key::P) {
            open_palette(ctx);
        }
        if keyboard::palette_open(ctx) || egui::Popup::is_any_open(ctx) {
            return;
        }
        let next = keyboard::take_key(ctx, Modifiers::CTRL, Key::Tab);
        let previous = keyboard::take_key(ctx, Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
        if next || previous {
            self.open_tab_switcher(ctx, true, previous);
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
        entries.extend(self.connection_commands());
        entries.sort_by_key(Entry::group);
        let reason = |entry: &Entry| match entry {
            Entry::Workspace(c) => self.disabled_reason(ctx, *c),
            Entry::Form(a) => (!a.enabled).then(|| t("keyboard.form_invalid")),
            Entry::OpenConnection { id, .. } => self.open_connection_disabled_reason(ctx, *id),
        };
        let mut chosen = None;
        let escape = keyboard::take_key(ctx, Modifiers::NONE, Key::Escape);
        let enter = keyboard::take_key(ctx, Modifiers::NONE, Key::Enter);
        let up = keyboard::take_key(ctx, Modifiers::NONE, Key::ArrowUp);
        let down = keyboard::take_key(ctx, Modifiers::NONE, Key::ArrowDown);
        // A keyboard launcher should be immediately readable, including its first frame.
        let palette_id = Id::new("command_palette");
        let modal = egui::Modal::new(palette_id).area(
            egui::Modal::default_area(palette_id)
                .fade_in(false)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 40.0)),
        );
        let response = modal.show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.0).clamp(280.0, 540.0));
            ui.visuals_mut().weak_text_alpha = 0.85;
            ui.horizontal(|ui| {
                ui.strong(t("keyboard.commands"));
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
                .max_height((ctx.content_rect().height() - 190.0).clamp(140.0, 320.0))
                .show(ui, |ui| {
                    if results.is_empty() {
                        ui.add_space(12.0);
                        ui.weak(t("keyboard.no_commands"));
                    }
                    let mut last_group = None;
                    for (i, entry) in results.iter().enumerate() {
                        let group = entry.group();
                        if last_group != Some(group) {
                            if last_group.is_some() {
                                ui.add_space(4.0);
                            }
                            ui.label(
                                egui::RichText::new(t(match group {
                                    0 => "keyboard.group_page",
                                    1 => "keyboard.group_navigation",
                                    3 => "sidebar.connections_heading",
                                    _ => "keyboard.group_application",
                                }))
                                .strong(),
                            );
                            last_group = Some(group);
                        }
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
                                .min_size(egui::vec2(ui.available_width(), 28.0))
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
            let switching_tabs = matches!(&chosen, Some(Entry::Workspace(Command::SwitchTab)));
            if let Some(id) = p.restore {
                match &chosen {
                    None | Some(Entry::Form(_)) => keyboard::restore_focus(ctx, id),
                    Some(Entry::Workspace(Command::Connection)) => {
                        // The new editor captures its return focus on the next frame.
                        // A deferred restoration would override its initial field focus.
                        ctx.memory_mut(|m| m.request_focus(id));
                    }
                    Some(Entry::Workspace(_) | Entry::OpenConnection { .. }) => {}
                }
            }
            if let Some(entry) = chosen {
                match entry {
                    Entry::Workspace(command) => {
                        self.execute_command(ctx, command);
                        if switching_tabs {
                            self.inherit_switcher_focus(ctx, p.restore);
                        }
                    }
                    Entry::Form(action) => keyboard::queue_action(ctx, &action),
                    Entry::OpenConnection { id, .. } => self.execute_open_connection(ctx, id),
                }
            }
        }
        p.just_opened = ctx.will_discard();
        save_palette(ctx, p);
    }
}

#[cfg(test)]
mod connection_tests;
#[cfg(test)]
mod tests;
