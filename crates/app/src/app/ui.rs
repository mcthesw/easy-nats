use eframe::egui;
use egui_dock::DockArea;

use crate::tabs::{AppTabViewer, TabAction, TabKind};

use super::{editors::ConsumerCreateEditor, model::EasyNatsApp, sidebar, windows};

impl eframe::App for EasyNatsApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        windows::render_windows(self, ui);
        sidebar::render_sidebar(self, ui);

        let mut tab_actions = Vec::new();
        let prev_style = ui.ctx().global_style();
        let mut dock_egui_style = (*prev_style).clone();
        dock_egui_style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
        ui.ctx().set_global_style(dock_egui_style);

        let mut dock_style = egui_dock::Style::from_egui(ui.style().as_ref());

        // Main
        dock_style.main_surface_border_rounding = egui::CornerRadius::ZERO;
        // Tab bar
        dock_style.tab_bar.bg_fill = ui.visuals().window_fill;
        dock_style.tab_bar.corner_radius = egui::CornerRadius::ZERO;
        // Tabs
        dock_style.tab.active.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.inactive.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.focused.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.hovered.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.active_with_kb_focus.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.inactive_with_kb_focus.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.focused_with_kb_focus.corner_radius = egui::CornerRadius::ZERO;
        dock_style.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;

        DockArea::new(&mut self.dock_state)
            .style(dock_style)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .show_inside(
                ui,
                &mut AppTabViewer {
                    backend: &self.backend,
                    actions: &mut tab_actions,
                    settings: &mut self.settings,
                    dark_mode: &mut self.dark_mode,
                    log_buffer: &self.log_buffer,
                    proto_manager: &self.proto_manager,
                    tab_id_alloc: &mut self.tab_id_alloc,
                },
            );

        ui.ctx().set_global_style(prev_style);

        // Auto-show Welcome tab when all tabs are closed
        if self.dock_state.iter_all_tabs().next().is_none() {
            self.open_tab(TabKind::Welcome);
        }

        for action in tab_actions {
            match action {
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
                TabAction::CloseOtherTabs { keep_title } => {
                    self.close_other_tabs(&keep_title);
                }
                TabAction::CloseAllTabs => {
                    self.close_all_tabs();
                }
                TabAction::CloseTabsToRight { of_title } => {
                    self.close_tabs_to_right(&of_title);
                }
                TabAction::OpenConnectionEditor => {
                    self.editor.visible = true;
                }
                TabAction::ApplyTheme { dark } => {
                    crate::apply_theme(ui.ctx(), dark);
                }
                TabAction::LoadProtoSchemas { dir } => {
                    self.proto_manager
                        .set_schema_dir(std::path::PathBuf::from(dir));
                }
                TabAction::ClearProtoSchemas => {
                    self.proto_manager.clear();
                }
            }
        }

        self.toasts.show(ui.ctx());
    }
}
