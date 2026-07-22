use eframe::egui::{self, Color32};

use super::{SyntaxPalette, ThemeId};

// Portions of the Catppuccin theme implementation below are derived from
// https://github.com/catppuccin/egui (commit ffb92d2da71b72bde41d83bbc0a46917de97b486),
// licensed under MIT:
//
// Copyright (c) 2023-present Catppuccin Org
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct CatppuccinTheme {
    rosewater: Color32,
    flamingo: Color32,
    pink: Color32,
    mauve: Color32,
    red: Color32,
    maroon: Color32,
    peach: Color32,
    yellow: Color32,
    green: Color32,
    teal: Color32,
    sky: Color32,
    sapphire: Color32,
    blue: Color32,
    lavender: Color32,
    text: Color32,
    subtext1: Color32,
    subtext0: Color32,
    overlay2: Color32,
    overlay1: Color32,
    overlay0: Color32,
    surface2: Color32,
    surface1: Color32,
    surface0: Color32,
    base: Color32,
    mantle: Color32,
    crust: Color32,
}

const LATTE: CatppuccinTheme = CatppuccinTheme {
    rosewater: Color32::from_rgb(220, 138, 120),
    flamingo: Color32::from_rgb(221, 120, 120),
    pink: Color32::from_rgb(234, 118, 203),
    mauve: Color32::from_rgb(136, 57, 239),
    red: Color32::from_rgb(210, 15, 57),
    maroon: Color32::from_rgb(230, 69, 83),
    peach: Color32::from_rgb(254, 100, 11),
    yellow: Color32::from_rgb(223, 142, 29),
    green: Color32::from_rgb(64, 160, 43),
    teal: Color32::from_rgb(23, 146, 153),
    sky: Color32::from_rgb(4, 165, 229),
    sapphire: Color32::from_rgb(32, 159, 181),
    blue: Color32::from_rgb(30, 102, 245),
    lavender: Color32::from_rgb(114, 135, 253),
    text: Color32::from_rgb(76, 79, 105),
    subtext1: Color32::from_rgb(92, 95, 119),
    subtext0: Color32::from_rgb(108, 111, 133),
    overlay2: Color32::from_rgb(124, 127, 147),
    overlay1: Color32::from_rgb(140, 143, 161),
    overlay0: Color32::from_rgb(156, 160, 176),
    surface2: Color32::from_rgb(172, 176, 190),
    surface1: Color32::from_rgb(188, 192, 204),
    surface0: Color32::from_rgb(204, 208, 218),
    base: Color32::from_rgb(239, 241, 245),
    mantle: Color32::from_rgb(230, 233, 239),
    crust: Color32::from_rgb(220, 224, 232),
};

const FRAPPE: CatppuccinTheme = CatppuccinTheme {
    rosewater: Color32::from_rgb(242, 213, 207),
    flamingo: Color32::from_rgb(238, 190, 190),
    pink: Color32::from_rgb(244, 184, 228),
    mauve: Color32::from_rgb(202, 158, 230),
    red: Color32::from_rgb(231, 130, 132),
    maroon: Color32::from_rgb(234, 153, 156),
    peach: Color32::from_rgb(239, 159, 118),
    yellow: Color32::from_rgb(229, 200, 144),
    green: Color32::from_rgb(166, 209, 137),
    teal: Color32::from_rgb(129, 200, 190),
    sky: Color32::from_rgb(153, 209, 219),
    sapphire: Color32::from_rgb(133, 193, 220),
    blue: Color32::from_rgb(140, 170, 238),
    lavender: Color32::from_rgb(186, 187, 241),
    text: Color32::from_rgb(198, 208, 245),
    subtext1: Color32::from_rgb(181, 191, 226),
    subtext0: Color32::from_rgb(165, 173, 206),
    overlay2: Color32::from_rgb(148, 156, 187),
    overlay1: Color32::from_rgb(131, 139, 167),
    overlay0: Color32::from_rgb(115, 121, 148),
    surface2: Color32::from_rgb(98, 104, 128),
    surface1: Color32::from_rgb(81, 87, 109),
    surface0: Color32::from_rgb(65, 69, 89),
    base: Color32::from_rgb(48, 52, 70),
    mantle: Color32::from_rgb(41, 44, 60),
    crust: Color32::from_rgb(35, 38, 52),
};

const MACCHIATO: CatppuccinTheme = CatppuccinTheme {
    rosewater: Color32::from_rgb(244, 219, 214),
    flamingo: Color32::from_rgb(240, 198, 198),
    pink: Color32::from_rgb(245, 189, 230),
    mauve: Color32::from_rgb(198, 160, 246),
    red: Color32::from_rgb(237, 135, 150),
    maroon: Color32::from_rgb(238, 153, 160),
    peach: Color32::from_rgb(245, 169, 127),
    yellow: Color32::from_rgb(238, 212, 159),
    green: Color32::from_rgb(166, 218, 149),
    teal: Color32::from_rgb(139, 213, 202),
    sky: Color32::from_rgb(145, 215, 227),
    sapphire: Color32::from_rgb(125, 196, 228),
    blue: Color32::from_rgb(138, 173, 244),
    lavender: Color32::from_rgb(183, 189, 248),
    text: Color32::from_rgb(202, 211, 245),
    subtext1: Color32::from_rgb(184, 192, 224),
    subtext0: Color32::from_rgb(165, 173, 203),
    overlay2: Color32::from_rgb(147, 154, 183),
    overlay1: Color32::from_rgb(128, 135, 162),
    overlay0: Color32::from_rgb(110, 115, 141),
    surface2: Color32::from_rgb(91, 96, 120),
    surface1: Color32::from_rgb(73, 77, 100),
    surface0: Color32::from_rgb(54, 58, 79),
    base: Color32::from_rgb(36, 39, 58),
    mantle: Color32::from_rgb(30, 32, 48),
    crust: Color32::from_rgb(24, 25, 38),
};

const MOCHA: CatppuccinTheme = CatppuccinTheme {
    rosewater: Color32::from_rgb(245, 224, 220),
    flamingo: Color32::from_rgb(242, 205, 205),
    pink: Color32::from_rgb(245, 194, 231),
    mauve: Color32::from_rgb(203, 166, 247),
    red: Color32::from_rgb(243, 139, 168),
    maroon: Color32::from_rgb(235, 160, 172),
    peach: Color32::from_rgb(250, 179, 135),
    yellow: Color32::from_rgb(249, 226, 175),
    green: Color32::from_rgb(166, 227, 161),
    teal: Color32::from_rgb(148, 226, 213),
    sky: Color32::from_rgb(137, 220, 235),
    sapphire: Color32::from_rgb(116, 199, 236),
    blue: Color32::from_rgb(137, 180, 250),
    lavender: Color32::from_rgb(180, 190, 254),
    text: Color32::from_rgb(205, 214, 244),
    subtext1: Color32::from_rgb(186, 194, 222),
    subtext0: Color32::from_rgb(166, 173, 200),
    overlay2: Color32::from_rgb(147, 153, 178),
    overlay1: Color32::from_rgb(127, 132, 156),
    overlay0: Color32::from_rgb(108, 112, 134),
    surface2: Color32::from_rgb(88, 91, 112),
    surface1: Color32::from_rgb(69, 71, 90),
    surface0: Color32::from_rgb(49, 50, 68),
    base: Color32::from_rgb(30, 30, 46),
    mantle: Color32::from_rgb(24, 24, 37),
    crust: Color32::from_rgb(17, 17, 27),
};

pub(super) fn apply_theme(ctx: &egui::Context, theme_id: ThemeId) {
    let theme = resolve_theme(theme_id);

    let old = ctx.global_style().visuals.clone();
    ctx.set_visuals(make_visuals(theme, old));
}

pub(super) fn syntax_palette(theme_id: ThemeId) -> SyntaxPalette {
    let theme = resolve_theme(theme_id);
    SyntaxPalette {
        plain: theme.text,
        property: theme.blue,
        string: theme.green,
        number: theme.peach,
        language_constant: theme.peach,
        punctuation: theme.overlay2,
    }
}

fn resolve_theme(theme_id: ThemeId) -> CatppuccinTheme {
    match theme_id {
        ThemeId::CatppuccinLatte => LATTE,
        ThemeId::CatppuccinFrappe => FRAPPE,
        ThemeId::CatppuccinMacchiato => MACCHIATO,
        ThemeId::CatppuccinMocha => MOCHA,
        _ => unreachable!("Catppuccin theme provider only handles Catppuccin themes"),
    }
}

fn make_visuals(theme: CatppuccinTheme, old: egui::Visuals) -> egui::Visuals {
    let is_latte = theme == LATTE;
    let shadow_color = if is_latte {
        Color32::from_black_alpha(25)
    } else {
        Color32::from_black_alpha(96)
    };

    egui::Visuals {
        hyperlink_color: theme.rosewater,
        faint_bg_color: theme.surface0,
        extreme_bg_color: theme.crust,
        code_bg_color: theme.mantle,
        warn_fg_color: theme.peach,
        error_fg_color: theme.maroon,
        window_fill: theme.base,
        panel_fill: theme.base,
        window_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.window_stroke
        },
        widgets: egui::style::Widgets {
            noninteractive: make_widget_visual(old.widgets.noninteractive, theme, theme.base),
            inactive: make_widget_visual(old.widgets.inactive, theme, theme.surface0),
            hovered: make_widget_visual(old.widgets.hovered, theme, theme.surface2),
            active: make_widget_visual(old.widgets.active, theme, theme.surface1),
            open: make_widget_visual(old.widgets.open, theme, theme.surface0),
        },
        selection: egui::style::Selection {
            bg_fill: theme.blue.linear_multiply(if is_latte { 0.4 } else { 0.2 }),
            stroke: egui::Stroke {
                color: theme.text,
                ..old.selection.stroke
            },
        },
        window_shadow: egui::epaint::Shadow {
            color: shadow_color,
            ..old.window_shadow
        },
        popup_shadow: egui::epaint::Shadow {
            color: shadow_color,
            ..old.popup_shadow
        },
        dark_mode: !is_latte,
        ..old
    }
}

fn make_widget_visual(
    old: egui::style::WidgetVisuals,
    theme: CatppuccinTheme,
    bg_fill: Color32,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.bg_stroke
        },
        fg_stroke: egui::Stroke {
            color: theme.text,
            ..old.fg_stroke
        },
        ..old
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_palettes_use_unmodified_official_colors() {
        for (theme_id, expected) in [
            (
                ThemeId::CatppuccinLatte,
                (
                    [76, 79, 105],
                    [30, 102, 245],
                    [64, 160, 43],
                    [254, 100, 11],
                    [124, 127, 147],
                ),
            ),
            (
                ThemeId::CatppuccinFrappe,
                (
                    [198, 208, 245],
                    [140, 170, 238],
                    [166, 209, 137],
                    [239, 159, 118],
                    [148, 156, 187],
                ),
            ),
            (
                ThemeId::CatppuccinMacchiato,
                (
                    [202, 211, 245],
                    [138, 173, 244],
                    [166, 218, 149],
                    [245, 169, 127],
                    [147, 154, 183],
                ),
            ),
            (
                ThemeId::CatppuccinMocha,
                (
                    [205, 214, 244],
                    [137, 180, 250],
                    [166, 227, 161],
                    [250, 179, 135],
                    [147, 153, 178],
                ),
            ),
        ] {
            let syntax = syntax_palette(theme_id);
            let (text, blue, green, peach, overlay2) = expected;
            assert_eq!(syntax.plain, Color32::from_rgb(text[0], text[1], text[2]));
            assert_eq!(
                syntax.property,
                Color32::from_rgb(blue[0], blue[1], blue[2])
            );
            assert_eq!(
                syntax.string,
                Color32::from_rgb(green[0], green[1], green[2])
            );
            assert_eq!(
                syntax.number,
                Color32::from_rgb(peach[0], peach[1], peach[2])
            );
            assert_eq!(
                syntax.language_constant,
                Color32::from_rgb(peach[0], peach[1], peach[2])
            );
            assert_eq!(
                syntax.punctuation,
                Color32::from_rgb(overlay2[0], overlay2[1], overlay2[2])
            );
        }
    }

    #[test]
    fn each_catppuccin_flavor_has_distinct_syntax_colors() {
        let palettes = [
            syntax_palette(ThemeId::CatppuccinLatte),
            syntax_palette(ThemeId::CatppuccinFrappe),
            syntax_palette(ThemeId::CatppuccinMacchiato),
            syntax_palette(ThemeId::CatppuccinMocha),
        ];

        for (index, palette) in palettes.iter().enumerate() {
            assert!(palettes[index + 1..].iter().all(|other| other != palette));
        }
    }
}
