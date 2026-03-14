use leptos::prelude::*;
use crate::data::projects::PROJECTS;
use crate::i18n::{translate, use_locale};

#[component]
pub fn ProjectModal(
    #[prop(into)] project_idx: RwSignal<Option<usize>>,
) -> impl IntoView {
    let locale = use_locale();

    let close = move |_| {
        project_idx.set(None);
    };

    // Close on Escape
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        Effect::new(move |_| {
            let key_cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Escape" {
                    project_idx.set(None);
                }
            });
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let target: &web_sys::EventTarget = doc.unchecked_ref();
                let _ = target.add_event_listener_with_callback("keydown", key_cb.as_ref().unchecked_ref());
                key_cb.forget();
            }
        });
    }

    view! {
        <Show when=move || project_idx.get().is_some()>
            <div class="project-modal-overlay" on:click=close>
                <div class="project-modal" on:click=|e: leptos::ev::MouseEvent| e.stop_propagation()>
                    {move || {
                        let idx = project_idx.get().unwrap_or(0);
                        if idx >= PROJECTS.len() { return view! { <div /> }.into_any(); }
                        let p = &PROJECTS[idx];
                        let key = p.key;
                        let color = p.color;
                        let href = p.href;
                        let icon_svg = p.icon_svg;
                        let logo_url = p.logo_url;
                        let display_name = match key {
                            "enabler_fun" => "enabler.fun",
                            "enablerdao" => "enablerdao.com",
                            "chatweb" => "chatweb.ai",
                            "elio" => "elio.love",
                            "banto" => "banto.work",
                            "jiuflow" => "jiuflow.art",
                            _ => key,
                        };

                        let appstore_url = p.appstore_url;

                        view! {
                            <div class="project-modal-inner" style=format!("--modal-accent: {color};")>
                                <button class="project-modal-close" on:click=close>"✕"</button>
                                <div class="project-modal-header">
                                    {if let Some(url) = logo_url {
                                        view! {
                                            <div class="project-modal-icon project-icon-logo">
                                                <img src=url alt="" style="width:56px;height:56px;object-fit:cover;border-radius:14px;" />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="project-modal-icon" style=format!("background: {color};")>
                                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" inner_html=icon_svg></svg>
                                            </div>
                                        }.into_any()
                                    }}
                                    <h2>{display_name}</h2>
                                </div>
                                <p class="project-modal-desc">
                                    {move || translate(locale.get(), &format!("work.{key}.desc"))}
                                </p>
                                <div class="project-modal-actions">
                                    <a href=href target="_blank" rel="noopener" class="hero-cta">
                                        {move || translate(locale.get(), "work.visit")}
                                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <path d="M7 17L17 7M17 7H7M17 7v10"/>
                                        </svg>
                                    </a>
                                    {appstore_url.map(|url| view! {
                                        <a href=url target="_blank" rel="noopener" class="appstore-badge appstore-badge-modal">
                                            <img src="/assets/badge-appstore.svg" alt="Download on the App Store" />
                                        </a>
                                    })}
                                </div>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </Show>
    }
}
