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
        Box::new(|_cc| Ok(Box::new(app::EasyNatsApp::new()))),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}

mod app {
    use eframe::egui;
    use nats_backend::BackendHandle;

    pub struct EasyNatsApp {
        backend: BackendHandle,
    }

    impl EasyNatsApp {
        pub fn new() -> Self {
            Self {
                backend: BackendHandle::spawn(),
            }
        }
    }

    impl eframe::App for EasyNatsApp {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            // Poll backend events each frame
            let events = self.backend.drain_events();
            if !events.is_empty() {
                for event in &events {
                    tracing::debug!(?event, "Received backend event");
                }
                ctx.request_repaint();
            }
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.heading("Easy NATS");
                ui.label("NATS Management Tool");
            });
        }
    }
}
