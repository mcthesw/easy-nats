mod connection;
mod consumer;
mod kv;
mod stream;

use eframe::egui;

use super::model::EasyNatsApp;

pub(crate) fn render_windows(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    connection::render(app, ui);
    stream::render(app, ui);
    consumer::render(app, ui);
    kv::render(app, ui);
}
