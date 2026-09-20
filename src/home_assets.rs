//! Split large, static home scripts/styles into content-addressed responses.
use axum::{extract::Path, http::StatusCode, response::{IntoResponse, Response}};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

struct Asset { original: String, replacement: String, name: String, body: String, mime: &'static str }
fn assets() -> &'static Vec<Asset> {
    static ASSETS: OnceLock<Vec<Asset>> = OnceLock::new();
    ASSETS.get_or_init(|| {
        let source = include_str!("../templates/home.html");
        let mut result = Vec::new();
        for (tag, ext, mime) in [("script", "js", "text/javascript; charset=utf-8"), ("style", "css", "text/css; charset=utf-8")] {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let mut rest = source;
            while let Some(start) = rest.find(&open) {
                rest = &rest[start + open.len()..];
                let Some(end) = rest.find(&close) else { break };
                let body = &rest[..end];
                if body.len() > 4096 {
                    assert!(!body.contains("{{") && !body.contains("{%"));
                    let name = format!("{:x}.{ext}", Sha256::digest(body.as_bytes()));
                    let replacement = if tag == "script" { format!("<script src=\"/home-assets/{name}\"></script>") }
                        else { format!("<link rel=\"stylesheet\" href=\"/home-assets/{name}\">") };
                    result.push(Asset { original: format!("{open}{body}{close}"), replacement, name, body: body.to_string(), mime });
                }
                rest = &rest[end + close.len()..];
            }
        }
        result
    })
}
pub fn split(mut html: String) -> String {
    for asset in assets() { html = html.replace(&asset.original, &asset.replacement); }
    html
}
pub async fn serve(Path(name): Path<String>) -> Response {
    match assets().iter().find(|asset| asset.name == name) {
        Some(asset) => ([("content-type", asset.mime), ("cache-control", "public, max-age=31536000, immutable")], asset.body.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_preserves_content_addresses() {
        let source = include_str!("../templates/home.html");
        let html = split(source.to_string());
        assert!(html.len() < 200_000);
        assert!(assets().len() >= 3);
        for asset in assets() {
            assert!(html.contains(&asset.replacement));
            assert!(!html.contains(&asset.original));
            assert!(asset.name.starts_with(&format!("{:x}", Sha256::digest(asset.body.as_bytes()))));
        }
        assert!(html.contains("{% for p in posts %}"));
    }
}
