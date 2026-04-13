use eframe::egui;

use crate::i18n::{self, Language, t};
use crate::settings::AppSettings;
use crate::tabs::TabKind;

use super::types::TabAction;

pub fn settings_ui(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    dark_mode: &mut bool,
    actions: &mut Vec<TabAction>,
) {
    ui.heading(t("settings.title"));
    ui.separator();

    ui.label(egui::RichText::new(t("settings.section_appearance")).strong());
    ui.add_space(4.0);

    // Language selector
    ui.horizontal(|ui| {
        ui.label(t("settings.language"));
        let mut lang = i18n::current_language();
        egui::ComboBox::from_id_salt("settings_lang")
            .selected_text(lang.label())
            .show_ui(ui, |ui| {
                for l in Language::ALL {
                    if ui.selectable_value(&mut lang, l, l.label()).changed() {
                        i18n::set_language(lang);
                        settings.language = lang;
                        settings.save();
                    }
                }
            });
    });

    // Theme toggle
    ui.horizontal(|ui| {
        ui.label(t("settings.theme"));
        let dark_label = t("settings.theme_dark");
        let light_label = t("settings.theme_light");
        if ui.selectable_label(*dark_mode, dark_label).clicked() && !*dark_mode {
            *dark_mode = true;
            settings.dark_mode = true;
            settings.save();
            actions.push(TabAction::ApplyTheme { dark: true });
        }
        if ui.selectable_label(!*dark_mode, light_label).clicked() && *dark_mode {
            *dark_mode = false;
            settings.dark_mode = false;
            settings.save();
            actions.push(TabAction::ApplyTheme { dark: false });
        }
    });

    ui.add_space(12.0);
    ui.separator();

    // Log viewer shortcut
    if ui.button(t("settings.open_log_viewer")).clicked() {
        actions.push(TabAction::OpenTab(Box::new(TabKind::LogViewer)));
    }
}
