use leptos::prelude::*;

use crate::i18n::{translate, use_locale};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::wasm_utils::animation_loop;

struct StatItem {
    target: f64,
    suffix: &'static str,
    label_key: &'static str,
}

const STATS: &[StatItem] = &[
    StatItem {
        target: 10.0,
        suffix: "+",
        label_key: "stats.products",
    },
    StatItem {
        target: 20.0,
        suffix: "+",
        label_key: "stats.years",
    },
    StatItem {
        target: 1800.0,
        suffix: "+",
        label_key: "stats.languages",
    },
    StatItem {
        target: 100.0,
        suffix: "%",
        label_key: "stats.rust",
    },
];

#[component]
fn AnimatedNumber(target: f64, suffix: &'static str) -> impl IntoView {
    let display = RwSignal::new(String::from("0"));
    let el_ref = NodeRef::<leptos::html::Span>::new();

    #[cfg(target_arch = "wasm32")]
    {
        let suffix_owned = suffix.to_string();

        Effect::new(move |_| {
            let el = match el_ref.get() {
                Some(e) => e,
                None => return,
            };

            let suffix = suffix_owned.clone();
            let el_node: web_sys::Element = el.into();

            let started = std::rc::Rc::new(std::cell::Cell::new(false));
            let start_time = std::rc::Rc::new(std::cell::Cell::new(0.0f64));
            let handle: std::rc::Rc<std::cell::RefCell<Option<crate::wasm_utils::AnimationHandle>>> =
                std::rc::Rc::new(std::cell::RefCell::new(None));

            let started_clone = started.clone();
            let start_time_clone = start_time.clone();
            let handle_clone = handle.clone();

            let observer_cb = Closure::<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>::new(
                move |entries: js_sys::Array, observer: web_sys::IntersectionObserver| {
                    for i in 0..entries.length() {
                        if let Ok(entry) = entries.get(i).dyn_into::<web_sys::IntersectionObserverEntry>() {
                            if entry.is_intersecting() && !started_clone.get() {
                                started_clone.set(true);
                                observer.unobserve(&entry.target());

                                let suffix = suffix.clone();
                                let start_time = start_time_clone.clone();
                                let duration = 1500.0; // 1.5s

                                let anim = animation_loop(move |timestamp| {
                                    if start_time.get() == 0.0 {
                                        start_time.set(timestamp);
                                    }
                                    let elapsed = timestamp - start_time.get();
                                    let progress = (elapsed / duration).min(1.0);

                                    // easeOutExpo
                                    let eased = if progress >= 1.0 {
                                        1.0
                                    } else {
                                        1.0 - 2.0f64.powf(-10.0 * progress)
                                    };

                                    let current = (target * eased).round() as i64;
                                    display.set(format!("{current}{suffix}"));
                                });

                                *handle_clone.borrow_mut() = Some(anim);
                            }
                        }
                    }
                },
            );

            let opts = web_sys::IntersectionObserverInit::new();
            opts.set_threshold(&JsValue::from_f64(0.3));

            if let Ok(observer) = web_sys::IntersectionObserver::new_with_options(
                observer_cb.as_ref().unchecked_ref(),
                &opts,
            ) {
                observer.observe(&el_node);
                observer_cb.forget();
            }
        });
    }

    // SSR fallback: show final value
    #[cfg(not(target_arch = "wasm32"))]
    {
        let val = target as i64;
        display.set(format!("{val}{suffix}"));
    }

    view! {
        <span node_ref=el_ref class="stat-number gradient-text">
            {move || display.get()}
        </span>
    }
}

#[component]
pub fn Stats() -> impl IntoView {
    let locale = use_locale();

    view! {
        <section class="section stats-section" id="stats">
            <div class="container">
                <div class="stats-grid">
                    {STATS.iter().map(|stat| {
                        let label_key = stat.label_key;
                        view! {
                            <div class="stat-item fade-in">
                                <AnimatedNumber target=stat.target suffix=stat.suffix />
                                <span class="stat-label">
                                    {move || translate(locale.get(), label_key)}
                                </span>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </section>
    }
}
