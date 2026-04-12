mod app;
mod format;
mod tabs;
mod toast;
mod ui_strings;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let native_options = eframe::NativeOptions::default();
    if let Err(e) = eframe::run_native(
        "Easy NATS",
        native_options,
        Box::new(|cc| {
            let dark = cc
                .egui_ctx
                .system_theme()
                .map(|t| t == eframe::egui::Theme::Dark)
                .unwrap_or(true);
            if dark {
                cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(eframe::egui::Visuals::light());
            }
            Ok(Box::new(app::EasyNatsApp::new(dark)))
        }),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}
