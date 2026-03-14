#![recursion_limit = "512"]

pub mod components;
pub mod data;
pub mod i18n;
pub mod pages;

#[cfg(target_arch = "wasm32")]
pub mod wasm_utils;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use components::layout::Layout;
use pages::{
    blog::{BlogList, BlogPost},
    home::HomePage,
    music::MusicPage,
    not_found::NotFound,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let ga_id = std::env::var("GOOGLE_ANALYTICS_ID")
        .unwrap_or_else(|_| "G-CSX8H7KHV7".to_string());
    let ga_src = format!(
        "https://www.googletagmanager.com/gtag/js?id={ga_id}"
    );
    let ga_inline = format!(
        "window.dataLayer=window.dataLayer||[];function gtag(){{dataLayer.push(arguments);}}gtag('js',new Date());gtag('config','{ga_id}');"
    );
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="theme-color" content="#0f172a" />
                <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
                <link rel="apple-touch-icon" sizes="180x180" href="/favicon.svg" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
                <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@300;400;500;600;700;800&family=Noto+Sans+JP:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
                <link rel="stylesheet" href="/pkg/yukihamada-jp.css" />
                <link rel="alternate" type="application/rss+xml" title="濱田優貴 Blog RSS" href="/feed.xml" />
            </head>
            <body>
                <App />
                <script type="application/ld+json">
                    r#"{"@context":"https://schema.org","@type":"Person","name":"Yuki Hamada","alternateName":"濱田優貴","url":"https://yukihamada.jp","image":"https://yukihamada.jp/og-image.svg","jobTitle":"Founder & CEO","worksFor":{"@type":"Organization","name":"Enabler, Inc.","url":"https://enablerhq.com"},"sameAs":["https://x.com/yukihamada","https://github.com/yukihamada","https://linkedin.com/in/yukihamada","https://facebook.com/yukihamada","https://instagram.com/yukihamada"]}"#
                </script>
                <script async src=ga_src></script>
                <script inner_html=ga_inline></script>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    i18n::provide_i18n();

    #[cfg(target_arch = "wasm32")]
    {
        let locale = i18n::use_locale();
        Effect::new(move |_| {
            let detected = i18n::browser::get_saved_locale()
                .or_else(i18n::browser::detect_locale)
                .unwrap_or(i18n::Locale::Ja);
            locale.set(detected);
            i18n::browser::update_html_lang(detected);
        });
    }

    view! {
        // CSS is already in shell() <head>; Stylesheet here would duplicate it on client nav
        <Router>
            <Layout>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/:lang") view=HomePage />
                    <Route path=path!("/:lang/blog") view=BlogList />
                    <Route path=path!("/:lang/blog/:slug") view=BlogPost />
                    <Route path=path!("/:lang/music") view=MusicPage />
                </Routes>
            </Layout>
        </Router>
    }
}

