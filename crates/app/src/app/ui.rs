use eframe::egui;
use egui_dock::DockArea;

use crate::tabs::{AppTabViewer, TabAction, TabKind};

use super::{
    editors::{
        ConsumerCreateEditor, ConsumerEditEditor, KvBucketEditEditor, KvEntryCreateEditor,
        StreamPublishEditor,
    },
    model::EasyNatsApp,
    sidebar, windows,
};

impl eframe::App for EasyNatsApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);
        if let Some(delay) = self.backend.next_wakeup() {
            ctx.request_repaint_after(delay);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        windows::render_windows(self, ui);
        sidebar::render_sidebar(self, ui);

        let search_sources = self.search_source_summaries();
        self.prepare_search_workspace_results(&search_sources);
        let connections: Vec<(u64, String)> = self
            .config
            .connections
            .iter()
            .map(|connection| (connection.id, connection.name.clone()))
            .collect();
        let mut tab_actions = Vec::new();
        let prev_style = ui.ctx().global_style();
        let mut dock_egui_style = (*prev_style).clone();
        dock_egui_style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
        ui.ctx().set_global_style(dock_egui_style);

        let mut dock_style = egui_dock::Style::from_egui(ui.style().as_ref());

        // Flat tab chrome: the tab bar matches the content background, with no
        // bar/content hairline, no per-leaf body border, and no per-tab outline boxes.
        // Tabs are frameless pills separated by a small gap and distinguished by fill
        // (inactive darker, active merging into content) plus text color — so the only
        // border at the window edge is the OS frame, and at the sidebar the sidebar
        // separator, instead of a doubled edge line.
        dock_style.main_surface_border_rounding = egui::CornerRadius::ZERO;
        dock_style.tab_bar.bg_fill = ui.visuals().window_fill;
        dock_style.tab_bar.hline_color = egui::Color32::TRANSPARENT;
        dock_style.tab_bar.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.tab_body.stroke = egui::Stroke::NONE;
        dock_style.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.spacing = 4.0;
        // Flatten every tab interaction state: no outline box, no rounding.
        let flatten_tab = |tab: &mut egui_dock::TabInteractionStyle| {
            tab.outline_color = egui::Color32::TRANSPARENT;
            tab.corner_radius = egui::CornerRadius::ZERO;
        };
        flatten_tab(&mut dock_style.tab.active);
        flatten_tab(&mut dock_style.tab.inactive);
        flatten_tab(&mut dock_style.tab.focused);
        flatten_tab(&mut dock_style.tab.hovered);
        flatten_tab(&mut dock_style.tab.active_with_kb_focus);
        flatten_tab(&mut dock_style.tab.inactive_with_kb_focus);
        flatten_tab(&mut dock_style.tab.focused_with_kb_focus);

        // Darken inactive pills so the lighter active tab (which matches the content
        // background) stands out more clearly. Uses extreme_bg_color (already the
        // TextEdit background) rather than a foreign chrome color.
        let inactive_fill = ui.visuals().extreme_bg_color;
        dock_style.tab.inactive.bg_fill = inactive_fill;
        dock_style.tab.inactive_with_kb_focus.bg_fill = inactive_fill;

        DockArea::new(&mut self.dock_state)
            .style(dock_style)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .show_inside(
                ui,
                &mut AppTabViewer {
                    backend: &self.backend,
                    actions: &mut tab_actions,
                    search_sources: &search_sources,
                    settings: &mut self.settings,
                    theme_id: &mut self.theme_id,
                    log_buffer: &self.log_buffer,
                    schema_manager: &self.schema_manager,
                    connections: &connections,
                    runtime_mode: self.runtime_mode,
                },
            );

        ui.ctx().set_global_style(prev_style);

        // Auto-show Welcome tab when all tabs are closed
        if self.dock_state.iter_all_tabs().next().is_none() {
            self.open_tab(TabKind::Welcome);
        }

        for action in tab_actions {
            match action {
                TabAction::OpenStreamPublish {
                    connection_id,
                    stream_name,
                    subject,
                } => {
                    self.stream_publish_editor =
                        StreamPublishEditor::for_stream(connection_id, stream_name, subject);
                }
                TabAction::OpenConsumerCreate {
                    connection_id,
                    stream_name,
                } => {
                    self.consumer_editor = ConsumerCreateEditor {
                        visible: true,
                        connection_id,
                        stream_name,
                        ..Default::default()
                    };
                }
                TabAction::OpenConsumerEdit {
                    connection_id,
                    stream_name,
                    consumer_info,
                } => {
                    self.consumer_edit_editor =
                        ConsumerEditEditor::from_info(connection_id, stream_name, &consumer_info);
                }
                TabAction::OpenKvBucketEdit {
                    connection_id,
                    bucket_info,
                } => {
                    self.kv_bucket_edit_editor =
                        KvBucketEditEditor::from_info(connection_id, &bucket_info);
                }
                TabAction::OpenKvEntryCreate {
                    connection_id,
                    bucket_name,
                    initial_key,
                } => {
                    self.kv_entry_create_editor =
                        KvEntryCreateEditor::for_bucket(connection_id, bucket_name, initial_key);
                }
                TabAction::ConfirmDeleteKvBucket {
                    connection_id,
                    bucket_name,
                } => {
                    self.kv_bucket_delete_confirm = Some((connection_id, bucket_name));
                }
                TabAction::ConfirmDeleteObjStoreBucket {
                    connection_id,
                    bucket_name,
                } => {
                    self.obj_store_bucket_delete_confirm = Some((connection_id, bucket_name));
                }
                TabAction::UploadObject {
                    connection_id,
                    bucket_name,
                } => {
                    self.request_object_upload(connection_id, bucket_name);
                }
                TabAction::DownloadObject {
                    connection_id,
                    bucket_name,
                    object_name,
                } => {
                    self.request_object_download(connection_id, bucket_name, object_name);
                }
                TabAction::CloseOtherTabs { keep_tab_id } => {
                    self.close_other_tabs(keep_tab_id);
                }
                TabAction::CloseAllTabs => {
                    self.close_all_tabs();
                }
                TabAction::CloseTabsToRight { of_tab_id } => {
                    self.close_tabs_to_right(of_tab_id);
                }
                TabAction::OpenConnectionEditor => {
                    self.editor.visible = true;
                }
                TabAction::ApplyTheme { theme_id } => {
                    self.theme_id = theme_id;
                    crate::theme::apply_theme(ui.ctx(), theme_id);
                }
                TabAction::OpenMessageSchemas => {
                    self.open_or_focus_message_schemas();
                }
                TabAction::AddMessageSchemaSource { name, kind, path } => {
                    self.schema_manager.add_source(name, kind, path);
                }
                TabAction::RemoveMessageSchemaSource { source_id } => {
                    self.schema_manager.remove_source(source_id);
                }
                TabAction::ReloadMessageSchemaSource { source_id } => {
                    self.schema_manager.reload_source(source_id);
                }
                TabAction::SetMessageSchemaSourceEnabled { source_id, enabled } => {
                    self.schema_manager.set_source_enabled(source_id, enabled);
                }
                TabAction::AddMessageSchemaBinding {
                    name,
                    connection_id,
                    subject_pattern,
                    source_id,
                    selector,
                    policy,
                } => {
                    if let Err(error) = self.schema_manager.add_binding(
                        name,
                        connection_id,
                        subject_pattern,
                        source_id,
                        selector,
                        policy,
                    ) {
                        self.toasts.push(crate::toast::ToastLevel::Error, error);
                    }
                }
                TabAction::RemoveMessageSchemaBinding { binding_id } => {
                    self.schema_manager.remove_binding(binding_id);
                }
                TabAction::SetMessageSchemaBindingEnabled {
                    binding_id,
                    enabled,
                } => {
                    self.schema_manager.set_binding_enabled(binding_id, enabled);
                }
                TabAction::ScanSearchWorkspaceKvValues { source_id } => {
                    self.scan_search_workspace_kv_values(&source_id);
                }
                TabAction::FetchSearchWorkspaceKvValue { source_id, key } => {
                    self.fetch_search_workspace_kv_value(&source_id, &key);
                }
                TabAction::NavigateSearchResult { locator } => {
                    self.navigate_search_result(locator);
                }
                TabAction::RecordTopic { topic } => {
                    self.settings.record_topic(&topic);
                    self.settings.save();
                }
            }
        }

        self.toasts.show(ui.ctx());
    }
}
