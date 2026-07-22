use base64::Engine;
use eframe::egui;

use crate::proto::{AutoDetectResult, ProtoViewState};
use crate::schema::{MessageSchemaManager, PayloadSchemaStatus, SchemaStatusLevel};
use crate::theme::SyntaxPalette;

mod json;
pub mod msgpack;
mod preview;

pub use json::{format_json, json_syntax_highlight};
pub use msgpack::format_msgpack;
pub use preview::{
    READ_ONLY_PREVIEW_FORMATS, format_read_only_preview, read_only_preview_layout_job,
};

/// Display format for message payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayloadFormat {
    #[default]
    Auto,
    Text,
    Json,
    MessagePack,
    Hex,
    Base64,
    Protobuf,
}

impl PayloadFormat {
    pub const ALL: &[PayloadFormat] = &[
        PayloadFormat::Auto,
        PayloadFormat::Text,
        PayloadFormat::Json,
        PayloadFormat::MessagePack,
        PayloadFormat::Hex,
        PayloadFormat::Base64,
        PayloadFormat::Protobuf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PayloadFormat::Auto => "Auto",
            PayloadFormat::Text => "Text",
            PayloadFormat::Json => "JSON",
            PayloadFormat::MessagePack => "MsgPack",
            PayloadFormat::Hex => "Hex",
            PayloadFormat::Base64 => "Base64",
            PayloadFormat::Protobuf => "Protobuf",
        }
    }
}

/// Detects the best display format for `data`.
/// Order: valid JSON -> valid UTF-8 text -> confident MessagePack -> hex dump.
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
    if msgpack::is_confident_auto(data) {
        return PayloadFormat::MessagePack;
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
pub fn render_payload(
    ui: &mut egui::Ui,
    data: &[u8],
    format: PayloadFormat,
    syntax_palette: SyntaxPalette,
) {
    let resolved = resolve_format(format, data);
    match resolved {
        PayloadFormat::Json => {
            let pretty = format_json(data);
            let job = json_syntax_highlight(&pretty, syntax_palette);
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
        PayloadFormat::MessagePack => {
            let msgpack = format_msgpack(data);
            ui.label(egui::RichText::new(msgpack).monospace());
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
    syntax_palette: SyntaxPalette,
) {
    let resolved = resolve_format(format, data);
    if resolved == PayloadFormat::Protobuf {
        render_protobuf_payload(ui, data, id_salt, proto_state, manager, syntax_palette);
    } else {
        render_payload(ui, data, format, syntax_palette);
    }
}

/// Render formatted payload with subject-bound schema support.
pub struct SchemaRenderContext<'a> {
    pub manager: &'a MessageSchemaManager,
    pub connection_id: u64,
    pub subject: &'a str,
    pub syntax_palette: SyntaxPalette,
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
                let job = json_syntax_highlight(&json, schema.syntax_palette);
                ui.label(job);
            } else {
                render_payload(ui, data, PayloadFormat::Auto, schema.syntax_palette);
            }
        } else {
            render_payload_with_proto(
                ui,
                data,
                format,
                id_salt,
                proto_state,
                schema.manager,
                schema.syntax_palette,
            );
        }
    } else {
        render_payload_with_proto(
            ui,
            data,
            format,
            id_salt,
            proto_state,
            schema.manager,
            schema.syntax_palette,
        );
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
    syntax_palette: SyntaxPalette,
) {
    let message_types = manager.manual_message_types();
    if !message_types.is_empty() {
        render_proto_type_selector(ui, id_salt, proto_state, &message_types);
        ui.add_space(4.0);
        render_proto_decoded(ui, data, proto_state, manager, syntax_palette);
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
    syntax_palette: SyntaxPalette,
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
                let job = json_syntax_highlight(&json, syntax_palette);
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
                let job = json_syntax_highlight(&json, syntax_palette);
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
    use rmpv::{Value, encode::write_value};

    fn encode_msgpack(value: Value) -> Vec<u8> {
        let mut data = Vec::new();
        write_value(&mut data, &value).unwrap();
        data
    }

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
    fn detect_messagepack_only_for_confident_structures() {
        let map = encode_msgpack(Value::Map(vec![(Value::from("kind"), Value::from("path"))]));
        let array = encode_msgpack(Value::Array(vec![Value::from(1), Value::from(2)]));
        let binary = encode_msgpack(Value::Binary(vec![0, 1, 255]));
        let ext = encode_msgpack(Value::Ext(-1, vec![0, 0, 0, 1]));

        assert_eq!(detect_format(&map), PayloadFormat::MessagePack);
        assert_eq!(detect_format(&array), PayloadFormat::MessagePack);
        assert_eq!(detect_format(&binary), PayloadFormat::MessagePack);
        assert_eq!(detect_format(&ext), PayloadFormat::MessagePack);

        let boolean = encode_msgpack(Value::Boolean(true));
        assert_eq!(detect_format(&boolean), PayloadFormat::Hex);
    }

    #[test]
    fn detect_auto_keeps_json_and_utf8_before_messagepack() {
        assert_eq!(detect_format(br#"{"a":1}"#), PayloadFormat::Json);

        let msgpack_positive_integer = encode_msgpack(Value::from(42));
        assert_eq!(
            detect_format(&msgpack_positive_integer),
            PayloadFormat::Text
        );
    }

    #[test]
    fn detect_auto_rejects_messagepack_with_trailing_bytes() {
        let mut data = encode_msgpack(Value::Map(vec![(Value::from("a"), Value::from(1))]));
        data.push(0);

        assert_eq!(detect_format(&data), PayloadFormat::Hex);
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
    fn messagepack_display_formats_maps_arrays_and_scalars() {
        let data = encode_msgpack(Value::Map(vec![
            (Value::from("kind"), Value::from("path")),
            (
                Value::from("items"),
                Value::Array(vec![Value::from(1), Value::Boolean(true)]),
            ),
        ]));

        let rendered = format_msgpack(&data);

        assert!(rendered.contains(r#""kind": "path""#));
        assert!(rendered.contains(r#""items": ["#));
        assert!(rendered.contains("1,"));
        assert!(rendered.contains("true"));

        let scalar = encode_msgpack(Value::from(42));
        assert_eq!(format_msgpack(&scalar), "42");
    }

    #[test]
    fn messagepack_display_preserves_binary_ext_and_non_string_keys() {
        let binary = encode_msgpack(Value::Binary(vec![0, 1, 255]));
        assert!(format_msgpack(&binary).contains("bin(3): 00 01 FF"));

        let timestamp_ext = encode_msgpack(Value::Ext(-1, vec![0, 0, 0, 1]));
        assert!(format_msgpack(&timestamp_ext).contains("ext(type: -1, len: 4): 00 00 00 01"));

        let map = encode_msgpack(Value::Map(vec![(Value::from(7), Value::from("lucky"))]));
        assert!(format_msgpack(&map).contains(r#"[7]: "lucky""#));
    }

    #[test]
    fn messagepack_display_reports_local_decode_errors() {
        let rendered = format_msgpack(&[0x81, 0xa1]);

        assert!(rendered.starts_with("MsgPack decode error:"));
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
    fn read_only_preview_supports_messagepack_selection() {
        assert!(READ_ONLY_PREVIEW_FORMATS.contains(&PayloadFormat::MessagePack));

        let data = encode_msgpack(Value::Map(vec![(Value::from("a"), Value::from(1))]));
        let preview = format_read_only_preview(&data, PayloadFormat::Auto);

        assert_eq!(preview.resolved_format, PayloadFormat::MessagePack);
        assert!(preview.text.contains(r#""a": 1"#));
    }

    #[test]
    fn read_only_preview_keeps_display_errors_isolated() {
        let invalid = [0x81, 0xa1];
        let msgpack = format_read_only_preview(&invalid, PayloadFormat::MessagePack);
        let hex = format_read_only_preview(&invalid, PayloadFormat::Hex);
        let text = format_read_only_preview(&invalid, PayloadFormat::Text);

        assert!(msgpack.text.starts_with("MsgPack decode error:"));
        assert!(hex.text.contains("81 A1"));
        assert!(!text.text.starts_with("MsgPack decode error:"));
    }

    #[test]
    fn read_only_preview_falls_back_for_unsupported_format() {
        let preview = format_read_only_preview(br#"{"a":1}"#, PayloadFormat::Protobuf);

        assert_eq!(preview.resolved_format, PayloadFormat::Json);
        assert!(preview.text.contains("\"a\": 1"));
    }

    #[test]
    fn preview_match_ranges_find_ascii_matches_case_insensitively() {
        let ranges = preview::preview_match_ranges("Balance balance BALANCE", "balance");

        assert_eq!(ranges, vec![0..7, 8..15, 16..23]);
    }

    #[test]
    fn preview_match_ranges_find_matches_after_json_formatting() {
        let preview =
            format_read_only_preview(br#"{"event_id":"abc","kind":"path"}"#, PayloadFormat::Json);
        let ranges = preview::preview_match_ranges(&preview.text, "kind");

        assert_eq!(ranges.len(), 1);
        assert_eq!(&preview.text[ranges[0].clone()], "kind");
    }

    #[test]
    fn preview_match_ranges_omit_absent_transformed_output() {
        let preview = format_read_only_preview(b"balance", PayloadFormat::Base64);

        assert!(preview::preview_match_ranges(&preview.text, "balance").is_empty());
    }
}
