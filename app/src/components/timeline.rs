use leptos::prelude::*;

use crate::data::timeline::TIMELINE;
use crate::i18n::{translate, use_locale};

#[component]
pub fn Timeline() -> impl IntoView {
    let locale = use_locale();

    // Scroll-based parallax effect
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;

        Effect::new(move |_| {
            let scroll_cb = Closure::<dyn FnMut()>::new(move || {
                let win = match web_sys::window() {
                    Some(w) => w,
                    None => return,
                };
                let doc = match win.document() {
                    Some(d) => d,
                    None => return,
                };
                let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(800.0);
                let center_y = win.scroll_y().unwrap_or(0.0) + vh * 0.5;

                if let Ok(items) = doc.query_selector_all(".timeline-item") {
                    for i in 0..items.length() {
                        if let Some(node) = items.get(i) {
                            let el: &web_sys::HtmlElement = node.unchecked_ref();
                            let rect = el.get_bounding_client_rect();
                            let item_center = rect.top() + win.scroll_y().unwrap_or(0.0) + rect.height() * 0.5;
                            let dist = (center_y - item_center).abs();
                            let max_dist = vh * 0.8;
                            let progress = (dist / max_dist).min(1.0);

                            let scale = 1.0 - progress * 0.05;
                            let translate_x = if i % 2 == 0 { progress * -8.0 } else { progress * 8.0 };
                            let opacity = 1.0 - progress * 0.3;

                            let _ = el.style().set_property("transform",
                                &format!("scale({scale:.3}) translateX({translate_x:.1}px)"));
                            let _ = el.style().set_property("opacity", &format!("{opacity:.2}"));
                        }
                    }
                }
            });
            if let Some(win) = web_sys::window() {
                let _ = win.add_event_listener_with_callback("scroll", scroll_cb.as_ref().unchecked_ref());
                scroll_cb.forget();
            }
        });
    }

    view! {
        <section class="section" id="career">
            <div class="container">
                <div class="section-header fade-in">
                    <h2>{move || translate(locale.get(), "career.title")}</h2>
                    <p>{move || translate(locale.get(), "career.subtitle")}</p>
                </div>
                <div class="timeline">
                    {TIMELINE.iter().map(|item| {
                        let key = item.key;
                        let year = item.year;
                        let link = item.link;
                        let highlight = item.highlight;
                        let logo_url = item.logo_url;
                        let dot_class = if highlight { "timeline-dot highlight" } else { "timeline-dot" };

                        view! {
                            <div class="timeline-item fade-in">
                                <div class=dot_class></div>
                                <div class="glass-card timeline-card">
                                    <div class="timeline-card-header">
                                        {logo_url.map(|url| view! {
                                            <img class="timeline-logo" src=url alt="" width="40" height="40" loading="lazy" />
                                        })}
                                        <div class="timeline-card-info">
                                            <div class="timeline-year">{year}</div>
                                            <div class="timeline-title">
                                                {match link {
                                                    Some(href) => view! {
                                                        <a href=href target="_blank" rel="noopener">
                                                            {move || translate(locale.get(), &format!("career.{key}.title"))}
                                                            " \u{2197}"
                                                        </a>
                                                    }.into_any(),
                                                    None => view! {
                                                        <span>{move || translate(locale.get(), &format!("career.{key}.title"))}</span>
                                                    }.into_any(),
                                                }}
                                            </div>
                                        </div>
                                    </div>
                                    <div class="timeline-role">{move || translate(locale.get(), &format!("career.{key}.role"))}</div>
                                    <p class="timeline-desc">{move || translate(locale.get(), &format!("career.{key}.desc"))}</p>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </section>
    }
}
