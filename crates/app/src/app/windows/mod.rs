mod connection;
mod consumer;
mod kv;
mod obj_store;
mod stream;

use eframe::egui;

use super::model::EasyNatsApp;

pub(crate) fn render_windows(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    connection::render(app, ui);
    stream::render(app, ui);
    consumer::render(app, ui);
    consumer::render_edit(app, ui);
    kv::render(app, ui);
    obj_store::render(app, ui);
}
