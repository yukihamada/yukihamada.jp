use std::collections::HashMap;
use std::sync::OnceLock;

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    Ja,
    En,
    Zh,
    Ko,
    Es,
    Fr,
}

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Locale::Ja => "ja",
            Locale::En => "en",
            Locale::Zh => "zh",
            Locale::Ko => "ko",
            Locale::Es => "es",
            Locale::Fr => "fr",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Locale::Ja => "日本語",
            Locale::En => "English",
            Locale::Zh => "中文",
            Locale::Ko => "한국어",
            Locale::Es => "Español",
            Locale::Fr => "Français",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "ja" => Some(Locale::Ja),
            "en" => Some(Locale::En),
            "zh" => Some(Locale::Zh),
            "ko" => Some(Locale::Ko),
            "es" => Some(Locale::Es),
            "fr" => Some(Locale::Fr),
            _ => None,
        }
    }

    /// Cycle to the next locale.
    pub fn next(self) -> Self {
        let all = Self::ALL;
        let idx = all.iter().position(|&l| l == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub const ALL: [Locale; 6] = [
        Locale::Ja,
        Locale::En,
        Locale::Zh,
        Locale::Ko,
        Locale::Es,
        Locale::Fr,
    ];
}

type TranslationMap = HashMap<String, String>;

static JA: OnceLock<TranslationMap> = OnceLock::new();
static EN: OnceLock<TranslationMap> = OnceLock::new();
static ZH: OnceLock<TranslationMap> = OnceLock::new();
static KO: OnceLock<TranslationMap> = OnceLock::new();
static ES: OnceLock<TranslationMap> = OnceLock::new();
static FR: OnceLock<TranslationMap> = OnceLock::new();

fn load(lock: &'static OnceLock<TranslationMap>, json: &str) -> &'static TranslationMap {
    lock.get_or_init(|| serde_json::from_str(json).unwrap())
}

fn get_map(locale: Locale) -> &'static TranslationMap {
    match locale {
        Locale::Ja => load(&JA, include_str!("ja.json")),
        Locale::En => load(&EN, include_str!("en.json")),
        Locale::Zh => load(&ZH, include_str!("zh.json")),
        Locale::Ko => load(&KO, include_str!("ko.json")),
        Locale::Es => load(&ES, include_str!("es.json")),
        Locale::Fr => load(&FR, include_str!("fr.json")),
    }
}

/// Look up a translation key. Falls back to English, then Japanese, then the key itself.
pub fn translate(locale: Locale, key: &str) -> String {
    if let Some(val) = get_map(locale).get(key) {
        return val.clone();
    }
    if locale != Locale::En {
        if let Some(val) = get_map(Locale::En).get(key) {
            return val.clone();
        }
    }
    if locale != Locale::Ja {
        if let Some(val) = get_map(Locale::Ja).get(key) {
            return val.clone();
        }
    }
    key.to_string()
}

pub fn provide_i18n() {
    let locale = RwSignal::new(Locale::Ja);
    provide_context(locale);
}

pub fn use_locale() -> RwSignal<Locale> {
    use_context::<RwSignal<Locale>>().expect("i18n context not provided")
}

/// Reactive translation closure.
pub fn t(key: &str) -> impl Fn() -> String + Clone + Send + Sync + 'static {
    let locale = use_locale();
    let key = key.to_string();
    move || translate(locale.get(), &key)
}

pub fn set_locale(new_locale: Locale) {
    let locale = use_locale();
    locale.set(new_locale);
    #[cfg(target_arch = "wasm32")]
    {
        browser::save_locale(new_locale);
        browser::update_html_lang(new_locale);
    }
}

#[cfg(target_arch = "wasm32")]
pub mod browser {
    use super::Locale;

    pub fn detect_locale() -> Option<Locale> {
        let window = web_sys::window()?;
        let lang = window.navigator().language()?;
        let code = lang.split('-').next().unwrap_or(&lang);
        Locale::from_code(code)
    }

    pub fn get_saved_locale() -> Option<Locale> {
        let storage = web_sys::window()?.local_storage().ok()??;
        let code = storage.get_item("locale").ok()??;
        Locale::from_code(&code)
    }

    pub fn save_locale(locale: Locale) {
        if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
            let _ = storage.set_item("locale", locale.code());
        }
    }

    pub fn update_html_lang(locale: Locale) {
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = el.set_attribute("lang", locale.code());
        }
    }
}
