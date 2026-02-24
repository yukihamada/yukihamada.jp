use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::AppState;

pub async fn sitemap_xml(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let base = &state.base_url;
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>{base}</loc>
    <lastmod>{now}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
    <xhtml:link rel="alternate" hreflang="ja" href="{base}"/>
    <xhtml:link rel="alternate" hreflang="en" href="{base}/en"/>
  </url>
  <url>
    <loc>{base}/en</loc>
    <lastmod>{now}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.9</priority>
    <xhtml:link rel="alternate" hreflang="ja" href="{base}"/>
    <xhtml:link rel="alternate" hreflang="en" href="{base}/en"/>
  </url>
</urlset>"#
    );

    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

pub async fn robots_txt(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let base = &state.base_url;
    let body = format!(
        "User-agent: *\nAllow: /\n\nSitemap: {base}/sitemap.xml\n"
    );
    ([(header::CONTENT_TYPE, "text/plain")], body)
}
