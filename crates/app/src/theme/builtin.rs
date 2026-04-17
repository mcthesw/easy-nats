use eframe::egui;

use super::ThemeId;

pub(super) fn apply_theme(ctx: &egui::Context, theme_id: ThemeId) {
    match theme_id {
        ThemeId::EguiDark => ctx.set_visuals(egui::Visuals::dark()),
        ThemeId::EguiLight => ctx.set_visuals(egui::Visuals::light()),
        _ => unreachable!("builtin theme provider only handles egui built-in themes"),
    }
}
