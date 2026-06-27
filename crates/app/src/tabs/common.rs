use std::time::SystemTime;

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

pub(crate) const SEARCH_RESULT_LIMIT: usize = 200;
pub(crate) const KV_VALUE_SEARCH_BATCH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchStatus {
    Inactive,
    Showing { matches: usize, capped: bool },
}

impl SearchStatus {
    pub(crate) fn text(self) -> Option<String> {
        match self {
            Self::Inactive => None,
            Self::Showing { matches, capped } if capped => {
                Some(format!("{} {matches}+", t("common.search_matches")))
            }
            Self::Showing { matches, .. } => {
                Some(format!("{} {matches}", t("common.search_matches")))
            }
        }
    }
}

pub(crate) fn render_search_row(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    search: &mut super::types::ScopedSearchState,
    placeholder: &str,
    primary_label: &str,
    secondary_label: &str,
) -> bool {
    let before = (search.query.clone(), search.primary, search.secondary);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut search.query)
                .id_salt(id_salt)
                .hint_text(placeholder)
                .desired_width((ui.available_width() - 84.0).clamp(96.0, 260.0)),
        );
        egui::ComboBox::from_id_salt(ui.id().with("search_fields"))
            .width(76.0)
            .selected_text(t("common.search_fields"))
            .show_ui(ui, |ui| {
                ui.checkbox(&mut search.primary, primary_label);
                ui.checkbox(&mut search.secondary, secondary_label);
            });
    });
    search.query != before.0 || search.primary != before.1 || search.secondary != before.2
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSearchQuery {
    query: String,
    is_ascii: bool,
}

impl NormalizedSearchQuery {
    pub(crate) fn new(query: &str) -> Option<Self> {
        let query = query.trim();
        if query.is_empty() {
            return None;
        }
        Some(Self::from_normalized(query.to_lowercase()))
    }

    pub(crate) fn from_normalized(query: String) -> Self {
        let is_ascii = query.is_ascii();
        Self { query, is_ascii }
    }

    pub(crate) fn matches(&self, value: &str) -> bool {
        if self.is_ascii && value.is_ascii() {
            contains_ascii_case_insensitive(value.as_bytes(), self.query.as_bytes())
        } else {
            value.to_lowercase().contains(&self.query)
        }
    }

    pub(crate) fn matches_scoped(
        &self,
        primary_enabled: bool,
        primary: &str,
        secondary_enabled: bool,
        secondary_matches: impl FnOnce(&Self) -> bool,
    ) -> bool {
        if primary_enabled && self.matches(primary) {
            return true;
        }
        secondary_enabled && secondary_matches(self)
    }
}

#[cfg(test)]
pub(crate) fn matches_query(value: &str, query: &str) -> bool {
    NormalizedSearchQuery::new(query).is_some_and(|query| query.matches(value))
}

fn contains_ascii_case_insensitive(value: &[u8], query: &[u8]) -> bool {
    if query.is_empty() {
        return true;
    }
    if query.len() > value.len() {
        return false;
    }

    value.windows(query.len()).any(|window| {
        window
            .iter()
            .zip(query.iter())
            .all(|(value, query)| value.to_ascii_lowercase() == *query)
    })
}

pub(crate) fn searchable_payload_text(payload: &[u8]) -> String {
    String::from_utf8_lossy(payload).into_owned()
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
    id_salt: impl egui::AsIdSalt,
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
    use std::cell::Cell;

    use super::{
        NormalizedSearchQuery, cycle_suggestion_index, matches_query, searchable_payload_text,
        visible_topic_suggestions,
    };

    #[test]
    fn visible_topic_suggestions_matches_prefix_and_skips_exact_match() {
        let suggestions = ["foo.bar", "foo.baz", "other"];

        let visible = visible_topic_suggestions(&suggestions, "foo");

        assert_eq!(visible, vec!["foo.bar", "foo.baz"]);
        assert!(visible_topic_suggestions(&suggestions, "foo.bar").is_empty());
        assert!(visible_topic_suggestions(&suggestions, "").is_empty());
    }

    #[test]
    fn search_helpers_match_case_insensitively() {
        assert!(matches_query("Balance: 42", "balance"));
        assert!(matches_query("Balance: 42", "42"));
        assert_eq!(searchable_payload_text(b"hello"), "hello");
    }

    #[test]
    fn normalized_search_query_trims_and_matches_ascii_case_insensitively() {
        let query = NormalizedSearchQuery::new("  BALANCE  ").expect("query is active");

        assert_eq!(
            query,
            NormalizedSearchQuery::from_normalized("balance".to_string())
        );
        assert!(query.matches("Account Balance: 42"));
        assert!(!query.matches("Account total: 42"));
    }

    #[test]
    fn normalized_search_query_preserves_unicode_case_matching() {
        let query = NormalizedSearchQuery::new("éclair").expect("query is active");

        assert!(query.matches("ÉCLAIR"));
    }

    #[test]
    fn normalized_search_query_reports_inactive_empty_queries() {
        assert!(NormalizedSearchQuery::new("   ").is_none());
    }

    #[test]
    fn scoped_match_short_circuits_secondary_field_when_primary_matches() {
        let query = NormalizedSearchQuery::new("orders").expect("query is active");
        let secondary_called = Cell::new(false);

        assert!(query.matches_scoped(true, "orders.created", true, |_| {
            secondary_called.set(true);
            false
        }));
        assert!(!secondary_called.get());
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
