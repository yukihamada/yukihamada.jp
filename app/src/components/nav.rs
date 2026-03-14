use leptos::prelude::*;

use crate::i18n::{translate, use_locale, Locale};
use super::theme_toggle::ThemeToggle;

#[component]
pub fn Nav() -> impl IntoView {
    let locale = use_locale();
    let menu_open = RwSignal::new(false);
    let lang_open = RwSignal::new(false);

    let toggle_menu = move |_| menu_open.update(|v| *v = !*v);
    let toggle_lang = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        lang_open.update(|v| *v = !*v);
    };

    view! {
        <nav class="nav" id="main-nav">
            <div class="container nav-inner">
                <a href={move || format!("/{}", locale.get().code())} class="nav-brand">
                    <span class="gradient-text">"YH"</span>
                </a>
                <button class="nav-toggle" aria-label="Toggle menu" on:click=toggle_menu>
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M3 12h18M3 6h18M3 18h18"/>
                    </svg>
                </button>
                <div class="nav-links" class:open=move || menu_open.get()>
                    <a href="#work">{move || translate(locale.get(), "nav.work")}</a>
                    <a href="#career">{move || translate(locale.get(), "nav.career")}</a>
                    <a href={move || format!("/{}/music", locale.get().code())}>{move || translate(locale.get(), "nav.music")}</a>
                    <a href={move || format!("/{}/blog", locale.get().code())}>{move || translate(locale.get(), "nav.blog")}</a>
                    <ThemeToggle />
                    <div class="nav-lang-wrap">
                        <button class="nav-lang" on:click=toggle_lang>
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width:14px;height:14px;margin-right:4px;">
                                <circle cx="12" cy="12" r="10"/>
                                <path d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"/>
                            </svg>
                            {move || locale.get().label()}
                        </button>
                        <div class="nav-lang-dropdown" class:open=move || lang_open.get()>
                            {Locale::ALL.iter().map(|&l| {
                                let code = l.code();
                                let label = l.label();
                                view! {
                                    <a
                                        href=format!("/{code}")
                                        class="nav-lang-option"
                                        class:active=move || locale.get() == l
                                    >
                                        {label}
                                    </a>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    }
}
