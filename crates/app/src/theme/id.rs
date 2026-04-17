use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeId {
    #[serde(rename = "egui-dark")]
    EguiDark,
    #[serde(rename = "egui-light")]
    EguiLight,
    #[serde(rename = "catppuccin-latte")]
    CatppuccinLatte,
    #[serde(rename = "catppuccin-frappe")]
    CatppuccinFrappe,
    #[serde(rename = "catppuccin-macchiato")]
    CatppuccinMacchiato,
    #[serde(rename = "catppuccin-mocha")]
    CatppuccinMocha,
}

impl ThemeId {
    pub fn from_legacy_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::EguiDark
        } else {
            Self::EguiLight
        }
    }
}
