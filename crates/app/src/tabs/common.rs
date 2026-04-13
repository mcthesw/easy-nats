use std::time::SystemTime;

use base64::Engine;
use eframe::egui;

use crate::i18n::t;

use super::types::AutoRefresh;

/// Draws a horizontal draggable separator. Returns the vertical delta to apply to split_ratio.
pub(crate) fn draggable_separator(ui: &mut egui::Ui, id_salt: &str) -> f32 {
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_width, 6.0), egui::Sense::hover());
    let id = ui.make_persistent_id(id_salt);
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    let color = if response.dragged() || response.hovered() {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke.color
    };
    ui.painter().line_segment(
        [
            rect.center_top() + egui::vec2(0.0, 3.0),
            rect.center_top() + egui::vec2(available_width - 8.0, 3.0),
        ],
        egui::Stroke::new(1.0, color),
    );
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if response.dragged() {
        response.drag_delta().y
    } else {
        0.0
    }
}

pub(crate) fn format_timestamp(ts: SystemTime) -> String {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let total_secs = d.as_secs();
            let hours = (total_secs / 3600) % 24;
            let minutes = (total_secs / 60) % 60;
            let seconds = total_secs % 60;
            let millis = d.subsec_millis();
            format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
        }
        Err(_) => "??:??:??".to_string(),
    }
}

pub(crate) fn payload_preview(payload: &[u8], max_len: usize) -> String {
    let s = String::from_utf8_lossy(payload);
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub(crate) fn decode_base64_payload(value_base64: Option<&str>) -> Vec<u8> {
    value_base64
        .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
        .unwrap_or_default()
}

pub(crate) fn kv_empty_preview(payload: &[u8], max_len: usize) -> String {
    let preview = payload_preview(payload, max_len);
    if preview.is_empty() {
        t("kv.empty_value").to_string()
    } else {
        preview
    }
}

/// Render auto-refresh toggle + interval selector inline.
pub(crate) fn auto_refresh_ui(ui: &mut egui::Ui, id_salt: &str, ar: &mut AutoRefresh) {
    ui.checkbox(&mut ar.enabled, t("common.auto_refresh"));
    if ar.enabled {
        egui::ComboBox::from_id_salt(id_salt)
            .width(55.0)
            .selected_text(format!("{}s", ar.interval_secs))
            .show_ui(ui, |ui| {
                for &secs in AutoRefresh::INTERVALS {
                    ui.selectable_value(&mut ar.interval_secs, secs, format!("{secs}s"));
                }
            });
        let elapsed = ar.last_refresh.elapsed().as_secs();
        ui.weak(format!("{elapsed}s ago"));
    }
}
