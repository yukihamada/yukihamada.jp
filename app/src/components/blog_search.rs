use leptos::prelude::*;
use crate::data::blog;
use crate::i18n::Locale;

#[component]
pub fn BlogSearch(
    #[prop(into)] on_filter: Callback<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let focused = RwSignal::new(false);

    let on_input = move |ev: leptos::ev::Event| {
        let val: String = event_target_value(&ev);
        query.set(val.clone());
        on_filter.run(val);
    };

    view! {
        <div class="blog-search-wrap" class:focused=move || focused.get()>
            <svg class="blog-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/>
                <path d="M21 21l-4.35-4.35"/>
            </svg>
            <input
                type="text"
                class="blog-search-input"
                placeholder="Search posts..."
                on:input=on_input
                on:focus=move |_| focused.set(true)
                on:blur=move |_| focused.set(false)
                prop:value=move || query.get()
            />
            <Show when=move || !query.get().is_empty()>
                <button class="blog-search-clear" on:click=move |_| {
                    query.set(String::new());
                    on_filter.run(String::new());
                }>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width:16px;height:16px;">
                        <path d="M18 6L6 18M6 6l12 12"/>
                    </svg>
                </button>
            </Show>
        </div>
    }
}

/// Filter posts based on query string. Returns matching indices.
pub fn filter_posts(posts: &[(&str, &str)], query: &str, _locale: Locale) -> Vec<usize> {
    if query.is_empty() {
        return (0..posts.len()).collect();
    }

    let q = query.to_lowercase();
    let mut results: Vec<(usize, u32)> = Vec::new();

    for (i, (slug, raw)) in posts.iter().enumerate() {
        let (meta, body) = blog::parse_frontmatter(raw);
        let mut score = 0u32;

        // Title match (highest weight)
        if meta.title.to_lowercase().contains(&q) {
            score += 100;
        }
        // Tag match
        for tag in &meta.tags {
            if tag.to_lowercase().contains(&q) {
                score += 50;
            }
        }
        // Description match
        if meta.description.to_lowercase().contains(&q) {
            score += 30;
        }
        // Slug match
        if slug.to_lowercase().contains(&q) {
            score += 20;
        }
        // Body match (lower weight)
        if body.to_lowercase().contains(&q) {
            score += 10;
        }

        if score > 0 {
            results.push((i, score));
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.1.cmp(&a.1));
    results.into_iter().map(|(i, _)| i).collect()
}
