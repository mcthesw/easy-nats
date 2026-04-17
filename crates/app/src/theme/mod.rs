mod builtin;
mod catalog;
mod catppuccin;
mod id;

use eframe::egui;

pub use catalog::{ThemeDefinition, theme_catalog, theme_definition};
pub use id::ThemeId;

pub fn resolve_theme(saved_theme: Option<ThemeId>, system_prefers_dark: Option<bool>) -> ThemeId {
    saved_theme
        .unwrap_or_else(|| ThemeId::from_legacy_dark_mode(system_prefers_dark.unwrap_or(true)))
}

pub fn apply_theme(ctx: &egui::Context, theme_id: ThemeId) {
    match theme_id {
        ThemeId::EguiDark | ThemeId::EguiLight => builtin::apply_theme(ctx, theme_id),
        ThemeId::CatppuccinLatte
        | ThemeId::CatppuccinFrappe
        | ThemeId::CatppuccinMacchiato
        | ThemeId::CatppuccinMocha => catppuccin::apply_theme(ctx, theme_id),
    }
}
