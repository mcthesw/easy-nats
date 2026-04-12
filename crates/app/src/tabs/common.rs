use std::time::SystemTime;

use base64::Engine;

use crate::ui_strings as S;

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
        S::KV_EMPTY_VALUE.to_string()
    } else {
        preview
    }
}
