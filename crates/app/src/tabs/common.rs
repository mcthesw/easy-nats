use std::time::SystemTime;

use base64::Engine;
use eframe::egui;

use crate::i18n::t;

use super::types::AutoRefresh;

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
