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

    fn prune_expired(&mut self) {
        self.items.retain(|t| t.created.elapsed() < self.duration);
    }

    /// Render toasts as overlay in the top-right corner. Call once per frame.
    pub fn show(&mut self, ctx: &egui::Context) {
        self.prune_expired();

        if self.items.is_empty() {
            return;
        }

        // Request repaint while toasts are visible (for auto-dismiss)
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        let mut dismissed: Option<usize> = None;
        let _area_resp = egui::Area::new(egui::Id::new("toasts_area"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0))
            .order(egui::Order::Foreground)
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
                                if ui.small_button(crate::i18n::t("toast.dismiss")).clicked() {
                                    dismissed = Some(i);
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
            });

        if let Some(i) = dismissed {
            self.items.remove(i);
            // Yield input focus back to the main window after dismissal
            // to prevent stale hover/drag state from blocking OS window interactions.
            ctx.memory_mut(|m| m.stop_text_input());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Toast, ToastLevel, Toasts};

    #[test]
    fn prune_discards_expired_toasts() {
        let mut toasts = Toasts {
            items: vec![
                Toast {
                    level: ToastLevel::Info,
                    message: "expired".to_string(),
                    created: std::time::Instant::now() - std::time::Duration::from_secs(5),
                },
                Toast {
                    level: ToastLevel::Success,
                    message: "fresh".to_string(),
                    created: std::time::Instant::now(),
                },
            ],
            duration: std::time::Duration::from_secs(4),
        };

        toasts.prune_expired();

        assert_eq!(toasts.items.len(), 1);
        assert_eq!(toasts.items[0].message, "fresh");
    }
}
