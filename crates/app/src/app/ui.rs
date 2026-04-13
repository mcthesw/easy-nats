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
        DockArea::new(&mut self.dock_state)
            .style(egui_dock::Style::from_egui(ui.style().as_ref()))
            .show_inside(
                ui,
                &mut AppTabViewer {
                    backend: &self.backend,
                    actions: &mut tab_actions,
                    settings: &mut self.settings,
                    dark_mode: &mut self.dark_mode,
                    log_buffer: &self.log_buffer,
                },
            );

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
            }
        }

        self.toasts.show(ui.ctx());
    }
}
