mod app;
mod format;
mod i18n;
mod keyboard;
mod log_layer;
mod proto;
mod runtime;
mod schema;
mod settings;
mod tabs;
mod theme;
mod toast;

/// Run the native Easy NATS desktop application.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    #[cfg(windows)]
    attach_parent_console();

    let paths = nats_backend::ProjectPaths::resolve();
    nats_backend::migrate_legacy_on_startup(&paths);

    let log_buffer = log_layer::SharedLogBuffer::default();
    let _log_guard = install_tracing(&paths, log_buffer.clone());

    let app_settings = settings::AppSettings::load();
    i18n::init(app_settings.language);

    let viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);
    #[cfg(target_os = "macos")]
    let viewport = viewport.with_icon(eframe::egui::viewport::IconData::default());
    #[cfg(not(target_os = "macos"))]
    let viewport = viewport.with_icon(load_app_icon());

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "Easy NATS",
        native_options,
        Box::new(move |creation_context| {
            creation_context
                .egui_ctx
                .options_mut(|options| options.reduce_texture_memory = true);
            setup_fonts(&creation_context.egui_ctx);
            let theme_id = app_settings.resolved_theme(
                creation_context
                    .egui_ctx
                    .system_theme()
                    .map(|theme| theme == eframe::egui::Theme::Dark),
            );
            theme::apply_theme(&creation_context.egui_ctx, theme_id);
            Ok(Box::new(app::EasyNatsApp::new(
                app_settings,
                theme_id,
                log_buffer,
            )))
        }),
    ) {
        tracing::error!("Failed to start application: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
    egui_ctx: Rc<RefCell<Option<eframe::egui::Context>>>,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            runner: eframe::WebRunner::new(),
            egui_ctx: Rc::new(RefCell::new(None)),
        }
    }

    /// Start the in-browser interactive demo.
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        language: &str,
        theme_id: &str,
    ) -> Result<(), JsValue> {
        let language = web_language(language)?;
        let theme_id = web_theme(theme_id)?;
        let mut settings = settings::AppSettings {
            language,
            theme: Some(theme_id),
            ..Default::default()
        };
        i18n::init(language);
        let egui_ctx = Rc::clone(&self.egui_ctx);

        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |creation_context| {
                    creation_context
                        .egui_ctx
                        .options_mut(|options| options.reduce_texture_memory = true);
                    setup_fonts(&creation_context.egui_ctx);
                    theme::apply_theme(&creation_context.egui_ctx, theme_id);
                    *egui_ctx.borrow_mut() = Some(creation_context.egui_ctx.clone());
                    settings.theme = Some(theme_id);
                    Ok(Box::new(app::EasyNatsApp::new_demo(settings, theme_id)))
                }),
            )
            .await
    }

    pub fn language(&self) -> Option<String> {
        self.runner
            .app_mut::<app::EasyNatsApp>()
            .map(|app| web_language_id(app.demo_language()).to_owned())
    }

    pub fn set_language(&self, language: &str) -> Result<(), JsValue> {
        let language = web_language(language)?;
        let mut app = self
            .runner
            .app_mut::<app::EasyNatsApp>()
            .ok_or_else(|| JsValue::from_str("Interactive Demo is not running"))?;
        app.apply_demo_language(language);
        drop(app);
        self.request_repaint();
        Ok(())
    }

    pub fn theme(&self) -> Option<String> {
        self.runner
            .app_mut::<app::EasyNatsApp>()
            .map(|app| web_theme_id(app.demo_theme()).to_owned())
    }

    pub fn set_theme(&self, theme_id: &str) -> Result<(), JsValue> {
        let theme_id = web_theme(theme_id)?;
        let mut app = self
            .runner
            .app_mut::<app::EasyNatsApp>()
            .ok_or_else(|| JsValue::from_str("Interactive Demo is not running"))?;
        app.apply_demo_theme(theme_id);
        drop(app);

        if let Some(ctx) = self.egui_ctx.borrow().as_ref() {
            theme::apply_theme(ctx, theme_id);
        }
        self.request_repaint();
        Ok(())
    }

    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|summary| summary.message())
    }

    fn request_repaint(&self) {
        if let Some(ctx) = self.egui_ctx.borrow().as_ref() {
            ctx.request_repaint();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn web_language(value: &str) -> Result<i18n::Language, JsValue> {
    match value {
        "en" => Ok(i18n::Language::En),
        "zh" => Ok(i18n::Language::Zh),
        _ => Err(JsValue::from_str("Unsupported Easy NATS language")),
    }
}

#[cfg(target_arch = "wasm32")]
fn web_language_id(language: i18n::Language) -> &'static str {
    match language {
        i18n::Language::En => "en",
        i18n::Language::Zh => "zh",
    }
}

#[cfg(target_arch = "wasm32")]
fn web_theme(value: &str) -> Result<theme::ThemeId, JsValue> {
    match value {
        "egui-dark" => Ok(theme::ThemeId::EguiDark),
        "egui-light" => Ok(theme::ThemeId::EguiLight),
        "catppuccin-latte" => Ok(theme::ThemeId::CatppuccinLatte),
        "catppuccin-frappe" => Ok(theme::ThemeId::CatppuccinFrappe),
        "catppuccin-macchiato" => Ok(theme::ThemeId::CatppuccinMacchiato),
        "catppuccin-mocha" => Ok(theme::ThemeId::CatppuccinMocha),
        _ => Err(JsValue::from_str("Unsupported Easy NATS theme")),
    }
}

#[cfg(target_arch = "wasm32")]
fn web_theme_id(theme_id: theme::ThemeId) -> &'static str {
    match theme_id {
        theme::ThemeId::EguiDark => "egui-dark",
        theme::ThemeId::EguiLight => "egui-light",
        theme::ThemeId::CatppuccinLatte => "catppuccin-latte",
        theme::ThemeId::CatppuccinFrappe => "catppuccin-frappe",
        theme::ThemeId::CatppuccinMacchiato => "catppuccin-macchiato",
        theme::ThemeId::CatppuccinMocha => "catppuccin-mocha",
    }
}

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

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
fn load_app_icon() -> eframe::egui::viewport::IconData {
    let bytes = include_bytes!("../../../assets/icons/easy-nats-256.png");
    let image = image::load_from_memory(bytes).expect("valid PNG icon");
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    eframe::egui::viewport::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
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

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "LXGWNeoXiHei".to_owned());
    proportional.insert(0, "Inter".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("LXGWNeoXiHei".to_owned());

    ctx.set_fonts(fonts);
}

#[cfg(not(target_arch = "wasm32"))]
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
            tracing_subscriber::registry().with(mem_layer).init();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn setup_fonts_supports_cjk_in_monospace() {
        let ctx = eframe::egui::Context::default();
        super::setup_fonts(&ctx);
        let _ = ctx.run_ui(eframe::egui::RawInput::default(), |_| {});
        assert!(ctx.fonts_mut(|fonts| {
            fonts.has_glyphs(&eframe::egui::FontId::monospace(13.0), "中文")
        }));
    }
}
