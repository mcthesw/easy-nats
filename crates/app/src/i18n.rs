//! YAML-based internationalization with runtime language switching.
//!
//! Translations are embedded via `include_str!` from `assets/i18n/*.yaml`.
//! Each YAML file is a flat map of `key: {en: "...", zh: "..."}`.
//! Keys are namespaced by filename: `t("sidebar.connections")`.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

// ── Language enum ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Zh,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::En, Language::Zh];

    pub fn label(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Zh => "中文",
        }
    }

    fn index(self) -> u8 {
        match self {
            Language::En => 0,
            Language::Zh => 1,
        }
    }

    fn from_index(idx: u8) -> Self {
        match idx {
            1 => Language::Zh,
            _ => Language::En,
        }
    }
}

// ── Translation store ──

#[derive(Debug)]
struct Translations {
    /// language-index → flat-key → translated text
    maps: [HashMap<String, String>; 2],
}

static STORE: OnceLock<Translations> = OnceLock::new();
static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

/// Initialise the i18n system. Must be called once before any `t()` calls.
pub fn init(lang: Language) {
    let yaml_sources: &[(&str, &str)] = &[
        ("common", include_str!("../../../assets/i18n/common.yaml")),
        ("sidebar", include_str!("../../../assets/i18n/sidebar.yaml")),
        (
            "connection",
            include_str!("../../../assets/i18n/connection.yaml"),
        ),
        (
            "publisher",
            include_str!("../../../assets/i18n/publisher.yaml"),
        ),
        (
            "subscriber",
            include_str!("../../../assets/i18n/subscriber.yaml"),
        ),
        ("stream", include_str!("../../../assets/i18n/stream.yaml")),
        (
            "consumer",
            include_str!("../../../assets/i18n/consumer.yaml"),
        ),
        ("kv", include_str!("../../../assets/i18n/kv.yaml")),
        (
            "obj_store",
            include_str!("../../../assets/i18n/obj_store.yaml"),
        ),
        ("toast", include_str!("../../../assets/i18n/toast.yaml")),
        (
            "settings",
            include_str!("../../../assets/i18n/settings.yaml"),
        ),
        (
            "log_viewer",
            include_str!("../../../assets/i18n/log_viewer.yaml"),
        ),
        (
            "server_info",
            include_str!("../../../assets/i18n/server_info.yaml"),
        ),
        ("metrics", include_str!("../../../assets/i18n/metrics.yaml")),
    ];

    let mut en = HashMap::new();
    let mut zh = HashMap::new();

    for (namespace, content) in yaml_sources {
        let parsed: HashMap<String, HashMap<String, String>> = serde_yaml::from_str(content)
            .unwrap_or_else(|e| {
                panic!("Failed to parse i18n file {namespace}.yaml: {e}");
            });
        for (key, langs) in parsed {
            let full_key = format!("{namespace}.{key}");
            if let Some(v) = langs.get("en") {
                en.insert(full_key.clone(), v.clone());
            }
            if let Some(v) = langs.get("zh") {
                zh.insert(full_key, v.clone());
            }
        }
    }

    STORE
        .set(Translations { maps: [en, zh] })
        .expect("i18n already initialised");
    set_language(lang);
}

/// Look up a translated string for the current language.
/// Falls back to English, then returns `"???"` if missing entirely.
///
/// Returns `&'static str` because translations live in a `static OnceLock`.
pub fn t(key: &str) -> &'static str {
    let Some(store) = STORE.get() else {
        return "???";
    };
    let lang = CURRENT_LANG.load(Ordering::Relaxed) as usize;
    if let Some(v) = store.maps[lang].get(key) {
        return v.as_str();
    }
    // Fallback to English
    if let Some(v) = store.maps[0].get(key) {
        return v.as_str();
    }
    tracing::warn!("Missing i18n key: {key}");
    "???"
}

/// Switch the active language. Takes effect on the next `t()` call.
pub fn set_language(lang: Language) {
    CURRENT_LANG.store(lang.index(), Ordering::Relaxed);
}

/// Return the currently active language.
pub fn current_language() -> Language {
    Language::from_index(CURRENT_LANG.load(Ordering::Relaxed))
}
