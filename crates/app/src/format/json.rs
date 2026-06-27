use eframe::egui;
use egui::text::LayoutJob;

use super::format_text;

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
        egui::Color32::from_rgb(152, 195, 121)
    } else {
        egui::Color32::from_rgb(80, 161, 79)
    };
    let number_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(209, 154, 102)
    } else {
        egui::Color32::from_rgb(152, 104, 1)
    };
    let keyword_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(86, 182, 194)
    } else {
        egui::Color32::from_rgb(1, 132, 188)
    };
    let key_color = if style.visuals.dark_mode {
        egui::Color32::from_rgb(224, 108, 117)
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
                let s = String::from(ch);
                job.append(&s, 0.0, egui::TextFormat::simple(mono.clone(), base_color));
                i += 1;
            }
        }
    }
    job
}
