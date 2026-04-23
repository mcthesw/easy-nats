use std::time::SystemTime;

use base64::Engine;
use eframe::egui;
use eframe::egui::{Popup, PopupCloseBehavior, TextEdit};

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

pub(crate) fn topic_history_text_edit(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    selected_suggestion: &mut Option<usize>,
    topic_suggestions: &[&str],
) -> egui::Response {
    let popup_id = ui.id().with(id_salt);
    let lock_tab_focus = !visible_topic_suggestions(topic_suggestions, value.trim()).is_empty();
    let input_resp = ui.add(TextEdit::singleline(value).lock_focus(lock_tab_focus));
    let prefix = value.trim();
    let suggestions = visible_topic_suggestions(topic_suggestions, prefix);

    if suggestions.is_empty() {
        *selected_suggestion = None;
        Popup::close_id(ui.ctx(), popup_id);
        return input_resp;
    }

    let popup_was_open = Popup::is_id_open(ui.ctx(), popup_id);
    if input_resp.has_focus() {
        Popup::open_id(ui.ctx(), popup_id);
    }
    if !input_resp.has_focus() && !popup_was_open {
        *selected_suggestion = None;
        return input_resp;
    }

    if selected_suggestion.is_some_and(|idx| idx >= suggestions.len()) {
        *selected_suggestion = None;
    }

    let cycle_forward = ui.input(|i| {
        i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::Tab) && !i.modifiers.shift
    });
    let cycle_backward = ui.input(|i| {
        i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::Tab) && i.modifiers.shift
    });
    let accept_selection = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let clear_selection = ui.input(|i| i.key_pressed(egui::Key::Escape));

    if cycle_forward {
        *selected_suggestion = Some(cycle_suggestion_index(
            *selected_suggestion,
            suggestions.len(),
            true,
        ));
        ui.memory_mut(|mem| mem.request_focus(input_resp.id));
    } else if cycle_backward {
        *selected_suggestion = Some(cycle_suggestion_index(
            *selected_suggestion,
            suggestions.len(),
            false,
        ));
        ui.memory_mut(|mem| mem.request_focus(input_resp.id));
    } else if clear_selection {
        *selected_suggestion = None;
        Popup::close_id(ui.ctx(), popup_id);
    }

    if accept_selection
        && let Some(idx) = *selected_suggestion
        && let Some(suggestion) = suggestions.get(idx)
    {
        *value = (*suggestion).to_string();
        *selected_suggestion = None;
        Popup::close_id(ui.ctx(), popup_id);
        ui.memory_mut(|mem| mem.request_focus(input_resp.id));
    }

    Popup::from_response(&input_resp)
        .id(popup_id)
        .open_memory(None)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(input_resp.rect.width())
        .show(|ui| {
            ui.set_min_width(input_resp.rect.width());
            for (idx, suggestion) in suggestions.iter().enumerate() {
                let selected = *selected_suggestion == Some(idx);
                let response = ui.selectable_label(selected, *suggestion);
                if response.hovered() {
                    *selected_suggestion = Some(idx);
                }
                if response.clicked() {
                    *value = (*suggestion).to_string();
                    *selected_suggestion = None;
                    Popup::close_id(ui.ctx(), popup_id);
                    ui.memory_mut(|mem| mem.request_focus(input_resp.id));
                }
            }
        });

    input_resp
}

fn visible_topic_suggestions<'a>(topic_suggestions: &'a [&str], prefix: &str) -> Vec<&'a str> {
    topic_suggestions
        .iter()
        .filter(|suggestion| {
            !prefix.is_empty() && suggestion.starts_with(prefix) && **suggestion != prefix
        })
        .copied()
        .take(10)
        .collect()
}

fn cycle_suggestion_index(current: Option<usize>, len: usize, forward: bool) -> usize {
    debug_assert!(len > 0);

    match (current, forward) {
        (None, true) => 0,
        (None, false) => len - 1,
        (Some(idx), true) => (idx + 1) % len,
        (Some(0), false) => len - 1,
        (Some(idx), false) => idx - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{cycle_suggestion_index, visible_topic_suggestions};

    #[test]
    fn visible_topic_suggestions_matches_prefix_and_skips_exact_match() {
        let suggestions = ["foo.bar", "foo.baz", "other"];

        let visible = visible_topic_suggestions(&suggestions, "foo");

        assert_eq!(visible, vec!["foo.bar", "foo.baz"]);
        assert!(visible_topic_suggestions(&suggestions, "foo.bar").is_empty());
        assert!(visible_topic_suggestions(&suggestions, "").is_empty());
    }

    #[test]
    fn cycle_suggestion_index_wraps_in_both_directions() {
        assert_eq!(cycle_suggestion_index(None, 3, true), 0);
        assert_eq!(cycle_suggestion_index(Some(2), 3, true), 0);
        assert_eq!(cycle_suggestion_index(None, 3, false), 2);
        assert_eq!(cycle_suggestion_index(Some(0), 3, false), 2);
        assert_eq!(cycle_suggestion_index(Some(2), 3, false), 1);
    }
}
