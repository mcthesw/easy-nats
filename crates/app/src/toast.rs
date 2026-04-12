use eframe::egui;

/// Level of a toast message.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ToastLevel {
    Info,
    Success,
    Error,
}

/// A single toast notification.
struct Toast {
    level: ToastLevel,
    message: String,
    created: std::time::Instant,
}

/// Toast notification manager — renders overlay messages.
pub struct Toasts {
    items: Vec<Toast>,
    duration: std::time::Duration,
}

impl Default for Toasts {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            duration: std::time::Duration::from_secs(4),
        }
    }
}

impl Toasts {
    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.items.push(Toast {
            level,
            message: message.into(),
            created: std::time::Instant::now(),
        });
    }

    /// Render toasts as overlay in the top-right corner. Call once per frame.
    pub fn show(&mut self, ctx: &egui::Context) {
        // Remove expired toasts
        self.items.retain(|t| t.created.elapsed() < self.duration);

        if self.items.is_empty() {
            return;
        }

        // Request repaint while toasts are visible (for auto-dismiss)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let mut dismissed: Option<usize> = None;
        egui::Area::new(egui::Id::new("toasts_area"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0))
            .show(ctx, |ui| {
                for (i, toast) in self.items.iter().enumerate() {
                    let (bg, text_color) = match toast.level {
                        ToastLevel::Info => {
                            (egui::Color32::from_rgb(50, 50, 60), egui::Color32::WHITE)
                        }
                        ToastLevel::Success => {
                            (egui::Color32::from_rgb(30, 80, 40), egui::Color32::WHITE)
                        }
                        ToastLevel::Error => {
                            (egui::Color32::from_rgb(120, 30, 30), egui::Color32::WHITE)
                        }
                    };

                    egui::Frame::new()
                        .fill(bg)
                        .corner_radius(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(text_color, &toast.message);
                                if ui
                                    .colored_label(text_color, crate::ui_strings::TOAST_DISMISS)
                                    .on_hover_text("Dismiss")
                                    .clicked()
                                {
                                    dismissed = Some(i);
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
            });

        if let Some(i) = dismissed {
            self.items.remove(i);
        }
    }
}
