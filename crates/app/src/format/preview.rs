use eframe::egui;
use egui::text::LayoutJob;
use std::ops::Range;

use super::{
    PayloadFormat, format_base64, format_hex, format_json, format_msgpack, format_text,
    resolve_format,
};

pub const READ_ONLY_PREVIEW_FORMATS: &[PayloadFormat] = &[
    PayloadFormat::Auto,
    PayloadFormat::Text,
    PayloadFormat::Json,
    PayloadFormat::MessagePack,
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
        PayloadFormat::MessagePack => format_msgpack(data),
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
