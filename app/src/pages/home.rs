use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::use_params_map;

use crate::components::{
    blog_preview::BlogPreview, hero::Hero, projects::Projects, stats::Stats,
    timeline::Timeline, skill_tree::SkillTree,
};
use crate::i18n::{translate, use_locale, Locale};
#[cfg(target_arch = "wasm32")]
use crate::i18n;

#[component]
pub fn HomePage() -> impl IntoView {
    let locale = use_locale();
    let params = use_params_map();

    Effect::new(move |_| {
        let lang = params.read().get("lang");
        if let Some(l) = lang.as_deref().and_then(Locale::from_code) {
            if locale.get() != l {
                locale.set(l);
                #[cfg(target_arch = "wasm32")]
                {
                    i18n::browser::save_locale(l);
                    i18n::browser::update_html_lang(l);
                }
            }
        }
    });

    let og_image = "https://yukihamada.jp/og-image.svg";
    let lang = params.with_untracked(|p| p.get("lang").unwrap_or_else(|| "ja".to_string()));
    let canonical_url = format!("https://yukihamada.jp/{lang}");

    view! {
        <Title text=move || translate(locale.get(), "site.title") />
        <Meta name="description" content=move || translate(locale.get(), "site.description") />
        <Meta name="author" content="Yuki Hamada" />
        <Link rel="canonical" href=canonical_url />
        <Meta property="og:title" content=move || translate(locale.get(), "site.title") />
        <Meta property="og:description" content=move || translate(locale.get(), "site.description") />
        <Meta property="og:type" content="website" />
        <Meta property="og:image" content=og_image />
        <Meta property="og:image:width" content="1200" />
        <Meta property="og:image:height" content="630" />
        <Meta property="og:site_name" content="Yuki Hamada" />
        <Meta property="og:url" content=move || format!("https://yukihamada.jp/{}", locale.get().code()) />
        <Meta name="twitter:card" content="summary_large_image" />
        <Meta name="twitter:site" content="@yukihamada" />
        <Meta name="twitter:creator" content="@yukihamada" />
        <Meta name="twitter:title" content=move || translate(locale.get(), "site.title") />
        <Meta name="twitter:description" content=move || translate(locale.get(), "site.description") />
        <Meta name="twitter:image" content=og_image />
        <Link rel="alternate" hreflang="ja" href="https://yukihamada.jp/ja" />
        <Link rel="alternate" hreflang="en" href="https://yukihamada.jp/en" />
        <Link rel="alternate" hreflang="zh" href="https://yukihamada.jp/zh" />
        <Link rel="alternate" hreflang="ko" href="https://yukihamada.jp/ko" />
        <Link rel="alternate" hreflang="es" href="https://yukihamada.jp/es" />
        <Link rel="alternate" hreflang="fr" href="https://yukihamada.jp/fr" />
        <Link rel="alternate" hreflang="x-default" href="https://yukihamada.jp/ja" />

        <Hero />
        <Stats />
        <SkillTree />
        <Projects />
        <Timeline />
        <BlogPreview />
    }
}
