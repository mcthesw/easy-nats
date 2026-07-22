mod builtin;
mod catalog;
mod catppuccin;
mod id;
mod syntax;

use eframe::egui;

pub use catalog::{theme_catalog, theme_definition};
pub use id::ThemeId;
pub(crate) use syntax::{SyntaxPalette, syntax_palette};

pub fn resolve_theme(saved_theme: Option<ThemeId>, system_prefers_dark: Option<bool>) -> ThemeId {
    saved_theme
        .unwrap_or_else(|| ThemeId::from_legacy_dark_mode(system_prefers_dark.unwrap_or(true)))
}

pub fn apply_theme(ctx: &egui::Context, theme_id: ThemeId) {
    let egui_theme = if theme_definition(theme_id).is_dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(egui_theme);

    match theme_id {
        ThemeId::EguiDark | ThemeId::EguiLight => builtin::apply_theme(ctx, theme_id),
        ThemeId::CatppuccinLatte
        | ThemeId::CatppuccinFrappe
        | ThemeId::CatppuccinMacchiato
        | ThemeId::CatppuccinMocha => catppuccin::apply_theme(ctx, theme_id),
    }
}

#[cfg(test)]
mod tests {
    use super::{ThemeId, apply_theme};
    use eframe::egui::{Context, Theme};

    #[test]
    fn applying_dark_theme_switches_egui_preference_from_light() {
        let ctx = Context::default();
        ctx.set_theme(Theme::Light);

        apply_theme(&ctx, ThemeId::CatppuccinMocha);

        assert_eq!(ctx.theme(), Theme::Dark);
        assert!(ctx.global_style().visuals.dark_mode);
    }

    #[test]
    fn applying_light_theme_switches_egui_preference_from_dark() {
        let ctx = Context::default();
        ctx.set_theme(Theme::Dark);

        apply_theme(&ctx, ThemeId::CatppuccinLatte);

        assert_eq!(ctx.theme(), Theme::Light);
        assert!(!ctx.global_style().visuals.dark_mode);
    }
}
