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

    ui.add_space(12.0);
    ui.separator();
    ui.label(egui::RichText::new(t("settings.section_protobuf")).strong());
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(t("settings.proto_schema_dir"));
        let display = settings
            .proto_schema_dir
            .as_deref()
            .unwrap_or(t("settings.proto_not_set"));
        ui.label(display);
    });
    ui.horizontal(|ui| {
        if ui.button(t("settings.proto_browse")).clicked()
            && let Some(dir) = rfd::FileDialog::new().pick_folder()
        {
            let dir_str = dir.to_string_lossy().to_string();
            settings.proto_schema_dir = Some(dir_str.clone());
            settings.save();
            actions.push(TabAction::LoadProtoSchemas { dir: dir_str });
        }
        if settings.proto_schema_dir.is_some() {
            if ui.button(t("settings.proto_reload")).clicked()
                && let Some(dir) = &settings.proto_schema_dir
            {
                actions.push(TabAction::LoadProtoSchemas { dir: dir.clone() });
            }
            if ui.button(t("settings.proto_clear")).clicked() {
                settings.proto_schema_dir = None;
                settings.save();
                actions.push(TabAction::ClearProtoSchemas);
            }
        }
    });
}
