use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::nav::Nav;
use super::footer::Footer;
use super::scroll_observer::ScrollObserver;
use super::mouse_trail::MouseTrail;
use super::konami::KonamiListener;
use super::achievements::AchievementTracker;
use super::terminal::Terminal;
use super::chat_widget::ChatWidget;
use crate::i18n::{use_locale, Locale};

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let locale = use_locale();
    let location = use_location();

    let path = location.pathname.get_untracked();
    if let Some(lang_code) = path.split('/').nth(1) {
        if let Some(l) = Locale::from_code(lang_code) {
            locale.set(l);
        }
    }

    let lang_script = format!(
        "document.documentElement.lang='{}';",
        locale.get_untracked().code()
    );

    let terminal_show = RwSignal::new(false);

    // Daily color scheme: shift hue based on current hour
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        Effect::new(move |_| {
            let hour = (js_sys::Date::new_0().get_hours() as f64) / 24.0;
            let hue_shift = (hour * 20.0 - 10.0) as i32;
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(root) = doc.document_element() {
                    let el: &web_sys::HtmlElement = root.dyn_ref().unwrap();
                    let base_hue = 22 + hue_shift;
                    let _ = el.style().set_property("--primary", &format!("hsl({base_hue} 95% 36%)"));
                    let _ = el.style().set_property("--primary-light", &format!("hsl({} 90% 48%)", base_hue + 4));
                    let _ = el.style().set_property("--primary-dim", &format!("hsl({} 80% 24%)", base_hue - 6));
                }
            }
        });

        // Global `/` key listener to open terminal
        Effect::new(move |_| {
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                // Don't trigger when typing in inputs/textareas
                if let Some(target) = e.target() {
                    if let Some(el) = target.dyn_ref::<web_sys::HtmlElement>() {
                        let tag = el.tag_name().to_lowercase();
                        if tag == "input" || tag == "textarea" || el.is_content_editable() {
                            return;
                        }
                    }
                }
                if e.key() == "/" && !e.ctrl_key() && !e.meta_key() {
                    e.prevent_default();
                    terminal_show.update(|v| *v = !*v);
                }
            });
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let target: &web_sys::EventTarget = doc.unchecked_ref();
                let _ = target.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                cb.forget();
            }
        });
    }

    view! {
        <script>{lang_script}</script>
        <ScrollObserver />
        <MouseTrail />
        <KonamiListener />
        <AchievementTracker />
        <Terminal show=terminal_show />
        <Nav />
        <main>{children()}</main>
        <Footer />
        <ChatWidget />
    }
}
