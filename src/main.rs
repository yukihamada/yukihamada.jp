mod handlers;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

pub struct AppState {
    pub base_url: String,
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Pages
        .route("/", get(handlers::pages::home))
        .route("/en", get(handlers::pages::home_en))
        // SEO
        .route("/sitemap.xml", get(handlers::seo::sitemap_xml))
        .route("/robots.txt", get(handlers::seo::robots_txt))
        // Health check
        .route("/health", get(health))
        // Static files
        .nest_service("/static", ServeDir::new("static").precompressed_gzip())
        // Middleware
        .layer(CompressionLayer::new())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "https://yukihamada.jp".to_string());

    let addr = format!("{}:{}", host, port);

    let state = Arc::new(AppState { base_url });

    let app = build_router(state);

    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
