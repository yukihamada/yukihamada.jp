use leptos::prelude::*;

#[component]
pub fn ScrollObserver() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        use crate::wasm_utils::prefers_reduced_motion;

        Effect::new(move |_| {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let document = match window.document() {
                Some(d) => d,
                None => return,
            };

            let reduced = prefers_reduced_motion();

            // --- IntersectionObserver for fade-in ---
            let observer_cb = Closure::<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>::new(
                move |entries: js_sys::Array, observer: web_sys::IntersectionObserver| {
                    for i in 0..entries.length() {
                        if let Ok(entry) = entries.get(i).dyn_into::<web_sys::IntersectionObserverEntry>() {
                            if entry.is_intersecting() {
                                let target = entry.target();
                                let _ = target.class_list().add_1("visible");
                                if reduced {
                                    // Immediately show without animation
                                    let el: &web_sys::HtmlElement = target.unchecked_ref();
                                    let _ = el.style().set_property("opacity", "1");
                                    let _ = el.style().set_property("transform", "none");
                                }
                                observer.unobserve(&target);
                            }
                        }
                    }
                },
            );

            let opts = web_sys::IntersectionObserverInit::new();
            opts.set_threshold(&JsValue::from_f64(0.1));
            opts.set_root_margin("0px 0px -50px 0px");

            let observer = match web_sys::IntersectionObserver::new_with_options(
                observer_cb.as_ref().unchecked_ref(),
                &opts,
            ) {
                Ok(o) => o,
                Err(_) => return,
            };

            // Observe existing elements
            let selector = ".fade-in,.fade-in-left";
            if let Ok(elements) = document.query_selector_all(selector) {
                for i in 0..elements.length() {
                    if let Some(el) = elements.get(i) {
                        observer.observe(el.unchecked_ref());
                    }
                }
            }

            // MutationObserver for dynamically added elements
            let observer_clone = observer.clone();
            let mutation_cb = Closure::<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>::new(
                move |_mutations: js_sys::Array, _: web_sys::MutationObserver| {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Ok(elements) = doc.query_selector_all(".fade-in,.fade-in-left") {
                            for i in 0..elements.length() {
                                if let Some(el) = elements.get(i) {
                                    let el: &web_sys::Element = el.unchecked_ref();
                                    if !el.class_list().contains("visible") {
                                        observer_clone.observe(el);
                                    }
                                }
                            }
                        }
                    }
                },
            );

            if let Ok(mutation_observer) = web_sys::MutationObserver::new(mutation_cb.as_ref().unchecked_ref()) {
                let config = web_sys::MutationObserverInit::new();
                config.set_child_list(true);
                config.set_subtree(true);
                let _ = mutation_observer.observe_with_options(&document.body().unwrap().into(), &config);
                mutation_cb.forget();
                std::mem::forget(mutation_observer);
            }

            observer_cb.forget();

            // --- Scroll event for nav progress + .scrolled ---
            let scroll_cb = Closure::<dyn FnMut()>::new(move || {
                let win = match web_sys::window() {
                    Some(w) => w,
                    None => return,
                };
                let doc = match win.document() {
                    Some(d) => d,
                    None => return,
                };
                let doc_el = doc.document_element().unwrap();
                let scroll_h = doc_el.scroll_height() as f64;
                let inner_h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                let scroll_y = win.scroll_y().unwrap_or(0.0);
                let h = scroll_h - inner_h;
                let progress = if h > 0.0 { scroll_y / h * 100.0 } else { 0.0 };

                if let Ok(Some(nav)) = doc.query_selector(".nav") {
                    let nav_el: &web_sys::HtmlElement = nav.unchecked_ref();
                    let _ = nav_el.style().set_property("--scroll-progress", &format!("{progress}%"));
                    if scroll_y > 50.0 {
                        let _ = nav.class_list().add_1("scrolled");
                    } else {
                        let _ = nav.class_list().remove_1("scrolled");
                    }
                }
            });

            let _ = window.add_event_listener_with_callback(
                "scroll",
                scroll_cb.as_ref().unchecked_ref(),
            );
            scroll_cb.forget();

            // --- Click outside to close lang dropdown ---
            let click_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let doc = match web_sys::window().and_then(|w| w.document()) {
                    Some(d) => d,
                    None => return,
                };
                if let Ok(Some(dd)) = doc.query_selector(".nav-lang-dropdown.open") {
                    if let Some(target) = e.target() {
                        let target_el: &web_sys::Element = target.unchecked_ref();
                        if let Ok(Some(wrap)) = doc.query_selector(".nav-lang-wrap") {
                            if !wrap.contains(Some(target_el)) {
                                let _ = dd.class_list().remove_1("open");
                            }
                        }
                    }
                }
            });

            let doc_target: &web_sys::EventTarget = document.unchecked_ref();
            let _ = doc_target.add_event_listener_with_callback(
                "click",
                click_cb.as_ref().unchecked_ref(),
            );
            click_cb.forget();
        });
    }

    view! {}
}
