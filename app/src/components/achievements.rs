use leptos::prelude::*;

#[allow(dead_code)]
struct Achievement {
    key: &'static str,
    icon: &'static str,
    label_key: &'static str,
}

const ACHIEVEMENTS: &[Achievement] = &[
    Achievement { key: "explorer", icon: "🗺️", label_key: "achievement.explorer" },
    Achievement { key: "loyal", icon: "⭐", label_key: "achievement.loyal" },
    Achievement { key: "konami", icon: "🎮", label_key: "achievement.konami" },
    Achievement { key: "typist", icon: "⌨️", label_key: "achievement.typist" },
    Achievement { key: "curious", icon: "🔍", label_key: "achievement.curious" },
];

#[cfg(target_arch = "wasm32")]
fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn is_unlocked(key: &str) -> bool {
    get_storage()
        .and_then(|s| s.get_item(&format!("achievement_{key}")).ok().flatten())
        .map(|v| v == "true")
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
pub fn unlock_achievement(key: &str) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(&format!("achievement_{key}"), "true");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unlock_achievement(_key: &str) {}

#[component]
pub fn AchievementTracker() -> impl IntoView {
    let toast_text = RwSignal::new(Option::<String>::None);

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;

        Effect::new(move |_| {
            let storage = match get_storage() {
                Some(s) => s,
                None => return,
            };

            // Track visit count
            let visits: i32 = storage
                .get_item("visit_count").ok().flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let new_visits = visits + 1;
            let _ = storage.set_item("visit_count", &new_visits.to_string());

            // Unlock "loyal" at 5 visits
            if new_visits >= 5 && !is_unlocked("loyal") {
                unlock_achievement("loyal");
                toast_text.set(Some("⭐ Loyal Visitor!".to_string()));
            }

            // Welcome back message for returning visitors
            if new_visits >= 3 {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(body) = doc.body() {
                        let _ = body.class_list().add_1("returning-visitor");
                    }
                }
            }

            // Track section views for "explorer" achievement
            let sections_needed = ["top", "work", "career"];
            let observer_cb = Closure::<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>::new(
                move |entries: js_sys::Array, _obs: web_sys::IntersectionObserver| {
                    let storage = match get_storage() {
                        Some(s) => s,
                        None => return,
                    };
                    for i in 0..entries.length() {
                        if let Ok(entry) = entries.get(i).dyn_into::<web_sys::IntersectionObserverEntry>() {
                            if entry.is_intersecting() {
                                let id = entry.target().id();
                                if !id.is_empty() {
                                    let _ = storage.set_item(&format!("seen_{id}"), "true");
                                }
                            }
                        }
                    }
                    // Check if all sections seen
                    let all_seen = sections_needed.iter().all(|s| {
                        storage.get_item(&format!("seen_{s}")).ok().flatten()
                            .map(|v| v == "true").unwrap_or(false)
                    });
                    if all_seen && !is_unlocked("explorer") {
                        unlock_achievement("explorer");
                        toast_text.set(Some("🗺️ Explorer!".to_string()));
                    }
                },
            );

            let opts = web_sys::IntersectionObserverInit::new();
            opts.set_threshold(&wasm_bindgen::JsValue::from_f64(0.3));

            if let Ok(observer) = web_sys::IntersectionObserver::new_with_options(
                observer_cb.as_ref().unchecked_ref(), &opts,
            ) {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    for id in &["top", "work", "career"] {
                        if let Some(el) = doc.get_element_by_id(id) {
                            observer.observe(&el);
                        }
                    }
                }
                observer_cb.forget();
            }
        });

        // Auto-dismiss toast
        Effect::new(move |_| {
            if toast_text.get().is_some() {
                wasm_bindgen_futures::spawn_local(async move {
                    crate::wasm_utils::sleep_ms(3000).await;
                    toast_text.set(None);
                });
            }
        });
    }

    // Start at 0 so SSR and initial WASM hydration produce identical DOM.
    // A WASM-only Effect updates the signal after hydration completes.
    let unlocked_count = RwSignal::new(0usize);

    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let c = ACHIEVEMENTS.iter().filter(|a| is_unlocked(a.key)).count();
        unlocked_count.set(c);
    });

    view! {
        <div class="achievement-root">
            {move || toast_text.get().map(|t| view! {
                <div class="achievement-toast">{t}</div>
            })}
            {move || {
                let c = unlocked_count.get();
                (c > 0).then(|| view! {
                    <div class="achievement-badge" title="Achievements unlocked">
                        <span class="achievement-count">{c}</span>
                        <span>"/"</span>
                        <span>{ACHIEVEMENTS.len()}</span>
                    </div>
                })
            }}
        </div>
    }
}
