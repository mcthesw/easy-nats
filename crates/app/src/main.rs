#![windows_subsystem = "windows"]

mod app;
mod format;
mod i18n;
mod log_layer;
mod proto;
mod settings;
mod tabs;
mod theme;
mod toast;

/// Attach to the parent console when launched from a terminal.
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

    // Migrate legacy files before loading app state.
    let paths = nats_backend::ProjectPaths::resolve();
    nats_backend::migrate_legacy_on_startup(&paths);

    let log_buffer = log_layer::SharedLogBuffer::default();
    // Keep the appender guard alive so buffered logs flush on exit.
    let _log_guard = install_tracing(&paths, log_buffer.clone());

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
            let theme_id = app_settings.resolved_theme(
                cc.egui_ctx
                    .system_theme()
                    .map(|t| t == eframe::egui::Theme::Dark),
            );
            theme::apply_theme(&cc.egui_ctx, theme_id);
            Ok(Box::new(app::EasyNatsApp::new(
                app_settings,
                theme_id,
                log_buffer,
            )))
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

/// Initialise the tracing subscriber.
///
/// When any of the standard I/O streams is attached to a terminal (interactive
/// terminal launch, `cargo run`, CI logs) the formatted layer writes to
/// stderr so developers and power users see log output immediately. When no
/// stream is a terminal — typical for packaged GUI launches through
/// Start-Menu shortcuts, `.app` bundles, or `.desktop` entries — the
/// formatted layer is redirected to a daily-rolled file under the platform
/// log directory instead, so release builds never keep a terminal window
/// visible by writing to stdout/stderr.
///
/// The returned `WorkerGuard` MUST be held for the lifetime of the process so
/// buffered records are flushed on exit; the caller binds it to `_log_guard`.
fn install_tracing(
    paths: &nats_backend::ProjectPaths,
    log_buffer: log_layer::SharedLogBuffer,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use std::io::IsTerminal;
    use tracing_subscriber::prelude::*;

    let env_filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };
    let mem_layer = log_layer::AppLogLayer::new(log_buffer);

    let has_terminal = std::io::stderr().is_terminal() || std::io::stdout().is_terminal();

    if has_terminal {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_filter(env_filter()),
            )
            .with(mem_layer)
            .init();
        return None;
    }

    match std::fs::create_dir_all(paths.log_dir()) {
        Ok(()) => {
            let appender = tracing_appender::rolling::daily(paths.log_dir(), "easy-nats.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(appender);
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(non_blocking)
                        .with_filter(env_filter()),
                )
                .with(mem_layer)
                .init();
            Some(guard)
        }
        Err(_) => {
            // No terminal and no writable log dir — install only the in-memory
            // layer so the UI still sees events and no output lands on stdio.
            tracing_subscriber::registry().with(mem_layer).init();
            None
        }
    }
}
