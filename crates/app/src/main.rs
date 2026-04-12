fn main() {
    // Initialize structured logging with RUST_LOG env filter
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
        Box::new(|_cc| Ok(Box::new(app::EasyNatsApp))),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}

mod app {
    use eframe::egui;

    #[derive(Default)]
    pub struct EasyNatsApp;

    impl eframe::App for EasyNatsApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.heading("Easy NATS");
                ui.label("NATS Management Tool");
            });
        }
    }
}
