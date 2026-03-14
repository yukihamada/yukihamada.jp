use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::use_params_map;

use crate::data::blog::{self, html_escape};
use crate::i18n::{translate, use_locale, Locale};
use crate::components::blog_search::{BlogSearch, filter_posts};
#[cfg(target_arch = "wasm32")]
use crate::i18n;

#[component]
pub fn BlogList() -> impl IntoView {
    let locale = use_locale();
    let params = use_params_map();

    Effect::new(move |_| {
        if let Some(l) = params.read().get("lang").as_deref().and_then(Locale::from_code) {
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

    let current_locale = locale.get_untracked();
    let lang = params.with_untracked(|p| p.get("lang").unwrap_or_else(|| "ja".to_string()));
    let canonical_blog = format!("https://yukihamada.jp/{lang}/blog");
    let all_posts = blog::posts_for_locale(current_locale);

    let posts_data: Vec<_> = all_posts
        .iter()
        .map(|(slug, raw)| {
            let (meta, body) = blog::parse_frontmatter(raw);
            let mins = blog::reading_minutes(body);
            (slug.to_string(), meta.title, meta.date, meta.description, meta.tags, mins)
        })
        .collect();

    let search_query = RwSignal::new(String::new());
    let filtered_indices = RwSignal::new((0..posts_data.len()).collect::<Vec<usize>>());

    let on_filter = Callback::new(move |query: String| {
        search_query.set(query.clone());
        let posts = blog::posts_for_locale(locale.get_untracked());
        let indices = filter_posts(posts, &query, locale.get_untracked());
        filtered_indices.set(indices);
    });

    view! {
        <Title text=move || format!("Blog - {}", translate(locale.get(), "site.title")) />
        <Meta name="description" content=move || translate(locale.get(), "blog.subtitle") />
        <Meta name="author" content="Yuki Hamada" />
        <Link rel="canonical" href=canonical_blog />
        <Meta property="og:title" content=move || format!("Blog - {}", translate(locale.get(), "site.title")) />
        <Meta property="og:description" content=move || translate(locale.get(), "blog.subtitle") />
        <Meta property="og:type" content="website" />
        <Meta property="og:image" content="https://yukihamada.jp/og-image.svg" />
        <Meta property="og:image:width" content="1200" />
        <Meta property="og:image:height" content="630" />
        <Meta property="og:site_name" content="Yuki Hamada" />
        <Meta property="og:url" content=move || format!("https://yukihamada.jp/{}/blog", locale.get().code()) />
        <Meta name="twitter:card" content="summary_large_image" />
        <Meta name="twitter:site" content="@yukihamada" />
        <Meta name="twitter:creator" content="@yukihamada" />
        <Meta name="twitter:title" content=move || format!("Blog - {}", translate(locale.get(), "site.title")) />
        <Meta name="twitter:description" content=move || translate(locale.get(), "blog.subtitle") />
        <Meta name="twitter:image" content="https://yukihamada.jp/og-image.svg" />
        <Link rel="alternate" hreflang="ja" href="https://yukihamada.jp/ja/blog" />
        <Link rel="alternate" hreflang="en" href="https://yukihamada.jp/en/blog" />
        <Link rel="alternate" hreflang="x-default" href="https://yukihamada.jp/ja/blog" />

        <div class="section" style="padding-top:100px;">
            <div class="container">
                <div class="section-header fade-in">
                    <h2>{move || translate(locale.get(), "blog.title")}</h2>
                    <p>{move || translate(locale.get(), "blog.subtitle")}</p>
                </div>
                <BlogSearch on_filter=on_filter />
                <div class="blog-list">
                    {move || {
                        let indices = filtered_indices.get();
                        indices.iter().filter_map(|&i| {
                            posts_data.get(i).map(|(slug, title, date, desc, tags, mins)| {
                                let slug = slug.clone();
                                let title = html_escape(title);
                                let date = html_escape(date);
                                let desc = html_escape(desc);
                                let mins = *mins;
                                let tags_html: Vec<_> = tags.iter().map(|t| {
                                    let t = html_escape(t);
                                    view! { <span class="blog-tag">{t}</span> }
                                }).collect();

                                view! {
                                    <a href=move || format!("/{}/blog/{slug}", locale.get().code()) class="glass-card blog-card fade-in">
                                        <div class="blog-card-date">{date.clone()}</div>
                                        <div class="blog-card-title">{title.clone()}</div>
                                        <p class="blog-card-desc">{desc.clone()}</p>
                                        <div class="blog-card-tags">{tags_html}</div>
                                        <div class="blog-card-read">{mins} " " {move || translate(locale.get(), "blog.min_read")}</div>
                                    </a>
                                }
                            })
                        }).collect::<Vec<_>>()
                    }}
                    <Show when=move || filtered_indices.get().is_empty() && !search_query.get().is_empty()>
                        <div class="blog-search-empty">
                            <p>"No posts found matching \""  {move || search_query.get()} "\""</p>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn BlogPost() -> impl IntoView {
    let locale = use_locale();
    let params = use_params_map();

    Effect::new(move |_| {
        if let Some(l) = params.read().get("lang").as_deref().and_then(Locale::from_code) {
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

    let slug = move || params.read().get("slug").unwrap_or_default();

    let post_data = move || {
        let s = slug();
        blog::posts_for_locale(locale.get())
            .iter()
            .find(|(ps, _)| *ps == s)
            .map(|(_, raw)| {
                let (meta, md_body) = blog::parse_frontmatter(raw);
                let html_content = blog::md_to_html(md_body);
                let mins = blog::reading_minutes(md_body);
                (meta.title, meta.date, meta.description, meta.tags, html_content, mins)
            })
    };

    view! {
        {move || {
            match post_data() {
                Some((title, date, desc, tags, html_content, mins)) => {
                    let page_title = format!("{} - Yuki Hamada", title);
                    let meta_desc = if desc.is_empty() { title.clone() } else { desc.clone() };
                    let og_title = page_title.clone();
                    let og_desc = meta_desc.clone();
                    let tw_title = page_title.clone();
                    let tw_desc = meta_desc.clone();
                    let current_slug = slug();
                    let og_url = format!("https://yukihamada.jp/{}/blog/{}", locale.get().code(), current_slug);
                    let canonical_url = og_url.clone();
                    let og_image = "https://yukihamada.jp/og-image.svg";
                    let ja_url = format!("https://yukihamada.jp/ja/blog/{current_slug}");
                    let en_url = format!("https://yukihamada.jp/en/blog/{current_slug}");
                    let article_tags: Vec<String> = tags.iter().map(|t| html_escape(t)).collect();
                    let tags_html: Vec<_> = article_tags.iter().map(|t| {
                        view! { <span class="blog-tag">{t.clone()}</span> }
                    }).collect();

                    view! {
                        <Title text=page_title />
                        <Meta name="description" content=meta_desc />
                        <Meta name="author" content="Yuki Hamada" />
                        <Link rel="canonical" href=canonical_url />
                        <Link rel="alternate" hreflang="ja" href=ja_url />
                        <Link rel="alternate" hreflang="en" href=en_url />
                        <Link rel="alternate" hreflang="x-default" href=format!("https://yukihamada.jp/ja/blog/{current_slug}") />
                        // OGP
                        <Meta property="og:title" content=og_title />
                        <Meta property="og:description" content=og_desc />
                        <Meta property="og:type" content="article" />
                        <Meta property="og:url" content=og_url />
                        <Meta property="og:image" content=og_image />
                        <Meta property="og:image:width" content="1200" />
                        <Meta property="og:image:height" content="630" />
                        <Meta property="og:site_name" content="Yuki Hamada" />
                        <Meta property="article:author" content="Yuki Hamada" />
                        <Meta property="article:published_time" content=html_escape(&date) />
                        {article_tags.iter().map(|t| view! {
                            <Meta property="article:tag" content=t.clone() />
                        }).collect::<Vec<_>>()}
                        // Twitter Card
                        <Meta name="twitter:card" content="summary_large_image" />
                        <Meta name="twitter:site" content="@yukihamada" />
                        <Meta name="twitter:creator" content="@yukihamada" />
                        <Meta name="twitter:title" content=tw_title />
                        <Meta name="twitter:description" content=tw_desc />
                        <Meta name="twitter:image" content=og_image />

                        <article class="article">
                            <div class="container">
                                <div class="article-header">
                                    <div class="article-date">{html_escape(&date)}</div>
                                    <h1 class="article-title">{html_escape(&title)}</h1>
                                    <div class="article-meta">
                                        <span>{mins} " " {translate(locale.get(), "blog.min_read")}</span>
                                        <span>{tags_html}</span>
                                    </div>
                                </div>
                                <div class="article-body" inner_html=html_content></div>
                                <div style="margin-top:64px;padding-top:32px;border-top:1px solid var(--border);">
                                    <a href=format!("/{}/blog", locale.get().code()) style="color:var(--primary-light);">
                                        {translate(locale.get(), "blog.back")}
                                    </a>
                                </div>
                            </div>
                        </article>
                    }.into_any()
                }
                None => {
                    view! {
                        <Title text="404 - Not Found" />
                        <div style="text-align:center;padding:120px 20px;">
                            <h2>"404"</h2>
                            <p style="color:var(--text-muted);">"Blog post not found"</p>
                            <a href=format!("/{}/blog", locale.get().code())>{translate(locale.get(), "blog.back")}</a>
                        </div>
                    }.into_any()
                }
            }
        }}
    }
}
