use eframe::egui;
use egui::text::LayoutJob;

use crate::theme::SyntaxPalette;

use super::format_text;

/// Pretty-print JSON. Returns the formatted string.
pub fn format_json(data: &[u8]) -> String {
    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| format_text(data)),
        Err(_) => format_text(data),
    }
}

/// Build a `LayoutJob` with basic JSON syntax highlighting.
pub fn json_syntax_highlight(json: &str, palette: SyntaxPalette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mono = egui::FontId::monospace(13.0);

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
                let color = if after_colon {
                    palette.string
                } else {
                    palette.property
                };
                job.append(&s, 0.0, egui::TextFormat::simple(mono.clone(), color));
                after_colon = false;
            }
            ':' => {
                job.append(
                    ":",
                    0.0,
                    egui::TextFormat::simple(mono.clone(), palette.punctuation),
                );
                after_colon = true;
                i += 1;
            }
            ',' | '{' | '}' | '[' | ']' => {
                let s = String::from(ch);
                job.append(
                    &s,
                    0.0,
                    egui::TextFormat::simple(mono.clone(), palette.punctuation),
                );
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
                    egui::TextFormat::simple(mono.clone(), palette.language_constant),
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
                    egui::TextFormat::simple(mono.clone(), palette.number),
                );
                after_colon = false;
            }
            _ => {
                let s = String::from(ch);
                job.append(
                    &s,
                    0.0,
                    egui::TextFormat::simple(mono.clone(), palette.plain),
                );
                i += 1;
            }
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use eframe::egui::Color32;

    use super::*;

    const TEST_PALETTE: SyntaxPalette = SyntaxPalette {
        plain: Color32::from_rgb(1, 2, 3),
        property: Color32::from_rgb(4, 5, 6),
        string: Color32::from_rgb(7, 8, 9),
        number: Color32::from_rgb(10, 11, 12),
        language_constant: Color32::from_rgb(13, 14, 15),
        punctuation: Color32::from_rgb(16, 17, 18),
    };

    fn color_for(job: &LayoutJob, text: &str) -> Color32 {
        let start = egui::text::ByteIndex(job.text.find(text).expect("token must be present"));
        job.sections
            .iter()
            .find(|section| section.byte_range.contains(&start))
            .map(|section| section.format.color)
            .expect("token must belong to a layout section")
    }

    #[test]
    fn syntax_highlight_uses_semantic_palette_roles() {
        let json = r#"{"key":"value","number":-12.5e+2,"boolean":true,"empty":null}"#;
        let job = json_syntax_highlight(json, TEST_PALETTE);

        assert_eq!(color_for(&job, r#""key""#), TEST_PALETTE.property);
        assert_eq!(color_for(&job, r#""value""#), TEST_PALETTE.string);
        assert_eq!(color_for(&job, "-12.5e+2"), TEST_PALETTE.number);
        assert_eq!(color_for(&job, "true"), TEST_PALETTE.language_constant);
        assert_eq!(color_for(&job, "null"), TEST_PALETTE.language_constant);
        assert_eq!(color_for(&job, "{"), TEST_PALETTE.punctuation);
        assert_eq!(color_for(&job, ":"), TEST_PALETTE.punctuation);
    }

    #[test]
    fn syntax_highlight_keeps_whitespace_in_plain_text_color() {
        let job = json_syntax_highlight("{ \"key\": true }", TEST_PALETTE);

        assert_eq!(color_for(&job, " "), TEST_PALETTE.plain);
    }
}
