use super::ThemeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeDefinition {
    pub id: ThemeId,
    pub label_key: &'static str,
    pub is_dark: bool,
}

const THEME_CATALOG: [ThemeDefinition; 6] = [
    ThemeDefinition {
        id: ThemeId::EguiDark,
        label_key: "settings.theme_egui_dark",
        is_dark: true,
    },
    ThemeDefinition {
        id: ThemeId::EguiLight,
        label_key: "settings.theme_egui_light",
        is_dark: false,
    },
    ThemeDefinition {
        id: ThemeId::CatppuccinLatte,
        label_key: "settings.theme_catppuccin_latte",
        is_dark: false,
    },
    ThemeDefinition {
        id: ThemeId::CatppuccinFrappe,
        label_key: "settings.theme_catppuccin_frappe",
        is_dark: true,
    },
    ThemeDefinition {
        id: ThemeId::CatppuccinMacchiato,
        label_key: "settings.theme_catppuccin_macchiato",
        is_dark: true,
    },
    ThemeDefinition {
        id: ThemeId::CatppuccinMocha,
        label_key: "settings.theme_catppuccin_mocha",
        is_dark: true,
    },
];

pub fn theme_catalog() -> &'static [ThemeDefinition] {
    &THEME_CATALOG
}

pub fn theme_definition(theme_id: ThemeId) -> &'static ThemeDefinition {
    THEME_CATALOG
        .iter()
        .find(|theme| theme.id == theme_id)
        .expect("theme catalog must define every ThemeId")
}
