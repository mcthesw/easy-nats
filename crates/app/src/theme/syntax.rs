use eframe::egui::{Color32, Visuals};

use super::{ThemeId, catppuccin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyntaxPalette {
    pub plain: Color32,
    pub property: Color32,
    pub string: Color32,
    pub number: Color32,
    pub language_constant: Color32,
    pub punctuation: Color32,
}

pub(crate) fn syntax_palette(theme_id: ThemeId) -> SyntaxPalette {
    match theme_id {
        ThemeId::EguiDark => builtin_palette(true),
        ThemeId::EguiLight => builtin_palette(false),
        ThemeId::CatppuccinLatte
        | ThemeId::CatppuccinFrappe
        | ThemeId::CatppuccinMacchiato
        | ThemeId::CatppuccinMocha => catppuccin::syntax_palette(theme_id),
    }
}

fn builtin_palette(dark: bool) -> SyntaxPalette {
    let plain = if dark {
        Visuals::dark().text_color()
    } else {
        Visuals::light().text_color()
    };
    let (property, string, number, language_constant) = if dark {
        (
            Color32::from_rgb(224, 108, 117),
            Color32::from_rgb(152, 195, 121),
            Color32::from_rgb(209, 154, 102),
            Color32::from_rgb(86, 182, 194),
        )
    } else {
        (
            Color32::from_rgb(228, 86, 73),
            Color32::from_rgb(80, 161, 79),
            Color32::from_rgb(152, 104, 1),
            Color32::from_rgb(1, 132, 188),
        )
    };

    SyntaxPalette {
        plain,
        property,
        string,
        number,
        language_constant,
        punctuation: plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egui_palettes_preserve_existing_json_colors() {
        let dark = syntax_palette(ThemeId::EguiDark);
        assert_eq!(dark.property, Color32::from_rgb(224, 108, 117));
        assert_eq!(dark.string, Color32::from_rgb(152, 195, 121));
        assert_eq!(dark.number, Color32::from_rgb(209, 154, 102));
        assert_eq!(dark.language_constant, Color32::from_rgb(86, 182, 194));
        assert_eq!(dark.plain, Visuals::dark().text_color());
        assert_eq!(dark.punctuation, dark.plain);

        let light = syntax_palette(ThemeId::EguiLight);
        assert_eq!(light.property, Color32::from_rgb(228, 86, 73));
        assert_eq!(light.string, Color32::from_rgb(80, 161, 79));
        assert_eq!(light.number, Color32::from_rgb(152, 104, 1));
        assert_eq!(light.language_constant, Color32::from_rgb(1, 132, 188));
        assert_eq!(light.plain, Visuals::light().text_color());
        assert_eq!(light.punctuation, light.plain);
    }

    #[test]
    fn every_theme_resolves_to_a_complete_palette() {
        for theme_id in [
            ThemeId::EguiDark,
            ThemeId::EguiLight,
            ThemeId::CatppuccinLatte,
            ThemeId::CatppuccinFrappe,
            ThemeId::CatppuccinMacchiato,
            ThemeId::CatppuccinMocha,
        ] {
            let palette = syntax_palette(theme_id);
            assert_ne!(palette.plain, Color32::TRANSPARENT);
            assert_ne!(palette.property, Color32::TRANSPARENT);
            assert_ne!(palette.string, Color32::TRANSPARENT);
            assert_ne!(palette.number, Color32::TRANSPARENT);
            assert_ne!(palette.language_constant, Color32::TRANSPARENT);
            assert_ne!(palette.punctuation, Color32::TRANSPARENT);
        }
    }
}
