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

    // Fade disabled widgets more strongly than the egui default so that
    // enabled and disabled states are easy to tell apart in every theme.
    ctx.all_styles_mut(|style| {
        style.visuals.disabled_alpha = 0.35;
    });
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

    #[test]
    fn every_theme_fades_disabled_widgets() {
        let ctx = Context::default();

        for theme in crate::theme::theme_catalog() {
            apply_theme(&ctx, theme.id);

            for egui_theme in [Theme::Dark, Theme::Light] {
                let visuals = &ctx.style_of(egui_theme).visuals;
                assert_eq!(
                    visuals.disabled_alpha, 0.35,
                    "theme {:?} must fade disabled widgets",
                    theme.id
                );
            }
        }
    }
}
