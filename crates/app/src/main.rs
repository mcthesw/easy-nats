#![windows_subsystem = "windows"]

mod app;
mod format;
mod i18n;
mod settings;
mod tabs;
mod toast;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let app_settings = settings::AppSettings::load();
    i18n::init(app_settings.language);

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "Easy NATS",
        native_options,
        Box::new(|cc| {
            let dark = cc
                .egui_ctx
                .system_theme()
                .map(|t| t == eframe::egui::Theme::Dark)
                .unwrap_or(app_settings.dark_mode);
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
