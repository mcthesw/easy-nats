use eframe::egui;
use tracing::Level;

use crate::i18n::t;
use crate::log_layer::SharedLogBuffer;

/// Render the in-app log viewer tab.
pub fn log_viewer_ui(ui: &mut egui::Ui, log_buffer: &SharedLogBuffer) {
    static LEVELS: [Level; 5] = [
        Level::ERROR,
        Level::WARN,
        Level::INFO,
        Level::DEBUG,
        Level::TRACE,
    ];
    static LEVEL_LABELS: [&str; 5] = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

    // Store filter in egui memory
    let filter_id = ui.make_persistent_id("log_viewer_level_filter");
    let mut filter_idx = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<usize>(filter_id))
        .unwrap_or(2); // default INFO
    ui.horizontal(|ui| {
        ui.heading(t("log_viewer.title"));
        ui.label(" ");
        ui.label(t("log_viewer.level_filter"));
        egui::ComboBox::from_id_salt("log_level_filter")
            .selected_text(LEVEL_LABELS[filter_idx])
            .show_ui(ui, |ui| {
                for (i, label) in LEVEL_LABELS.iter().enumerate() {
                    ui.selectable_value(&mut filter_idx, i, *label);
                }
            });
    });
    ui.ctx().data_mut(|d| d.insert_persisted(filter_id, filter_idx));
    ui.separator();

    let min_level = LEVELS[filter_idx];
    let entries = {
        log_buffer
            .lock()
            .map(|buf| {
                buf.iter()
                    .rev()
                    .filter(|e| is_visible_at_level(e.level, min_level))
                    .take(400)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    if entries.is_empty() {
        ui.label(t("log_viewer.empty"));
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(true)
        .id_salt("log_viewer_scroll")
        .show(ui, |ui| {
            for entry in entries.iter().rev() {
                let color = level_color(entry.level, ui);
                let level_str = match entry.level {
                    Level::ERROR => "ERROR",
                    Level::WARN => " WARN",
                    Level::INFO => " INFO",
                    Level::DEBUG => "DEBUG",
                    Level::TRACE => "TRACE",
                };
                let ts = format_timestamp(entry.timestamp);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(color, format!("[{ts}] {level_str}"));
                    ui.weak(&entry.target);
                    ui.label(&entry.message);
                });
            }
        });
}

fn level_color(level: Level, ui: &egui::Ui) -> egui::Color32 {
    match level {
        Level::ERROR => egui::Color32::from_rgb(230, 60, 60),
        Level::WARN => egui::Color32::from_rgb(220, 170, 50),
        Level::INFO => ui.visuals().text_color(),
        Level::DEBUG => egui::Color32::from_rgb(100, 160, 230),
        Level::TRACE => ui.visuals().weak_text_color(),
    }
}

fn format_timestamp(ts: std::time::SystemTime) -> String {
    match ts.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let h = (secs / 3600) % 24;
            let m = (secs / 60) % 60;
            let s = secs % 60;
            let ms = d.subsec_millis();
            format!("{h:02}:{m:02}:{s:02}.{ms:03}")
        }
        Err(_) => "??:??:??.???".to_string(),
    }
}

fn is_visible_at_level(level: Level, min_level: Level) -> bool {
    fn rank(level: Level) -> u8 {
        match level {
            Level::ERROR => 5,
            Level::WARN => 4,
            Level::INFO => 3,
            Level::DEBUG => 2,
            Level::TRACE => 1,
        }
    }

    rank(level) >= rank(min_level)
}
