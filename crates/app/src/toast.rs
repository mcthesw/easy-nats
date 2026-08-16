use eframe::egui;

/// Level of a toast message.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ToastLevel {
    Info,
    Success,
    Error,
}

impl ToastLevel {
    /// How long a toast of this level stays visible. Errors linger longer
    /// so the failure reason can actually be read.
    fn duration(self) -> std::time::Duration {
        match self {
            ToastLevel::Info | ToastLevel::Success => std::time::Duration::from_secs(4),
            ToastLevel::Error => std::time::Duration::from_secs(10),
        }
    }
}

/// A single toast notification.
struct Toast {
    level: ToastLevel,
    message: String,
    created: web_time::Instant,
}

/// Toast notification manager — renders overlay messages.
#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.items.push(Toast {
            level,
            message: message.into(),
            created: web_time::Instant::now(),
        });
    }

    fn prune_expired(&mut self) {
        self.items
            .retain(|t| t.created.elapsed() < t.level.duration());
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

    fn toast(level: ToastLevel, message: &str, age_secs: u64) -> Toast {
        Toast {
            level,
            message: message.to_string(),
            created: web_time::Instant::now() - std::time::Duration::from_secs(age_secs),
        }
    }

    #[test]
    fn prune_discards_expired_toasts() {
        let mut toasts = Toasts {
            items: vec![
                toast(ToastLevel::Info, "expired", 5),
                toast(ToastLevel::Success, "fresh", 0),
            ],
        };

        toasts.prune_expired();

        assert_eq!(toasts.items.len(), 1);
        assert_eq!(toasts.items[0].message, "fresh");
    }

    #[test]
    fn error_toasts_outlive_info_toasts() {
        let mut toasts = Toasts {
            items: vec![
                toast(ToastLevel::Error, "error", 5),
                toast(ToastLevel::Info, "info", 5),
            ],
        };

        toasts.prune_expired();

        assert_eq!(toasts.items.len(), 1);
        assert_eq!(toasts.items[0].message, "error");
    }

    #[test]
    fn error_toasts_expire_after_their_longer_duration() {
        let mut toasts = Toasts {
            items: vec![toast(ToastLevel::Error, "error", 11)],
        };

        toasts.prune_expired();

        assert!(toasts.items.is_empty());
    }
}
