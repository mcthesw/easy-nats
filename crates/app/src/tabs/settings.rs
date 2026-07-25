use eframe::egui;

use crate::i18n::{self, Language, t};
use crate::settings::{AppSettings, PubSubTabMode};
use crate::theme::{ThemeId, theme_catalog, theme_definition};

use super::types::TabAction;

pub fn settings_ui(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    theme_id: &mut ThemeId,
    actions: &mut Vec<TabAction>,
    supports_local_files: bool,
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

    // Theme selector
    ui.horizontal(|ui| {
        ui.label(t("settings.theme"));
        egui::ComboBox::from_id_salt("settings_theme")
            .selected_text(t(theme_definition(*theme_id).label_key))
            .show_ui(ui, |ui| {
                for theme in theme_catalog().iter().copied() {
                    if ui
                        .selectable_value(theme_id, theme.id, t(theme.label_key))
                        .changed()
                    {
                        settings.theme = Some(theme.id);
                        settings.save();
                        actions.push(TabAction::ApplyTheme { theme_id: theme.id });
                    }
                }
            });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.label(egui::RichText::new(t("settings.section_behavior")).strong());
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(t("settings.pubsub_tab_mode"));
        egui::ComboBox::from_id_salt("settings_pubsub_tab_mode")
            .selected_text(t(settings.pubsub_tab_mode.label_key()))
            .show_ui(ui, |ui| {
                for mode in PubSubTabMode::ALL {
                    if ui
                        .selectable_value(&mut settings.pubsub_tab_mode, mode, t(mode.label_key()))
                        .changed()
                    {
                        settings.save();
                    }
                }
            });
    });

    if ui
        .checkbox(
            &mut settings.show_backing_streams_in_sidebar,
            t("settings.show_backing_streams"),
        )
        .changed()
    {
        settings.save();
    }

    if supports_local_files {
        ui.add_space(12.0);
        ui.separator();
        ui.label(egui::RichText::new(t("settings.section_message_schemas")).strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(t("settings.message_schemas_hint"));
            if ui.button(t("settings.open_message_schemas")).clicked() {
                actions.push(TabAction::OpenMessageSchemas);
            }
        });
    }
}
