use leptos::prelude::*;
use leptos_meta::*;

use crate::i18n::{translate, use_locale};

#[component]
pub fn NotFound() -> impl IntoView {
    let locale = use_locale();

    view! {
        <Title text="404 - Not Found" />
        <div style="text-align:center;padding:120px 20px;min-height:60vh;display:flex;align-items:center;justify-content:center;">
            <div>
                <h1 style="font-size:4rem;font-weight:800;margin-bottom:16px;" class="gradient-text">
                    {move || translate(locale.get(), "error.not_found")}
                </h1>
                <p style="color:var(--text-muted);font-size:1.2rem;margin-bottom:32px;">
                    {move || translate(locale.get(), "error.not_found_desc")}
                </p>
                <a href=move || format!("/{}", locale.get().code()) class="hero-cta" style="display:inline-flex;">
                    {move || translate(locale.get(), "error.back_home")}
                </a>
            </div>
        </div>
    }
}
