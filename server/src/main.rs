use axum::{
    extract::State,
    http::{Uri, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use axum::body::Body;
use axum::http::Request;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use serde::{Deserialize, Serialize};
use yukihamada_app::App;

// Load .env on startup (optional — Fly.io injects env vars directly)
fn load_dotenv() {
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }
            if let Some((k, v)) = line.split_once('=') {
                if std::env::var(k).is_err() {
                    std::env::set_var(k, v);
                }
            }
        }
    }
}

const LANGS: &[&str] = &["ja", "en", "zh", "ko", "es", "fr"];

async fn redirect_root() -> Redirect {
    Redirect::permanent("/ja")
}

/// Fallback handler: redirect trailing-slash URLs, then delegate to Leptos SSR.
/// Matchit 0.7.x returns ExtraTrailingSlash (not NotFound) for e.g. /ja/, so
/// explicit `/:lang/` routes never fire — the fallback is the reliable interception point.
async fn fallback(
    uri: Uri,
    State(opts): State<LeptosOptions>,
    req: Request<Body>,
) -> Response {
    let path = uri.path();
    if path != "/" && path.ends_with('/') {
        let trimmed = path.trim_end_matches('/');
        let location = match uri.query() {
            Some(q) => format!("{trimmed}?{q}"),
            None => trimmed.to_string(),
        };
        return Redirect::permanent(&location).into_response();
    }
    leptos_axum::file_and_error_handler::<LeptosOptions, _>(yukihamada_app::shell)(
        uri,
        State(opts),
        req,
    )
    .await
}

async fn sitemap() -> Response {
    let base = "https://yukihamada.jp";
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xhtml="http://www.w3.org/1999/xhtml">
"#,
    );

    let static_paths = ["", "/blog", "/music"];
    for path in &static_paths {
        for &lang in LANGS {
            xml.push_str(&format!("  <url>\n    <loc>{base}/{lang}{path}</loc>\n"));
            for &alt in LANGS {
                xml.push_str(&format!(
                    "    <xhtml:link rel=\"alternate\" hreflang=\"{alt}\" href=\"{base}/{alt}{path}\" />\n"
                ));
            }
            xml.push_str("    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{base}/ja{path}\" />\n");
            xml.push_str("  </url>\n");
        }
    }

    for (slug, _) in yukihamada_app::data::blog::POSTS_JA {
        for &lang in &["ja", "en"] {
            xml.push_str(&format!("  <url>\n    <loc>{base}/{lang}/blog/{slug}</loc>\n"));
            xml.push_str(&format!(
                "    <xhtml:link rel=\"alternate\" hreflang=\"ja\" href=\"{base}/ja/blog/{slug}\" />\n"
            ));
            xml.push_str(&format!(
                "    <xhtml:link rel=\"alternate\" hreflang=\"en\" href=\"{base}/en/blog/{slug}\" />\n"
            ));
            xml.push_str("  </url>\n");
        }
    }

    xml.push_str("</urlset>\n");

    (
        [("content-type", "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

async fn rss_feed() -> Response {
    let base = "https://yukihamada.jp";
    let mut items = String::new();
    for (slug, raw) in yukihamada_app::data::blog::POSTS_JA {
        let (meta, body) = yukihamada_app::data::blog::parse_frontmatter(raw);
        let desc = if meta.description.is_empty() {
            body.chars().take(200).collect::<String>()
        } else {
            meta.description.clone()
        };
        items.push_str(&format!(
            "    <item>\n      <title><![CDATA[{title}]]></title>\n      <link>{base}/ja/blog/{slug}</link>\n      <guid>{base}/ja/blog/{slug}</guid>\n      <pubDate>{date}</pubDate>\n      <description><![CDATA[{desc}]]></description>\n    </item>\n",
            title = meta.title,
            date = meta.date,
        ));
    }
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>濱田優貴 Blog</title>
    <link>{base}/ja</link>
    <description>技術、起業、柔術、そしてAIエージェントについて</description>
    <language>ja</language>
    <atom:link href="{base}/feed.xml" rel="self" type="application/rss+xml" />
{items}  </channel>
</rss>
"#
    );
    (
        [("content-type", "application/rss+xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

async fn robots() -> Response {
    let body = "User-agent: *\nAllow: /\nSitemap: https://yukihamada.jp/sitemap.xml\n\n# RSS Feed\n# https://yukihamada.jp/feed.xml\n";
    (
        [("content-type", "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    lang: String,
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
}

const SYSTEM_PROMPT: &str = "あなたは濱田優貴（Yuki Hamada）の個人サイトのAIアシスタントです。\n\
訪問者からの質問に、濱田優貴本人になりきって親しみやすく答えてください。\n\
\n\
【濱田優貴について】\n\
- 株式会社イネブラ 代表取締役CEO\n\
- 元メルカリ 取締役・CPO・CINO\n\
- エンジェル投資家\n\
- 令和トラベル社外取締役、NOT A HOTELコファウンダー（元取締役）\n\
- 技術スタック: Rust, Swift, TypeScript, WebAssembly, Axum, Leptos, React, SwiftUI\n\
- インフラ: AWS Lambda, Fly.io, Docker, SQLite, DynamoDB\n\
- プロダクト: chatweb.ai（マルチモデルAIチャット）, elio.love（iOS AIエージェント）,\n\
  jiuflow.art（柔術学習）, banto.work（建設業DX）, enabler.fun（バケーションレンタル）\n\
- 趣味: ブラジリアン柔術（BJJ）, 音楽制作（AI作曲×ギター）\n\
- 連絡先: mail@yukihamada.jp\n\
- X（Twitter）: @yukihamada\n\
\n\
簡潔かつ温かみのある口調で、100〜200文字程度で答えてください。\n\
英語で質問されたら英語で、日本語なら日本語で答えてください。";

async fn chat(Json(req): Json<ChatRequest>) -> Result<Json<ChatResponse>, StatusCode> {
    let api_key = std::env::var("CHATWEB_API_KEY")
        .unwrap_or_else(|_| "cw_41498c0b092348ac90fbb1f5ea0e2c44".to_string());

    // Sanitize user message for shell safety
    let safe_msg = req.message.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        r#"{{"model":"auto","messages":[{{"role":"system","content":{system}}},{{"role":"user","content":"{safe_msg}"}}],"max_tokens":300,"temperature":0.7}}"#,
        system = serde_json::to_string(SYSTEM_PROMPT).unwrap_or_default(),
    );

    let output = tokio::process::Command::new("curl")
        .args([
            "-s", "--max-time", "20",
            "-X", "POST",
            "-H", &format!("Authorization: Bearer {api_key}"),
            "-H", "Content-Type: application/json",
            "-d", &body,
            "https://api.chatweb.ai/v1/chat/completions",
        ])
        .output()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let reply = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("すみません、うまく答えられませんでした。")
        .to_string();

    Ok(Json(ChatResponse { reply }))
}

#[tokio::main]
async fn main() {
    load_dotenv();
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/", get(redirect_root))
        .route("/api/chat", post(chat))
        .route("/sitemap.xml", get(sitemap))
        .route("/feed.xml", get(rss_feed))
        .route("/robots.txt", get(robots))
        .route("/health", get(health))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || yukihamada_app::shell(leptos_options.clone())
        })
        .fallback(fallback)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
