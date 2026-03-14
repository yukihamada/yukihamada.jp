use leptos::prelude::*;

use crate::data::projects::PROJECTS;
use crate::i18n::{translate, use_locale};
use super::project_modal::ProjectModal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn setup_tilt(el: &web_sys::HtmlElement) {
    let el_clone = el.clone();
    let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let rect = el_clone.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let y = e.client_y() as f64 - rect.top();
        let w = rect.width();
        let h = rect.height();
        let nx = (x / w - 0.5) * 2.0;
        let ny = (y / h - 0.5) * 2.0;
        let rotate_y = nx * 8.0;
        let rotate_x = -ny * 8.0;
        let _ = el_clone.style().set_property(
            "transform",
            &format!("perspective(800px) rotateX({rotate_x}deg) rotateY({rotate_y}deg) translateZ(10px)"),
        );
    });
    let _ = el.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());
    move_cb.forget();

    let el_leave = el.clone();
    let leave_cb = Closure::<dyn FnMut()>::new(move || {
        let _ = el_leave.style().set_property("transform", "perspective(800px) rotateX(0) rotateY(0) translateZ(0)");
    });
    let _ = el.add_event_listener_with_callback("mouseleave", leave_cb.as_ref().unchecked_ref());
    leave_cb.forget();
}

#[component]
pub fn Projects() -> impl IntoView {
    let locale = use_locale();
    let modal_idx = RwSignal::new(Option::<usize>::None);

    // Track all projects hovered for "curious" achievement
    #[cfg(target_arch = "wasm32")]
    let hovered_set = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::<usize>::new()));

    view! {
        <section class="section" id="work">
            <div class="container">
                <div class="section-header fade-in">
                    <p class="section-label">{move || translate(locale.get(), "work.label")}</p>
                    <h2>{move || translate(locale.get(), "work.title")}</h2>
                    <p>{move || translate(locale.get(), "work.subtitle")}</p>
                </div>
                <div class="projects-grid">
                    {PROJECTS.iter().enumerate().map(|(idx, p)| {
                        let key = p.key;
                        let badge = p.badge;
                        let icon_svg = p.icon_svg;
                        let color = p.color;
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

                        let card_ref = NodeRef::<leptos::html::Div>::new();

                        #[cfg(target_arch = "wasm32")]
                        {
                            let hovered_set = hovered_set.clone();
                            Effect::new(move |_| {
                                if let Some(el) = card_ref.get() {
                                    let html_el: &web_sys::HtmlElement = &el;
                                    setup_tilt(html_el);

                                    // Track hover for achievement
                                    let hs = hovered_set.clone();
                                    let hover_cb = Closure::<dyn FnMut()>::new(move || {
                                        let mut set = hs.borrow_mut();
                                        set.insert(idx);
                                        if set.len() >= PROJECTS.len() {
                                            super::achievements::unlock_achievement("curious");
                                        }
                                    });
                                    let _ = html_el.add_event_listener_with_callback("mouseenter", hover_cb.as_ref().unchecked_ref());
                                    hover_cb.forget();
                                }
                            });
                        }

                        let appstore_url = p.appstore_url;

                        let on_expand = move |e: leptos::ev::MouseEvent| {
                            e.prevent_default();
                            e.stop_propagation();
                            modal_idx.set(Some(idx));
                        };

                        view! {
                            <div node_ref=card_ref class="project-card fade-in"
                               style=format!("--card-accent: {color};")>
                                <div class="project-card-inner">
                                    <div class="project-header">
                                        {if let Some(url) = logo_url {
                                            view! {
                                                <div class="project-icon project-icon-logo">
                                                    <img src=url alt="" width="48" height="48" loading="lazy" style="width:48px;height:48px;object-fit:cover;border-radius:12px;"/>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="project-icon" style=format!("background: {color};")>
                                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" inner_html=icon_svg></svg>
                                                </div>
                                            }.into_any()
                                        }}
                                        <div style="display:flex;gap:8px;align-items:center;">
                                            {badge.map(|b| view! {
                                                <span class="project-badge" style=format!("background: {color} / 0.12; color: {color};")>{b}</span>
                                            })}
                                            <button class="project-expand-btn" on:click=on_expand title="Details">
                                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                                                    <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
                                                </svg>
                                            </button>
                                        </div>
                                    </div>
                                    <h3 class="project-title">{display_name}</h3>
                                    <p class="project-desc">
                                        {move || translate(locale.get(), &format!("work.{key}.desc"))}
                                    </p>
                                    <div class="project-links">
                                        <a href=p.href target="_blank" rel="noopener" class="project-link">
                                            {move || translate(locale.get(), "work.visit")}
                                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <path d="M7 17L17 7M17 7H7M17 7v10"/>
                                            </svg>
                                        </a>
                                        {appstore_url.map(|url| view! {
                                            <a href=url target="_blank" rel="noopener" class="appstore-badge">
                                                <img src="/assets/badge-appstore.svg" alt="Download on the App Store" />
                                            </a>
                                        })}
                                    </div>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
            <ProjectModal project_idx=modal_idx />
        </section>
    }
}
