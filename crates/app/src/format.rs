use base64::Engine;
use eframe::egui;
use egui::text::LayoutJob;
use std::ops::Range;

use crate::proto::{AutoDetectResult, ProtoViewState};
use crate::schema::{MessageSchemaManager, PayloadSchemaStatus, SchemaStatusLevel};

/// Display format for message payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayloadFormat {
    #[default]
    Auto,
    Text,
    Json,
    Hex,
    Base64,
    Protobuf,
}

impl PayloadFormat {
    pub const ALL: &[PayloadFormat] = &[
        PayloadFormat::Auto,
        PayloadFormat::Text,
        PayloadFormat::Json,
        PayloadFormat::Hex,
        PayloadFormat::Base64,
        PayloadFormat::Protobuf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PayloadFormat::Auto => "Auto",
            PayloadFormat::Text => "Text",
            PayloadFormat::Json => "JSON",
            PayloadFormat::Hex => "Hex",
            PayloadFormat::Base64 => "Base64",
            PayloadFormat::Protobuf => "Protobuf",
        }
    }
}

/// Detects the best display format for `data`.
/// Order: valid JSON → valid UTF-8 text → hex dump.
pub fn detect_format(data: &[u8]) -> PayloadFormat {
    if data.is_empty() {
        return PayloadFormat::Text;
    }
    // Try JSON first
    if serde_json::from_slice::<serde_json::Value>(data).is_ok() {
        return PayloadFormat::Json;
    }
    // Try UTF-8 text
    if std::str::from_utf8(data).is_ok() {
        return PayloadFormat::Text;
    }
    PayloadFormat::Hex
}

/// Resolves `Auto` to a concrete format, or passes through the explicit choice.
pub fn resolve_format(format: PayloadFormat, data: &[u8]) -> PayloadFormat {
    if format == PayloadFormat::Auto {
        detect_format(data)
    } else {
        format
    }
}

// ─── Formatters ───

/// Render payload as plain UTF-8 text (lossy).
pub fn format_text(data: &[u8]) -> String {
    String::from_utf8_lossy(data).into_owned()
}

/// Pretty-print JSON. Returns the formatted string.
pub fn format_json(data: &[u8]) -> String {
    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| format_text(data)),
        Err(_) => format_text(data),
    }
}

/// Build a `LayoutJob` with basic JSON syntax highlighting.
pub fn json_syntax_highlight(json: &str, style: &egui::Style) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mono = egui::FontId::monospace(13.0);
    let base_color = style.visuals.text_color();

    let string_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(152, 195, 121) // green
    } else {
        egui::Color32::from_rgb(80, 161, 79)
    };
    let number_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(209, 154, 102) // orange
    } else {
        egui::Color32::from_rgb(152, 104, 1)
    };
    let keyword_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(86, 182, 194) // cyan
    } else {
        egui::Color32::from_rgb(1, 132, 188)
    };
    let key_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(224, 108, 117) // red
    } else {
        egui::Color32::from_rgb(228, 86, 73)
    };

    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    let mut after_colon = false;

    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '"' => {
                // Scan string
                let start = i;
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if chars[i] == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let color = if after_colon { string_color } else { key_color };
                job.append(&s, 0.0, egui::TextFormat::simple(mono.clone(), color));
                after_colon = false;
            }
            ':' => {
                job.append(":", 0.0, egui::TextFormat::simple(mono.clone(), base_color));
                after_colon = true;
                i += 1;
            }
            ',' | '{' | '}' | '[' | ']' => {
                let s = String::from(ch);
                job.append(&s, 0.0, egui::TextFormat::simple(mono.clone(), base_color));
                after_colon = false;
                i += 1;
            }
            't' | 'f' | 'n' => {
                // true, false, null
                let start = i;
                while i < chars.len() && chars[i].is_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                job.append(
                    &word,
                    0.0,
                    egui::TextFormat::simple(mono.clone(), keyword_color),
                );
                after_colon = false;
            }
            c if c == '-' || c.is_ascii_digit() => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '-'
                        || chars[i] == '+'
                        || chars[i] == 'e'
                        || chars[i] == 'E')
                {
                    i += 1;
                }
                let num: String = chars[start..i].iter().collect();
                job.append(
                    &num,
                    0.0,
                    egui::TextFormat::simple(mono.clone(), number_color),
                );
                after_colon = false;
            }
            _ => {
                // Whitespace or other
                let s = String::from(ch);
                job.append(&s, 0.0, egui::TextFormat::simple(mono.clone(), base_color));
                i += 1;
            }
        }
    }
    job
}

/// Format as hex dump with offset | hex | ASCII columns.
/// Output format per line: `00000000  48 65 6C 6C 6F 20  |Hello |`
pub fn format_hex(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (offset, chunk) in data.chunks(16).enumerate() {
        // Offset
        out.push_str(&format!("{:08X}  ", offset * 16));
        // Hex bytes
        for (i, byte) in chunk.iter().enumerate() {
            out.push_str(&format!("{byte:02X} "));
            if i == 7 {
                out.push(' ');
            }
        }
        // Pad remaining hex columns
        let remaining = 16 - chunk.len();
        for i in 0..remaining {
            out.push_str("   ");
            if chunk.len() + i == 7 {
                out.push(' ');
            }
        }
        // ASCII
        out.push_str(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                out.push(*byte as char);
            } else {
                out.push('.');
            }
        }
        out.push('|');
        out.push('\n');
    }
    out
}

/// Format as Base64 encoded string.
pub fn format_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub const READ_ONLY_PREVIEW_FORMATS: &[PayloadFormat] = &[
    PayloadFormat::Auto,
    PayloadFormat::Text,
    PayloadFormat::Json,
    PayloadFormat::Hex,
    PayloadFormat::Base64,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOnlyPreview {
    pub resolved_format: PayloadFormat,
    pub text: String,
}

pub fn format_read_only_preview(data: &[u8], format: PayloadFormat) -> ReadOnlyPreview {
    let requested = if READ_ONLY_PREVIEW_FORMATS.contains(&format) {
        format
    } else {
        PayloadFormat::Auto
    };
    let resolved = resolve_format(requested, data);
    let text = match resolved {
        PayloadFormat::Json => format_json(data),
        PayloadFormat::Hex => format_hex(data),
        PayloadFormat::Base64 => format_base64(data),
        PayloadFormat::Text | PayloadFormat::Auto => format_text(data),
        PayloadFormat::Protobuf => format_text(data),
    };
    ReadOnlyPreview {
        resolved_format: resolved,
        text,
    }
}

pub fn preview_match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let query = query.trim();
    if text.is_empty() || query.is_empty() {
        return Vec::new();
    }

    if text.is_ascii() && query.is_ascii() {
        return ascii_match_ranges(text, query);
    }

    unicode_match_ranges(text, query)
}

pub fn read_only_preview_layout_job(
    preview: &ReadOnlyPreview,
    style: &egui::Style,
    query: &str,
) -> LayoutJob {
    let ranges = preview_match_ranges(&preview.text, query);
    highlighted_monospace_job(&preview.text, style, &ranges)
}

fn ascii_match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let text_bytes = text.as_bytes();
    let query_bytes = query.as_bytes();
    if query_bytes.len() > text_bytes.len() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut index = 0;
    while index + query_bytes.len() <= text_bytes.len() {
        let window = &text_bytes[index..index + query_bytes.len()];
        let matches = window
            .iter()
            .zip(query_bytes.iter())
            .all(|(value, query)| value.eq_ignore_ascii_case(query));
        if matches {
            ranges.push(index..index + query_bytes.len());
            index += query_bytes.len();
        } else {
            index += 1;
        }
    }
    ranges
}

fn unicode_match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let query_chars = query.chars().count();
    if query_chars == 0 {
        return Vec::new();
    }
    let query_lower = query.to_lowercase();
    let starts = text.char_indices().map(|(idx, _)| idx).collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut cursor = 0;

    while cursor < starts.len() {
        let start = starts[cursor];
        let Some(end) = byte_index_after_chars(text, start, query_chars) else {
            break;
        };
        if text[start..end].to_lowercase() == query_lower {
            ranges.push(start..end);
            cursor += query_chars;
        } else {
            cursor += 1;
        }
    }
    ranges
}

fn byte_index_after_chars(text: &str, start: usize, char_count: usize) -> Option<usize> {
    let mut iter = text[start..].char_indices();
    for _ in 0..char_count {
        iter.next()?;
    }
    iter.next().map(|(idx, _)| start + idx).or(Some(text.len()))
}

fn highlighted_monospace_job(
    text: &str,
    style: &egui::Style,
    ranges: &[Range<usize>],
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mono = egui::FontId::monospace(13.0);
    let text_color = style.visuals.text_color();
    let highlight_bg = style.visuals.selection.bg_fill;
    let highlight_fg = style.visuals.selection.stroke.color;
    let normal = egui::TextFormat::simple(mono.clone(), text_color);
    let highlighted = egui::TextFormat {
        font_id: mono,
        color: highlight_fg,
        background: highlight_bg,
        ..Default::default()
    };

    let mut cursor = 0;
    for range in ranges {
        if range.start > cursor {
            job.append(&text[cursor..range.start], 0.0, normal.clone());
        }
        job.append(&text[range.clone()], 0.0, highlighted.clone());
        cursor = range.end;
    }
    if cursor < text.len() {
        job.append(&text[cursor..], 0.0, normal);
    }
    job
}

/// Show a format selector combo box. Returns true if the format changed.
pub fn format_selector(ui: &mut egui::Ui, id_salt: &str, format: &mut PayloadFormat) -> bool {
    let before = *format;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format.label())
        .show_ui(ui, |ui| {
            for &f in PayloadFormat::ALL {
                ui.selectable_value(format, f, f.label());
            }
        });
    *format != before
}

/// Render formatted payload into the UI with appropriate styling.
pub fn render_payload(ui: &mut egui::Ui, data: &[u8], format: PayloadFormat) {
    let resolved = resolve_format(format, data);
    match resolved {
        PayloadFormat::Json => {
            let pretty = format_json(data);
            let job = json_syntax_highlight(&pretty, ui.style());
            ui.label(job);
        }
        PayloadFormat::Hex => {
            let hex = format_hex(data);
            ui.label(egui::RichText::new(hex).monospace());
        }
        PayloadFormat::Base64 => {
            let b64 = format_base64(data);
            ui.label(egui::RichText::new(b64).monospace());
        }
        PayloadFormat::Protobuf => {
            // Wire-format fallback when called without proto state
            let text = MessageSchemaManager::decode_wire_format(data);
            ui.label(egui::RichText::new(text).monospace());
        }
        _ => {
            // Text (or Auto that resolved to Text)
            let text = format_text(data);
            ui.label(&text);
        }
    }
}

/// Render formatted payload with protobuf schema support.
/// When format is Protobuf and a schema manager is available, shows the rich
/// proto UI with type selector and decoded JSON. Otherwise falls back to basic rendering.
pub fn render_payload_with_proto(
    ui: &mut egui::Ui,
    data: &[u8],
    format: PayloadFormat,
    id_salt: &str,
    proto_state: &mut ProtoViewState,
    manager: &MessageSchemaManager,
) {
    let resolved = resolve_format(format, data);
    if resolved == PayloadFormat::Protobuf {
        render_protobuf_payload(ui, data, id_salt, proto_state, manager);
    } else {
        render_payload(ui, data, format);
    }
}

/// Render formatted payload with subject-bound schema support.
pub struct SchemaRenderContext<'a> {
    pub manager: &'a MessageSchemaManager,
    pub connection_id: u64,
    pub subject: &'a str,
}

pub fn render_payload_with_schema(
    ui: &mut egui::Ui,
    data: &[u8],
    format: PayloadFormat,
    id_salt: &str,
    proto_state: &mut ProtoViewState,
    schema: SchemaRenderContext<'_>,
) {
    if let Some(rendered) =
        schema
            .manager
            .render_schema_payload(schema.connection_id, schema.subject, data)
    {
        render_schema_status(ui, &rendered.status);
        ui.add_space(4.0);
        if format == PayloadFormat::Auto {
            if let Some(json) = rendered.json {
                let job = json_syntax_highlight(&json, ui.style());
                ui.label(job);
            } else {
                render_payload(ui, data, PayloadFormat::Auto);
            }
        } else {
            render_payload_with_proto(ui, data, format, id_salt, proto_state, schema.manager);
        }
    } else {
        render_payload_with_proto(ui, data, format, id_salt, proto_state, schema.manager);
    }
}

pub fn render_schema_status(ui: &mut egui::Ui, status: &PayloadSchemaStatus) {
    let color = match status.level {
        SchemaStatusLevel::Info => ui.visuals().weak_text_color(),
        SchemaStatusLevel::Success => egui::Color32::from_rgb(57, 153, 83),
        SchemaStatusLevel::Warning => ui.visuals().warn_fg_color,
        SchemaStatusLevel::Error => ui.visuals().error_fg_color,
    };
    ui.colored_label(color, format!("{}: {}", status.label, status.message));
}

/// Render protobuf-specific controls and decoded output.
fn render_protobuf_payload(
    ui: &mut egui::Ui,
    data: &[u8],
    id_salt: &str,
    proto_state: &mut ProtoViewState,
    manager: &MessageSchemaManager,
) {
    let message_types = manager.manual_message_types();
    if !message_types.is_empty() {
        render_proto_type_selector(ui, id_salt, proto_state, &message_types);
        ui.add_space(4.0);
        render_proto_decoded(ui, data, proto_state, manager);
    } else {
        let text = MessageSchemaManager::decode_wire_format(data);
        ui.label(egui::RichText::new(text).monospace());
    }
}

fn render_proto_type_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    state: &mut ProtoViewState,
    message_types: &[String],
) {
    // Reset stale selection if schemas changed
    if !state.auto_detect
        && !state.selected_type.is_empty()
        && !message_types.contains(&state.selected_type)
    {
        state.selected_type.clear();
        state.cached_output = None;
    }

    ui.horizontal(|ui| {
        ui.label("Message type:");
        let display = if state.auto_detect {
            "(Auto-detect)".to_string()
        } else if state.selected_type.is_empty() {
            "(Select type)".to_string()
        } else {
            state.selected_type.clone()
        };

        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(&display)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut state.auto_detect, true, "(Auto-detect)")
                    .changed()
                {
                    state.cached_output = None;
                }
                for t in message_types {
                    if ui
                        .selectable_label(!state.auto_detect && state.selected_type == *t, t)
                        .clicked()
                    {
                        state.auto_detect = false;
                        state.selected_type = t.clone();
                        state.cached_output = None;
                    }
                }
            });
    });
}

fn render_proto_decoded(
    ui: &mut egui::Ui,
    data: &[u8],
    state: &mut ProtoViewState,
    manager: &MessageSchemaManager,
) {
    if data.is_empty() {
        ui.label("(empty payload)");
        return;
    }

    if state.auto_detect {
        match manager.auto_detect_manual_proto(data) {
            AutoDetectResult::Match { type_name, json } => {
                ui.label(
                    egui::RichText::new(format!("Detected: {type_name}"))
                        .small()
                        .weak(),
                );
                let job = json_syntax_highlight(&json, ui.style());
                ui.label(job);
            }
            AutoDetectResult::Ambiguous(types) => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!(
                        "Ambiguous — {} types matched. Select one manually.",
                        types.len()
                    ),
                );
                let text = MessageSchemaManager::decode_wire_format(data);
                ui.label(egui::RichText::new(text).monospace());
            }
            AutoDetectResult::NoMatch => {
                ui.label(egui::RichText::new("No matching message type found").weak());
                let text = MessageSchemaManager::decode_wire_format(data);
                ui.label(egui::RichText::new(text).monospace());
            }
        }
    } else if !state.selected_type.is_empty() {
        match manager.decode_manual_proto(data, &state.selected_type) {
            Ok(json) => {
                let job = json_syntax_highlight(&json, ui.style());
                ui.label(job);
            }
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, format!("Decode error: {e}"));
                let text = MessageSchemaManager::decode_wire_format(data);
                ui.label(egui::RichText::new(text).monospace());
            }
        }
    } else {
        ui.label(egui::RichText::new("Select a message type above").weak());
        let text = MessageSchemaManager::decode_wire_format(data);
        ui.label(egui::RichText::new(text).monospace());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_json() {
        assert_eq!(detect_format(b"{\"key\": \"value\"}"), PayloadFormat::Json);
        assert_eq!(detect_format(b"[1, 2, 3]"), PayloadFormat::Json);
    }

    #[test]
    fn detect_text() {
        assert_eq!(detect_format(b"hello world"), PayloadFormat::Text);
        assert_eq!(detect_format(b""), PayloadFormat::Text);
    }

    #[test]
    fn detect_hex_for_binary() {
        assert_eq!(detect_format(&[0xFF, 0xFE, 0x00, 0x01]), PayloadFormat::Hex);
    }

    #[test]
    fn json_pretty_print() {
        let data = b"{\"a\":1,\"b\":\"hello\"}";
        let result = format_json(data);
        assert!(result.contains("  \"a\": 1"));
        assert!(result.contains("  \"b\": \"hello\""));
    }

    #[test]
    fn hex_dump_output() {
        let data = b"Hello";
        let hex = format_hex(data);
        assert!(hex.contains("00000000"));
        assert!(hex.contains("48 65 6C 6C 6F"));
        assert!(hex.contains("|Hello|"));
    }

    #[test]
    fn hex_dump_multiline() {
        let data: Vec<u8> = (0..20).collect();
        let hex = format_hex(&data);
        assert!(hex.contains("00000000"));
        assert!(hex.contains("00000010"));
    }

    #[test]
    fn base64_output() {
        assert_eq!(format_base64(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn resolve_auto_to_json() {
        assert_eq!(
            resolve_format(PayloadFormat::Auto, b"{\"a\":1}"),
            PayloadFormat::Json
        );
    }

    #[test]
    fn resolve_explicit_overrides() {
        assert_eq!(
            resolve_format(PayloadFormat::Hex, b"hello"),
            PayloadFormat::Hex
        );
    }

    #[test]
    fn read_only_preview_pretty_prints_json() {
        let preview = format_read_only_preview(br#"{"a":1,"b":"hello"}"#, PayloadFormat::Auto);

        assert_eq!(preview.resolved_format, PayloadFormat::Json);
        assert!(preview.text.contains("  \"a\": 1"));
        assert!(preview.text.contains("  \"b\": \"hello\""));
    }

    #[test]
    fn read_only_preview_formats_text_and_binary_auto_fallback() {
        let text = format_read_only_preview(b"hello", PayloadFormat::Auto);
        assert_eq!(text.resolved_format, PayloadFormat::Text);
        assert_eq!(text.text, "hello");

        let binary = format_read_only_preview(&[0xFF, 0xFE, 0x00], PayloadFormat::Auto);
        assert_eq!(binary.resolved_format, PayloadFormat::Hex);
        assert!(binary.text.contains("FF FE 00"));
    }

    #[test]
    fn read_only_preview_falls_back_for_unsupported_format() {
        let preview = format_read_only_preview(br#"{"a":1}"#, PayloadFormat::Protobuf);

        assert_eq!(preview.resolved_format, PayloadFormat::Json);
        assert!(preview.text.contains("\"a\": 1"));
    }

    #[test]
    fn preview_match_ranges_find_ascii_matches_case_insensitively() {
        let ranges = preview_match_ranges("Balance balance BALANCE", "balance");

        assert_eq!(ranges, vec![0..7, 8..15, 16..23]);
    }

    #[test]
    fn preview_match_ranges_find_matches_after_json_formatting() {
        let preview =
            format_read_only_preview(br#"{"event_id":"abc","kind":"path"}"#, PayloadFormat::Json);
        let ranges = preview_match_ranges(&preview.text, "kind");

        assert_eq!(ranges.len(), 1);
        assert_eq!(&preview.text[ranges[0].clone()], "kind");
    }

    #[test]
    fn preview_match_ranges_omit_absent_transformed_output() {
        let preview = format_read_only_preview(b"balance", PayloadFormat::Base64);

        assert!(preview_match_ranges(&preview.text, "balance").is_empty());
    }
}
