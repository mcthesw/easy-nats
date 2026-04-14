#![windows_subsystem = "windows"]

mod app;
mod format;
mod i18n;
mod log_layer;
mod proto;
mod settings;
mod tabs;
mod toast;

/// Re-attach to the parent console so log output appears in the terminal
/// when the application is launched from cmd / PowerShell.
#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    unsafe extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
    }
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    #[cfg(windows)]
    attach_parent_console();

    let log_buffer = log_layer::SharedLogBuffer::default();
    {
        use tracing_subscriber::prelude::*;
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer().with_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                ),
            )
            .with(log_layer::AppLogLayer::new(log_buffer.clone()))
            .init();
    }

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
        Box::new(move |cc| {
            setup_fonts(&cc.egui_ctx);
            let dark = cc
                .egui_ctx
                .system_theme()
                .map(|t| t == eframe::egui::Theme::Dark)
                .unwrap_or(app_settings.dark_mode);
            apply_theme(&cc.egui_ctx, dark);
            Ok(Box::new(app::EasyNatsApp::new(dark, log_buffer)))
        }),
    ) {
        tracing::error!("Failed to start application: {e}");
    }
}

fn setup_fonts(ctx: &eframe::egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "Inter".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "LXGWNeoXiHei".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/LXGWNeoXiHei-Regular.ttf"
        ))),
    );

    // Inter as primary, LXGW as CJK fallback; keep egui defaults (emoji-icon-font, NotoEmoji) after
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "LXGWNeoXiHei".to_owned());
    proportional.insert(0, "Inter".to_owned());

    ctx.set_fonts(fonts);
}

pub(crate) fn apply_theme(ctx: &eframe::egui::Context, dark: bool) {
    use eframe::egui::Visuals;
    if dark {
        ctx.set_visuals(Visuals::dark());
    } else {
        ctx.set_visuals(Visuals::light());
    }
}
