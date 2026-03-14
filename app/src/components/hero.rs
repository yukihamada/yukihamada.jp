use leptos::prelude::*;

use crate::i18n::{translate, use_locale};

#[cfg(target_arch = "wasm32")]
use crate::i18n::Locale;

#[cfg(target_arch = "wasm32")]
use crate::wasm_utils::sleep_ms;

#[cfg(target_arch = "wasm32")]
use std::cell::Cell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
const ROLES_JA: &[&str] = &[
    "ポーカープレイヤー",
    "柔術家",
    "愛犬家",
    "ギタリスト",
    "アーティスト",
    "起業家",
    "エンジェル投資家",
];

#[cfg(target_arch = "wasm32")]
const ROLES_EN: &[&str] = &[
    "Poker Player",
    "BJJ Practitioner",
    "Dog Lover",
    "Guitarist",
    "Artist",
    "Entrepreneur",
    "Angel Investor",
];

use super::particle_canvas::ParticleCanvas;
use super::shader_bg::ShaderBg;
use super::typing_game::TypingGame;

#[component]
pub fn Hero() -> impl IntoView {
    let locale = use_locale();
    let typed_text = RwSignal::new(String::new());
    let show_typing_game = RwSignal::new(false);

    #[cfg(target_arch = "wasm32")]
    {
        let cancel = Rc::new(Cell::new(false));

        Effect::new(move |prev: Option<Rc<Cell<bool>>>| {
            if let Some(old_cancel) = prev {
                old_cancel.set(true);
            }

            let new_cancel = Rc::new(Cell::new(false));
            let cancel_flag = new_cancel.clone();
            let current_locale = locale.get();

            let roles: Vec<String> = match current_locale {
                Locale::Ja => ROLES_JA.iter().map(|s| s.to_string()).collect(),
                _ => ROLES_EN.iter().map(|s| s.to_string()).collect(),
            };

            wasm_bindgen_futures::spawn_local(async move {
                loop {
                    for role in &roles {
                        let chars: Vec<char> = role.chars().collect();
                        for i in 1..=chars.len() {
                            if cancel_flag.get() { return; }
                            typed_text.set(chars[..i].iter().collect());
                            sleep_ms(100).await;
                        }
                        if cancel_flag.get() { return; }
                        sleep_ms(1800).await;

                        let len = chars.len();
                        for i in (0..len).rev() {
                            if cancel_flag.get() { return; }
                            typed_text.set(chars[..i].iter().collect());
                            sleep_ms(50).await;
                        }
                        if cancel_flag.get() { return; }
                        sleep_ms(400).await;
                    }
                }
            });

            new_cancel
        });

        drop(cancel);

        // Scroll morphing on avatar
        Effect::new(move |_| {
            use wasm_bindgen::prelude::*;
            let scroll_cb = Closure::<dyn FnMut()>::new(move || {
                let win = web_sys::window().unwrap();
                let scroll_y = win.scroll_y().unwrap_or(0.0);
                let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(800.0);
                let progress = (scroll_y / vh).min(1.0);

                if let Some(doc) = win.document() {
                    if let Ok(Some(avatar)) = doc.query_selector(".hero-avatar-wrap") {
                        let el: &web_sys::HtmlElement = avatar.unchecked_ref();
                        let scale = 1.0 - progress * 0.08;
                        let opacity = 1.0 - progress * 0.5;
                        let _ = el.style().set_property("transform", &format!("scale({scale:.3})"));
                        let _ = el.style().set_property("opacity", &format!("{opacity:.3}"));
                    }
                }
            });
            if let Some(win) = web_sys::window() {
                let _ = win.add_event_listener_with_callback("scroll", scroll_cb.as_ref().unchecked_ref());
                scroll_cb.forget();
            }
        });
    }

    let on_role_click = move |_| {
        show_typing_game.set(true);
    };

    view! {
        <section class="hero" id="top">
            <ShaderBg />
            <ParticleCanvas />
            <div class="hero-content">
                <div class="hero-avatar-wrap">
                    <div class="hero-avatar">
                        <img
                            src="/assets/yuki-profile.jpg"
                            alt="濱田優貴"
                            width="160"
                            height="160"
                            style="width:100%;height:100%;object-fit:cover;border-radius:50%;"
                        />
                    </div>
                    <div class="hero-avatar-status"></div>
                </div>
                <h1>
                    {move || translate(locale.get(), "hero.name_first")}
                    " "
                    <span class="gradient-text">{move || translate(locale.get(), "hero.name_last")}</span>
                </h1>
                <p class="hero-sub-name">{move || translate(locale.get(), "hero.name_sub")}</p>
                <div class="hero-role" style="cursor:pointer;" on:click=on_role_click title="Click to play typing game!">
                    <span id="typed-role">{move || typed_text.get()}</span>
                    <span class="cursor"></span>
                </div>
                <p class="hero-subtitle">{move || translate(locale.get(), "hero.subtitle")}</p>
                <div class="hero-buttons">
                    <a href="#work" class="hero-cta">
                        {move || translate(locale.get(), "hero.cta")}
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M6 9l6 6 6-6"/>
                        </svg>
                    </a>
                </div>
            </div>
            <div class="scroll-indicator">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
            </div>
            <TypingGame show=show_typing_game />
        </section>
    }
}
