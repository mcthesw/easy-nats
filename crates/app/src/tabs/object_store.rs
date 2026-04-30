use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle, ObjectStoreBucketInfo, ObjectStoreObjectInfo};

use crate::i18n::t;

use super::common::{auto_refresh_ui, format_bytes};
use super::types::{ObjectStoreBucketState, TabAction};

pub fn obj_store_bucket_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut ObjectStoreBucketState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    ui.horizontal(|ui| {
        ui.heading(bucket_name);
        if ui.button(t("obj_store.delete_bucket")).clicked() {
            actions.push(TabAction::ConfirmDeleteObjStoreBucket {
                connection_id,
                bucket_name: bucket_name.to_string(),
            });
        }
    });

    ui.horizontal(|ui| {
        auto_refresh_ui(ui, "objstore_auto_refresh", &mut state.auto_refresh);
    });
    if state.auto_refresh.should_refresh() {
        backend.send(BackendCommand::ListObjects {
            connection_id,
            bucket: bucket_name.to_string(),
        });
        state.loading_objects = true;
        state.auto_refresh.mark_refreshed();
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    } else if state.auto_refresh.enabled {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    }

    if let Some(info) = &state.info {
        egui::CollapsingHeader::new(t("obj_store.bucket_info"))
            .id_salt(("objstore_bucket_info", connection_id, bucket_name))
            .default_open(false)
            .show(ui, |ui| bucket_info_panel(ui, info));
    }
    ui.separator();

    let panel_id = egui::Id::new(("objstore_left_panel", connection_id, bucket_name));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show_inside(ui, |ui| {
            render_object_list(ui, connection_id, bucket_name, state, backend);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        render_detail_panel(ui, connection_id, bucket_name, state, backend);
    });
}

fn render_object_list(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut ObjectStoreBucketState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.loading_objects, egui::Button::new("↻"))
            .on_hover_text(t("obj_store.refresh"))
            .clicked()
        {
            backend.send(BackendCommand::ListObjects {
                connection_id,
                bucket: bucket_name.to_string(),
            });
            state.loading_objects = true;
        }
        if ui
            .button("⬆")
            .on_hover_text(t("obj_store.upload"))
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_file()
            && let Ok(data) = std::fs::read(&path)
        {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            backend.send(BackendCommand::UploadObject {
                connection_id,
                bucket: bucket_name.to_string(),
                name,
                data,
            });
        }
    });

    ui.add(
        egui::TextEdit::singleline(&mut state.object_filter)
            .hint_text(t("obj_store.filter"))
            .desired_width(ui.available_width()),
    );

    if state.loading_objects {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t("obj_store.loading"));
        });
    }

    let filter_lower = state.object_filter.to_lowercase();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for obj in &state.objects {
            let name = obj.name.as_str();
            if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower) {
                continue;
            }
            let label = format!("{name}  ({})", format_bytes(obj.size as u64));
            let selected = state.selected_object.as_deref() == Some(name);
            if ui.selectable_label(selected, &label).clicked() {
                state.selected_object = Some(name.to_string());
                state.delete_confirm = false;
            }
        }
    });
}

fn render_detail_panel(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut ObjectStoreBucketState,
    backend: &BackendHandle,
) {
    let Some(obj_name) = &state.selected_object else {
        ui.centered_and_justified(|ui| {
            ui.weak(t("obj_store.select_object"));
        });
        return;
    };
    let obj_name = obj_name.clone();

    let obj = state.objects.iter().find(|object| object.name == obj_name);

    ui.heading(&obj_name);
    ui.separator();

    if let Some(obj) = obj {
        render_object_metadata(ui, obj);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button(t("obj_store.download")).clicked()
            && let Some(path) = rfd::FileDialog::new().set_file_name(&obj_name).save_file()
        {
            backend.send(BackendCommand::DownloadObject {
                connection_id,
                bucket: bucket_name.to_string(),
                name: obj_name.clone(),
                file_path: path,
            });
        }

        if !state.delete_confirm {
            if ui.button(t("obj_store.delete_object")).clicked() {
                state.delete_confirm = true;
            }
        } else {
            ui.label(t("obj_store.confirm_delete"));
            if ui.button(t("common.yes")).clicked() {
                backend.send(BackendCommand::DeleteObject {
                    connection_id,
                    bucket: bucket_name.to_string(),
                    name: obj_name.clone(),
                });
                state.selected_object = None;
                state.delete_confirm = false;
            }
            if ui.button(t("common.no")).clicked() {
                state.delete_confirm = false;
            }
        }
    });
}

fn render_object_metadata(ui: &mut egui::Ui, obj: &ObjectStoreObjectInfo) {
    egui::Grid::new("obj_metadata")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("obj_store.size"));
            ui.label(format_bytes(obj.size as u64));
            ui.end_row();

            ui.label(t("obj_store.chunks"));
            ui.label(obj.chunks.to_string());
            ui.end_row();

            if let Some(digest) = obj.digest.as_deref() {
                ui.label(t("obj_store.digest"));
                ui.label(digest);
                ui.end_row();
            }
            if let Some(modified) = obj.modified.as_deref() {
                ui.label(t("obj_store.modified"));
                ui.label(modified);
                ui.end_row();
            }
        });
}

fn bucket_info_panel(ui: &mut egui::Ui, info: &ObjectStoreBucketInfo) {
    egui::Grid::new("objstore_info_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("obj_store.bucket_name"));
            ui.label(&info.bucket);
            ui.end_row();

            ui.label(t("obj_store.total_bytes"));
            ui.label(format_bytes(info.bytes));
            ui.end_row();

            ui.label(t("obj_store.object_count"));
            ui.label(info.object_count.to_string());
            ui.end_row();

            ui.label(t("common.storage_label"));
            ui.label(&info.storage);
            ui.end_row();

            ui.label(t("common.replicas"));
            ui.label(info.num_replicas.to_string());
            ui.end_row();
        });
}
