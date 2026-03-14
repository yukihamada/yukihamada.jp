use leptos::prelude::*;
use crate::i18n::{translate, use_locale};

const TEXTS_EN: &[&str] = &[
    "Founder and CEO of Enabler Inc.",
    "Former CPO and Director at Mercari",
    "Building the future with Rust and WASM",
    "Angel investor and serial entrepreneur",
    "Co-founded NOT A HOTEL",
];

const TEXTS_JA: &[&str] = &[
    "株式会社イネブラ 代表取締役CEO",
    "元メルカリ取締役CPO",
    "RustとWASMで未来を創る",
    "エンジェル投資家・連続起業家",
    "NOT A HOTEL共同創業者",
];

#[component]
pub fn TypingGame(
    #[prop(into)] show: RwSignal<bool>,
) -> impl IntoView {
    let locale = use_locale();
    let target_text = RwSignal::new(String::new());
    let typed = RwSignal::new(String::new());
    let started = RwSignal::new(false);
    let finished = RwSignal::new(false);
    let start_time = RwSignal::new(0.0f64);
    let wpm = RwSignal::new(0.0f64);
    let accuracy = RwSignal::new(100.0f64);

    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Pick random text when shown
    Effect::new(move |_| {
        if show.get() {
            #[cfg(target_arch = "wasm32")]
            {
                use crate::i18n::Locale;
                let texts = match locale.get() {
                    Locale::Ja => TEXTS_JA,
                    _ => TEXTS_EN,
                };
                let idx = (js_sys::Math::random() * texts.len() as f64) as usize;
                target_text.set(texts[idx.min(texts.len() - 1)].to_string());
                typed.set(String::new());
                started.set(false);
                finished.set(false);
                wpm.set(0.0);
                accuracy.set(100.0);

                // Focus input after render
                wasm_bindgen_futures::spawn_local(async move {
                    crate::wasm_utils::sleep_ms(100).await;
                    if let Some(el) = input_ref.get() {
                        let _ = el.focus();
                    }
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                target_text.set(TEXTS_EN[0].to_string());
            }
        }
    });

    let on_input = move |ev: leptos::ev::Event| {
        let val: String = event_target_value(&ev);

        if !started.get() {
            started.set(true);
            #[cfg(target_arch = "wasm32")]
            {
                start_time.set(js_sys::Date::now());
            }
        }

        typed.set(val.clone());

        let target = target_text.get_untracked();
        if val.len() >= target.len() {
            finished.set(true);

            #[cfg(target_arch = "wasm32")]
            {
                let elapsed_sec = (js_sys::Date::now() - start_time.get_untracked()) / 1000.0;
                let words = target.split_whitespace().count() as f64;
                let w = if elapsed_sec > 0.0 { words / elapsed_sec * 60.0 } else { 0.0 };
                wpm.set(w);

                let correct = val.chars().zip(target.chars())
                    .filter(|(a, b)| a == b).count();
                let acc = correct as f64 / target.len().max(1) as f64 * 100.0;
                accuracy.set(acc);

                // Unlock achievement
                crate::components::achievements::unlock_achievement("typist");
            }
        }
    };

    let close = move |_| {
        show.set(false);
    };

    view! {
        <Show when=move || show.get()>
            <div class="typing-game-overlay" on:click=close>
                <div class="typing-game-modal" on:click=|e: leptos::ev::MouseEvent| e.stop_propagation()>
                    <button class="typing-game-close" on:click=close>"✕"</button>
                    <h3>{move || translate(locale.get(), "typing.title")}</h3>

                    <div class="typing-target">
                        {move || {
                            let target = target_text.get();
                            let input = typed.get();
                            target.chars().enumerate().map(|(i, ch)| {
                                let class = if i < input.len() {
                                    if input.chars().nth(i) == Some(ch) { "char-correct" } else { "char-wrong" }
                                } else if i == input.len() {
                                    "char-current"
                                } else {
                                    "char-pending"
                                };
                                view! { <span class=class>{ch.to_string()}</span> }
                            }).collect::<Vec<_>>()
                        }}
                    </div>

                    <Show when=move || !finished.get()>
                        <input
                            node_ref=input_ref
                            type="text"
                            class="typing-input"
                            autocomplete="off"
                            autocapitalize="off"
                            spellcheck="false"
                            on:input=on_input
                            prop:value=move || typed.get()
                        />
                    </Show>

                    <Show when=move || finished.get()>
                        <div class="typing-result">
                            <div class="typing-stat">
                                <span class="stat-number gradient-text">
                                    {move || format!("{:.0}", wpm.get())}
                                </span>
                                <span class="stat-label">"WPM"</span>
                            </div>
                            <div class="typing-stat">
                                <span class="stat-number gradient-text">
                                    {move || format!("{:.0}%", accuracy.get())}
                                </span>
                                <span class="stat-label">{move || translate(locale.get(), "typing.accuracy")}</span>
                            </div>
                        </div>
                        <button class="hero-cta" style="margin-top:16px;" on:click=move |_| {
                            show.set(true); // re-trigger effect
                            typed.set(String::new());
                            finished.set(false);
                            started.set(false);
                        }>
                            {move || translate(locale.get(), "typing.retry")}
                        </button>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
