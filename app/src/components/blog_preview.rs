use leptos::prelude::*;

use crate::data::blog::{self, html_escape};
use crate::i18n::{translate, use_locale};

#[component]
pub fn BlogPreview() -> impl IntoView {
    let locale = use_locale();
    let posts = blog::posts_for_locale(locale.get_untracked());

    if posts.is_empty() {
        return view! { <div></div> }.into_any();
    }

    let (slug0, raw0) = &posts[0];
    let (meta0, body0) = blog::parse_frontmatter(raw0);
    let mins0 = blog::reading_minutes(body0);
    let slug0 = slug0.to_string();
    let title0 = meta0.title.clone();
    let desc0 = meta0.description.clone();
    let date0 = meta0.date.clone();
    let tags0 = meta0.tags.clone();

    let other_posts: Vec<_> = posts
        .iter()
        .skip(1)
        .take(3)
        .map(|(slug, raw)| {
            let (meta, _body) = blog::parse_frontmatter(raw);
            (slug.to_string(), meta.title, meta.description, meta.date, meta.tags.into_iter().take(2).collect::<Vec<_>>())
        })
        .collect();

    view! {
        <section class="section" id="blog">
            <div class="container">
                <div class="blog-preview-header fade-in">
                    <div>
                        <p class="section-label">
                            {move || translate(locale.get(), "blog.label")}
                        </p>
                        <h2 class="section-header-title">
                            {move || translate(locale.get(), "blog.title")}
                            <span class="gradient-text">{move || translate(locale.get(), "blog.title_gradient")}</span>
                        </h2>
                    </div>
                    <a href={move || format!("/{}/blog", locale.get().code())} class="blog-view-all">
                        {move || translate(locale.get(), "blog.view_all")}
                    </a>
                </div>
                <div class="blog-home-grid fade-in">
                    <a href=move || format!("/{}/blog/{slug0}", locale.get().code()) class="blog-featured-card glass-card">
                        <div class="blog-featured-tags">
                            <span class="blog-featured-badge">
                                {move || translate(locale.get(), "blog.featured")}
                            </span>
                            {tags0.iter().map(|t| {
                                let t = html_escape(t);
                                view! { <span class="blog-tag">{t}</span> }
                            }).collect::<Vec<_>>()}
                        </div>
                        <h3 class="blog-featured-title">{html_escape(&title0)}</h3>
                        <p class="blog-featured-desc">{html_escape(&desc0)}</p>
                        <div class="blog-featured-footer">
                            <span class="blog-featured-meta">
                                {html_escape(&date0)} " · " {mins0} " " {move || translate(locale.get(), "blog.min_read")}
                            </span>
                            <span class="blog-featured-cta">
                                {move || translate(locale.get(), "blog.read_more")}
                            </span>
                        </div>
                    </a>
                    <div class="blog-side-list">
                        {other_posts.iter().map(|(slug, title, desc, date, tags)| {
                            let slug = slug.clone();
                            let title = html_escape(title);
                            let desc = html_escape(desc);
                            let date = html_escape(date);
                            let tags_html: Vec<_> = tags.iter().map(|t| {
                                let t = html_escape(t);
                                view! { <span class="blog-tag">{t}</span> }
                            }).collect();

                            view! {
                                <a href=move || format!("/{}/blog/{slug}", locale.get().code()) class="blog-side-card glass-card fade-in">
                                    <div class="blog-side-top">
                                        {tags_html}
                                        <span class="blog-side-date">{date}</span>
                                    </div>
                                    <div class="blog-side-title">{title}</div>
                                    <p class="blog-side-desc">{desc}</p>
                                </a>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            </div>
        </section>
    }.into_any()
}
