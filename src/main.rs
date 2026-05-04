mod blog;

use axum::{
    extract::{Path, Query, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{header, StatusCode, HeaderMap, Method},
    response::{Html, IntoResponse, Json, Redirect, Response, sse::{Event, Sse}},
    routing::{get, post},
    Router,
};
use askama::Template;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::{Read as IoRead, Write as IoWrite};
use tokio_stream::StreamExt as _;
use tokio::sync::broadcast;
#[allow(unused_imports)]
use async_stream::stream;
use tower_http::{compression::CompressionLayer, services::ServeDir};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

// ── Shared PTY for admin terminal ──

struct PtyRunning {
    writer: Box<dyn IoWrite + Send>,
    master: Box<dyn MasterPty + Send>,
}

struct SharedPty {
    tx: broadcast::Sender<Vec<u8>>,
    inner: Arc<Mutex<Option<PtyRunning>>>,
}

impl SharedPty {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(512);
        SharedPty { tx, inner: Arc::new(Mutex::new(None)) }
    }

    fn ensure_started(&self, anthropic_key: Option<String>) {
        let mut g = self.inner.lock().unwrap();
        if g.is_some() { return; }

        let size = PtySize { rows: 24, cols: 220, pixel_width: 0, pixel_height: 0 };
        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(size) {
            Ok(p) => p,
            Err(e) => { eprintln!("PTY open error: {e}"); return; }
        };

        let mut cmd = CommandBuilder::new("/bin/bash");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("HOME", "/root");
        cmd.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
        if let Some(key) = anthropic_key {
            cmd.env("ANTHROPIC_API_KEY", key);
        }

        let _child = match pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => { eprintln!("PTY spawn error: {e}"); return; }
        };
        drop(pair.slave);

        let reader: Box<dyn std::io::Read + Send> = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => { eprintln!("PTY reader error: {e}"); return; }
        };
        let writer: Box<dyn std::io::Write + Send> = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => { eprintln!("PTY writer error: {e}"); return; }
        };

        *g = Some(PtyRunning { writer, master: pair.master });
        drop(g);

        let tx_clone = self.tx.clone();
        let inner_clone = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => { let _ = tx_clone.send(buf[..n].to_vec()); }
                }
            }
            inner_clone.lock().unwrap().take();
            println!("PTY session ended");
        });
    }

    fn write_input(&self, data: &[u8]) {
        let mut g = self.inner.lock().unwrap();
        if let Some(ref mut r) = *g {
            let _ = r.writer.write_all(data);
            let _ = r.writer.flush();
        }
    }

    fn resize(&self, cols: u16, rows: u16) {
        let g = self.inner.lock().unwrap();
        if let Some(ref r) = *g {
            let _ = r.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        }
    }
}

// ── Video ──
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct VideoMeta {
    id: String,
    title: String,
    is_public: bool,
    uploader: String,
    size_bytes: u64,
    created_at: u64,
    mime_type: String,
}

const VIDEO_META_FILE: &str = "/data/videos_meta.json";
const VIDEO_DIR: &str = "/data/videos";
const MAX_VIDEO_BYTES: usize = 150 * 1024 * 1024; // 150 MB

fn load_video_meta() -> Vec<VideoMeta> {
    std::fs::read_to_string(VIDEO_META_FILE).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_video_meta(videos: &[VideoMeta]) {
    if let Ok(j) = serde_json::to_string(videos) {
        let _ = std::fs::write(VIDEO_META_FILE, j);
    }
}

// ── Data ──

struct AppState {
    posts: Vec<blog::BlogPost>,
    tags: Vec<(String, usize)>,
    stripe_key: Option<String>,
    resend_key: Option<String>,
    stripe_webhook_secret: Option<String>,
    charin_api_key: Option<String>,
    newsletter_admin_token: Option<String>,
    anthropic_key: Option<String>,
    // Fanclub auth: OTP store (email → (code, expiry_secs_since_epoch, attempts))
    otp_store: Mutex<HashMap<String, (String, u64, u8)>>,
    // Session store (token → (email, expiry_secs_since_epoch))
    fanclub_sessions: Mutex<HashMap<String, (String, u64)>>,
    // OTP rate limit: IP → list of request timestamps (secs since epoch)
    otp_rate_limit: Mutex<HashMap<String, Vec<u64>>>,
    // Dashboard session store: token → expiry_secs_since_epoch
    dash_sessions: Mutex<HashMap<String, u64>>,
    // Newsletter admin auth: IP → (failed_attempts, window_start_secs)
    newsletter_auth_attempts: Mutex<HashMap<String, (u32, u64)>>,
    // Chat rate limit: IP → list of request timestamps (secs since epoch)
    chat_rate_limit: Mutex<HashMap<String, Vec<u64>>>,
    // MCP rate limit (separate bucket)
    mcp_rate_limit: Mutex<HashMap<String, Vec<u64>>>,
    // Optional MCP API key for authenticated access
    mcp_key: Option<String>,
    // m5 HITL server URL (auto-registered by m5)
    m5_url: Mutex<Option<String>>,
    // Token used by m5 to register its URL
    m5_register_token: Option<String>,
    // Bearer token to authenticate requests to m5-hitl /ask
    m5_hitl_token: Option<String>,
    // Per-user chat memory: user_id → Vec<ChatMsg>, last 40 messages kept
    user_memory: Mutex<HashMap<String, Vec<ChatMsg>>>,
    // Admin sessions (token → (email, expiry))
    admin_sessions: Mutex<HashMap<String, (String, u64)>>,
    // General user sessions (token → (email, expiry))
    user_sessions: Mutex<HashMap<String, (String, u64)>>,
    // Shared PTY session for admin terminal
    shared_pty: Arc<SharedPty>,
    // Telegram bot token for admin notifications
    telegram_token: Option<String>,
    // Groq API key for voice transcription
    groq_api_key: Option<String>,
    // Broadcast channel for notifying admin SSE stream of new chat messages
    chat_notify_tx: broadcast::Sender<String>,
    // Pending admin replies keyed by visitor session_id
    pending_admin_replies: Mutex<HashMap<String, String>>,
    // Owner (Yuki) last heartbeat unix timestamp (0 = never)
    owner_last_seen: AtomicU64,
    // Live-chat sessions waiting for owner reply: session_id → reply sender
    pending_live_chats: Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>,
    // Video gallery metadata
    videos: Mutex<Vec<VideoMeta>>,
    // Gmail OAuth
    gmail_client_id: Option<String>,
    gmail_client_secret: Option<String>,
    gmail_refresh_token: Option<String>,
    gmail_email: Option<String>,
    // Cached Gmail access token: (token, expires_at_secs)
    gmail_access_token: Mutex<Option<(String, u64)>>,
}

const SESSIONS_FILE: &str = "/data/sessions.json";

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PersistedSessions {
    fanclub: HashMap<String, (String, u64)>,
    dash: HashMap<String, u64>,
}

fn load_sessions() -> PersistedSessions {
    std::fs::read_to_string(SESSIONS_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist_sessions(fanclub: &HashMap<String, (String, u64)>, dash: &HashMap<String, u64>) {
    let p = PersistedSessions { fanclub: fanclub.clone(), dash: dash.clone() };
    if let Ok(json) = serde_json::to_string(&p) {
        let _ = std::fs::write(SESSIONS_FILE, json);
    }
}

struct Project {
    name: &'static str,
    url: &'static str,
    desc_ja: &'static str,
    metrics: &'static str,
    logo: &'static str,
    app_url: &'static str,
}

const PROJECTS: &[Project] = &[
    Project {
        name: "chatweb.ai",
        url: "https://chatweb.ai",
        desc_ja: "Claude・Geminiをブラウザから直接使えるAIターミナル。API料金30%マージンで課金、Rustでゼロからビルド",
        metrics: "月間数千会話",
        logo: "/assets/logos/chatweb.svg",
        app_url: "",
    },
    Project {
        name: "Elio",
        url: "https://elio.love",
        desc_ja: "サーバーに送らず端末内で動くAIアシスタント。プライバシー重視のユーザー向けに Qwen3 を iOS 上でネイティブ動作",
        metrics: "App Store 公開中",
        logo: "/assets/logos/elio.png",
        app_url: "https://apps.apple.com/jp/app/elio-chat/id6757635481",
    },
    Project {
        name: "パシャ",
        url: "https://pasha.run",
        desc_ja: "領収書を撮るだけで自動仕訳、電子帳簿保存法の全8要件に対応。ブロックチェーンで改ざん防止",
        metrics: "App Store 審査中",
        logo: "",
        app_url: "https://testflight.apple.com/join/CTmyqV6H",
    },
    Project {
        name: "StayFlow",
        url: "https://stayflowapp.com",
        desc_ja: "民泊・旅館オーナーが予約管理とゲスト対応を一元化。AIが返答文を自動生成してオーナーの時間を返す",
        metrics: "1,860 UV/月 · 500+ 施設",
        logo: "",
        app_url: "",
    },
    Project {
        name: "BANTO",
        url: "https://banto.work",
        desc_ja: "建設業の請求書・見積書をデジタル化し、即払いファクタリングでキャッシュフローを改善",
        metrics: "500+ 社導入",
        logo: "/assets/logos/banto.png",
        app_url: "",
    },
    Project {
        name: "JiuFlow",
        url: "https://jiuflow.art",
        desc_ja: "柔術テクニックをグラフ構造でマッピング。技の繋がりを可視化して体系的に学べる",
        metrics: "140+ 大会データ収録",
        logo: "/assets/logos/jiuflow.png",
        app_url: "",
    },
    Project {
        name: "Koe Device",
        url: "https://koe.live",
        desc_ja: "フェス会場に配置したデバイスが音声を同期し、群衆が「楽器」になる体験を作る。ESP32+Rustで自作",
        metrics: "4形態 (SUB/FILL/COIN/STAGE)",
        logo: "",
        app_url: "",
    },
    Project {
        name: "Koe",
        url: "https://koe.elio.love",
        desc_ja: "ウェイクワードで起動するオンデバイスAI音声入力。macOS・iOS・Windows対応",
        metrics: "Mac / iOS / Windows",
        logo: "",
        app_url: "",
    },
    Project {
        name: "SOLUNA",
        url: "https://solun.art",
        desc_ja: "北海道弟子屈のリゾート会員制ホテル。SOLUNAプロトコルで体験を共有・記録する",
        metrics: "北海道弟子屈",
        logo: "",
        app_url: "",
    },
    Project {
        name: "ミセバンAI",
        url: "https://misebanai.com",
        desc_ja: "実店舗向けAI接客エージェント。来店者への商品提案・FAQ対応をAIが担当",
        metrics: "β提供中",
        logo: "",
        app_url: "",
    },
    Project {
        name: "enablerdao",
        url: "https://enablerdao.com",
        desc_ja: "ライフスタイル×Web3コミュニティ。ENAIトークン(Solana)でガバナンス参加",
        metrics: "Solana mainnet",
        logo: "/assets/logos/enabler.png",
        app_url: "",
    },
];

struct Career {
    company: &'static str,
    role: &'static str,
    period: &'static str,
    desc: &'static str,
    url: &'static str,
    logo: &'static str,
    highlight: bool,
}

const CAREERS: &[Career] = &[
    Career {
        company: "イネブラ",
        role: "代表取締役CEO",
        period: "2024〜",
        desc: "AI・スマートホーム・フィンテック領域で11+ プロダクトを自ら開発・運営。Rust/Swift/Reactで全てコードを書く創業型CEO",
        url: "https://enablerhq.com",
        logo: "/assets/logos/enabler.png",
        highlight: true,
    },
    Career {
        company: "令和トラベル",
        role: "社外取締役",
        period: "2024〜",
        desc: "NEWT — AI旅行エージェント。2021年創業、数十億円調達。プロダクト戦略の外部視点として参画",
        url: "https://newt.net",
        logo: "/assets/logos/reiwa.png",
        highlight: false,
    },
    Career {
        company: "NOT A HOTEL",
        role: "共同創業者・元取締役",
        period: "2018〜2024",
        desc: "「所有」と「宿泊」の概念を変えた会員制別荘サービス。数百億規模の不動産×テックビジネスを共同創業",
        url: "https://notahotel.com",
        logo: "/assets/logos/notahotel.png",
        highlight: false,
    },
    Career {
        company: "メルカリ",
        role: "取締役 CPO / CINO",
        period: "2014〜2021",
        desc: "日本最大のC2Cマーケット。MAU 2000万超・累計出品10億件超へ成長させた。米国展開のプロダクト責任者も兼任",
        url: "",
        logo: "/assets/logos/mercari.png",
        highlight: false,
    },
    Career {
        company: "サイブリッジ",
        role: "共同創業者・取締役",
        period: "2003〜2013",
        desc: "塾講師ナビ・日本最大級の学習塾口コミサービスを開発。10年かけてM&Aで事業売却",
        url: "",
        logo: "",
        highlight: false,
    },
];

struct Track {
    title: &'static str,
    src: &'static str,
    artwork: &'static str,
}

const TRACKS: &[Track] = &[
    Track { title: "Free to Change", src: "/audio/free-to-change.mp3", artwork: "/assets/album-free-to-change.jpg" },
    Track { title: "HELLO 2150", src: "/audio/hello-2150.mp3", artwork: "/assets/album-hello-2150.jpg" },
    Track { title: "Everybody say BJJ", src: "/audio/everybody-say-bjj.mp3", artwork: "/assets/album-everybody-bjj.jpg" },
    Track { title: "I Love You", src: "/audio/i-love-you.mp3", artwork: "/assets/album-i-love-you.jpg" },
    Track { title: "I Need Your Attention", src: "/audio/i-need-your-attention.mp3", artwork: "/assets/album-attention.jpg" },
    Track { title: "それ恋じゃなく柔術でした", src: "/audio/sore-koi-janaku-jujutsu.mp3", artwork: "/assets/album-koi-jujutsu.jpg" },
    Track { title: "塩とピクセル", src: "/audio/shio-to-pixel.mp3", artwork: "/assets/album-shio-pixel.jpg" },
    Track { title: "結び直す朝", src: "/audio/musubinaosu-asa.mp3", artwork: "/assets/album-musubinaosu.jpg" },
];

// ── Templates ──

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    projects: &'a [Project],
    careers: &'a [Career],
    posts: &'a [blog::BlogPost],
    tracks: &'a [Track],
}

#[derive(Template)]
#[template(path = "blog_list.html")]
struct BlogListTemplate<'a> {
    posts: &'a [blog::BlogPost],
    tags: &'a [(String, usize)],
    filter_tag: Option<String>,
}

#[derive(Template)]
#[template(path = "blog_post.html")]
struct BlogPostTemplate<'a> {
    post: &'a blog::BlogPost,
    prev_post: Option<&'a blog::BlogPost>,
    next_post: Option<&'a blog::BlogPost>,
    related_posts: Vec<&'a blog::BlogPost>,
    og_image: String,
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate;

#[derive(Template)]
#[template(path = "mcp.html")]
struct McpPageTemplate {
    posts_count: usize,
}

async fn mcp_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tmpl = McpPageTemplate { posts_count: state.posts.len() };
    Html(tmpl.render().unwrap_or_default())
}

// ── Handlers ──

async fn home(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    // CLI clients (curl, wget, httpie, ...) → serve an ANSI-colored terminal page
    let ua = headers.get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let accept = headers.get(header::ACCEPT)
        .and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let is_cli = (ua.starts_with("curl/") || ua.starts_with("wget/")
        || ua.starts_with("httpie/") || ua.contains("fetch/") || ua.contains("libwww"))
        && !accept.contains("text/html");

    if is_cli {
        let body = render_cli_home(&state.posts);
        return (
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            body,
        ).into_response();
    }

    let tmpl = HomeTemplate {
        projects: PROJECTS,
        careers: CAREERS,
        posts: &state.posts[..state.posts.len().min(5)],
        tracks: TRACKS,
    };
    Html(tmpl.render().unwrap_or_default()).into_response()
}

fn render_cli_home(posts: &[blog::BlogPost]) -> String {
    // ANSI escape codes
    let reset = "\x1b[0m";
    let bold = "\x1b[1m";
    let dim = "\x1b[2m";
    let accent = "\x1b[38;5;215m";   // warm orange (#e8a04c-ish)
    let cyan = "\x1b[38;5;117m";
    let green = "\x1b[38;5;150m";
    let muted = "\x1b[38;5;244m";
    let pink = "\x1b[38;5;212m";

    let banner = format!("\
{a}      __  __ ___  _________                         __      {r}
{a}  __ / // //  _/ / ___/ __ \\____ __ __ _____ _____ / /____  {r}
{a} /  / // /_/ /  / /  / /_/ / __// // //  ' // _  // __/ _ \\ {r}
{a}/__/_//_//___/  \\___/\\____/ \\__/ \\_, //_/|_/ \\_,_/\\__/\\___/ {r}
{a}                                /___/                       {r}
", a = accent, r = reset);

    // Simpler fallback banner using block characters
    let banner2 = format!("\
{bold}{accent}
 ██╗   ██╗██╗   ██╗██╗  ██╗██╗    ██╗  ██╗ █████╗ ███╗   ███╗ █████╗ ██████╗  █████╗
 ╚██╗ ██╔╝██║   ██║██║ ██╔╝██║    ██║  ██║██╔══██╗████╗ ████║██╔══██╗██╔══██╗██╔══██╗
  ╚████╔╝ ██║   ██║█████╔╝ ██║    ███████║███████║██╔████╔██║███████║██║  ██║███████║
   ╚██╔╝  ██║   ██║██╔═██╗ ██║    ██╔══██║██╔══██║██║╚██╔╝██║██╔══██║██║  ██║██╔══██║
    ██║   ╚██████╔╝██║  ██╗██║    ██║  ██║██║  ██║██║ ╚═╝ ██║██║  ██║██████╔╝██║  ██║
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═╝    ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝
{reset}");
    let _ = banner;

    let mut s = String::new();
    s.push_str(&banner2);
    s.push_str(&format!(
        "\n  {accent}{bold}濱田優貴{reset}  {dim}//{reset}  {accent}Yuki Hamada{reset}\n"
    ));
    s.push_str(&format!(
        "  {dim}建てて、残して、いいやつと。{reset}\n"
    ));
    s.push_str(&format!("  {muted}Enabler CEO / ex-Mercari CPO / 柔術青帯{reset}\n\n"));

    s.push_str(&format!("  {cyan}{bold}▎PROJECTS{reset}\n"));
    let projects = [
        ("Soluna",     "solun.art",     "共同所有型リゾート + FEST HAWAII 2026"),
        ("JiuFlow",    "jiuflow.art",   "柔術テクニックマッピング"),
        ("Koe",        "koe.live",      "群衆を楽器にするデバイス"),
        ("chatweb.ai", "chatweb.ai",    "マルチモデル AI チャット"),
        ("パシャ",      "pasha.run",     "AI OCR 経費管理"),
    ];
    for (name, url, desc) in projects {
        s.push_str(&format!("    {accent}▶{reset} {bold}{:<11}{reset} {muted}{:<14}{reset} {}\n",
            name, url, desc));
    }
    s.push('\n');

    s.push_str(&format!("  {cyan}{bold}▎RECENT POSTS{reset}\n"));
    for p in posts.iter().take(5) {
        let title: String = p.title.chars().take(50).collect();
        s.push_str(&format!("    {muted}{}{reset}  {}\n    {dim}   → yukihamada.jp/blog/{}{reset}\n",
            p.date, title, p.slug));
    }
    s.push('\n');

    s.push_str(&format!("  {cyan}{bold}▎AI CHAT (try it!){reset}\n"));
    s.push_str(&format!("    {green}$ curl -sN https://yukihamada.jp/api/chat \\{reset}\n"));
    s.push_str(&format!("    {green}    -H 'Content-Type: application/json' \\{reset}\n"));
    s.push_str(&format!("    {green}    -d '{{\"messages\":[{{\"role\":\"user\",\"content\":\"your question\"}}]}}'{reset}\n\n"));

    s.push_str(&format!("  {cyan}{bold}▎MCP & A2A (for agents){reset}\n"));
    s.push_str(&format!("    MCP  {muted}→{reset} https://yukihamada.jp/mcp\n"));
    s.push_str(&format!("    A2A  {muted}→{reset} https://yukihamada.jp/.well-known/agent.json\n\n"));

    s.push_str(&format!("  {cyan}{bold}▎CONTACT{reset}\n"));
    s.push_str(&format!("    {pink}✉{reset}  mail@yukihamada.jp\n"));
    s.push_str(&format!("    {pink}𝕏{reset}  @yukihamada\n"));
    s.push_str(&format!("    {pink}⎇{reset}  github.com/yukihamada\n\n"));

    s.push_str(&format!("  {dim}──────────────────────────────────────────────────{reset}\n"));
    s.push_str(&format!("  {dim}  Visit:{reset} {accent}https://yukihamada.jp{reset}\n"));
    s.push_str(&format!("  {dim}  Source of this terminal page:{reset} {muted}github.com/yukihamada{reset}\n\n"));
    s
}

async fn blog_list() -> impl IntoResponse {
    Redirect::permanent("/#blog")
}

async fn blog_list_tag(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    let fetch_mode = headers.get("sec-fetch-mode")
        .and_then(|v| v.to_str().ok()).unwrap_or("");
    if fetch_mode == "navigate" {
        let tmpl = HomeTemplate {
            projects: PROJECTS,
            careers: CAREERS,
            posts: &state.posts[..state.posts.len().min(5)],
            tracks: TRACKS,
        };
        let mut html = tmpl.render().unwrap_or_default();
        let script = r#"<script>window.__autoOpenApp="blog";</script>"#;
        html = html.replace("</body>", &format!("{script}\n</body>"));
        return Html(html).into_response();
    }
    if let Some(tag) = params.get("tag") {
        Redirect::permanent(&format!("/#blog?tag={tag}")).into_response()
    } else {
        Redirect::permanent("/#blog").into_response()
    }
}

async fn blog_post(
    Path(slug): Path<String>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    let fetch_mode = headers.get("sec-fetch-mode")
        .and_then(|v| v.to_str().ok()).unwrap_or("");
    // Crawlers (Googlebot etc.) send no Sec-Fetch-* headers → serve actual HTML for indexing.
    // openPost fetch() sends Sec-Fetch-Mode: cors/no-cors → serve actual HTML for window content.
    // Browser navigation → serve home page with auto-open script (avoids redirect+hash race).
    if fetch_mode == "navigate" {
        let tmpl = HomeTemplate {
            projects: PROJECTS,
            careers: CAREERS,
            posts: &state.posts[..state.posts.len().min(5)],
            tracks: TRACKS,
        };
        let safe_slug = slug.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>();
        let mut html = tmpl.render().unwrap_or_default();
        let script = format!(r#"<script>window.__autoOpenSlug="{safe_slug}";</script>"#);
        html = html.replace("</body>", &format!("{script}\n</body>"));
        return Html(html).into_response();
    }

    // Serve actual blog post HTML (for openPost fetch or same-origin requests)
    let posts = &state.posts;
    let idx = posts.iter().position(|p| p.slug == slug);
    match idx {
        Some(i) => {
            let post = &posts[i];
            let prev_post = posts.get(i + 1);
            let next_post = if i > 0 { posts.get(i - 1) } else { None };
            let related_posts: Vec<&blog::BlogPost> = posts.iter()
                .filter(|p| p.slug != slug && p.tags.iter().any(|t| post.tags.contains(t)))
                .take(3)
                .collect();
            let ogp_path = format!("public/blog/images/{}-ogp.jpg", post.slug);
            let og_image = if std::path::Path::new(&ogp_path).exists() {
                format!("https://yukihamada.jp/blog/images/{}-ogp.jpg", post.slug)
            } else {
                "https://yukihamada.jp/og-image.jpg".to_string()
            };
            let tmpl = BlogPostTemplate { post, prev_post, next_post, related_posts, og_image };
            Html(tmpl.render().unwrap_or_default()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn blog_soluna_proxy(Path(slug): Path<String>) -> Response {
    Redirect::permanent(&format!("/#blog/soluna/{slug}")).into_response()
}


async fn redirect_root() -> Redirect {
    Redirect::permanent("/")
}

async fn redirect_terminal() -> Redirect { Redirect::permanent("/#terminal") }
async fn redirect_projects() -> Redirect { Redirect::permanent("/#projects") }
async fn redirect_career()   -> Redirect { Redirect::permanent("/#career") }
async fn redirect_music()    -> Redirect { Redirect::permanent("/#music") }
async fn redirect_browser()  -> Redirect { Redirect::permanent("/#browser") }
async fn redirect_koe()      -> Redirect { Redirect::permanent("/#koe") }
async fn redirect_uta()      -> Redirect { Redirect::permanent("/#uta") }
async fn redirect_news()     -> Redirect { Redirect::permanent("/#news") }
async fn redirect_settings() -> Redirect { Redirect::permanent("/#settings") }
async fn redirect_camera()   -> Redirect { Redirect::permanent("/#camera") }
async fn redirect_game()     -> Redirect { Redirect::permanent("/#game") }
async fn redirect_finder()   -> Redirect { Redirect::permanent("/#finder") }
async fn redirect_contact()  -> Redirect { Redirect::permanent("/#contact") }
async fn redirect_now()      -> Redirect { Redirect::permanent("/#now") }
async fn redirect_podcast()  -> Redirect { Redirect::permanent("/#podcast") }

async fn about() -> impl IntoResponse {
    Html(AboutTemplate.render().unwrap_or_default())
}

async fn soluna_page() -> impl IntoResponse {
    Html(r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>SOLUNA — 濱田優貴</title>
<meta name="robots" content="noindex,nofollow">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
<style>
*{margin:0;padding:0;box-sizing:border-box}
:root{--gold:#c8a455;--bg:#080806;--surface:rgba(255,255,255,.04);--line:rgba(255,255,255,.08)}
body{background:var(--bg);color:#e8e8e4;font-family:'Inter',sans-serif;min-height:100vh}
.topbar{display:flex;align-items:center;justify-content:space-between;padding:16px 24px;border-bottom:1px solid var(--line)}
.topbar-logo{font-size:.75rem;font-weight:800;letter-spacing:.2em;color:var(--gold);text-decoration:none}
.topbar-nav{display:flex;gap:12px;align-items:center}
.topbar-nav a{font-size:.75rem;color:rgba(255,255,255,.5);text-decoration:none;padding:6px 10px;border-radius:4px;transition:color .15s}
.topbar-nav a:hover{color:#fff}
.topbar-share{background:rgba(200,164,85,.12);border:1px solid rgba(200,164,85,.3);color:var(--gold);font-size:.7rem;font-weight:600;letter-spacing:.08em;padding:6px 12px;border-radius:4px;cursor:pointer;transition:background .15s}
.topbar-share:hover{background:rgba(200,164,85,.2)}
main{max-width:640px;margin:0 auto;padding:48px 24px}
/* Auth card */
.auth-card{background:var(--surface);border:1px solid var(--line);border-radius:12px;padding:36px 32px;margin-bottom:32px}
.auth-logo{font-size:.7rem;font-weight:800;letter-spacing:.25em;color:var(--gold);margin-bottom:24px}
h1{font-size:1.3rem;font-weight:800;color:#f0ece4;margin-bottom:8px}
.sub{font-size:.8rem;color:rgba(255,255,255,.35);margin-bottom:24px;line-height:1.6}
label{display:block;font-size:.65rem;letter-spacing:.15em;color:#555;text-transform:uppercase;margin-bottom:6px}
input{width:100%;background:#0a0908;border:1px solid var(--line);color:#c8c0b0;padding:12px 14px;border-radius:6px;font-size:.85rem;outline:none;margin-bottom:12px}
input:focus{border-color:var(--gold)}
.btn{width:100%;background:var(--gold);color:#080806;font-weight:800;font-size:.78rem;letter-spacing:.1em;padding:14px;border:none;border-radius:6px;cursor:pointer;transition:background .15s}
.btn:hover{background:#d4b068}
.msg{margin-top:12px;font-size:.78rem;min-height:18px;text-align:center}
.msg.ok{color:#4a9a5a}.msg.err{color:#e05555}
#step2{display:none}
.back-btn{font-size:.72rem;color:#555;text-decoration:underline;cursor:pointer;background:none;border:none;margin-top:10px;display:block;text-align:center}
/* Member panel */
#member-panel{display:none}
.member-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:24px}
.member-name{font-size:1.1rem;font-weight:700;color:#f0ece4}
.member-tag{font-size:.65rem;font-weight:700;letter-spacing:.1em;background:rgba(200,164,85,.15);color:var(--gold);padding:3px 10px;border-radius:20px;border:1px solid rgba(200,164,85,.3)}
.logout-btn{font-size:.7rem;color:rgba(255,255,255,.3);text-decoration:underline;cursor:pointer;background:none;border:none}
.section-title{font-size:.65rem;font-weight:700;letter-spacing:.15em;color:rgba(255,255,255,.35);text-transform:uppercase;margin:28px 0 12px}
.prop-card{background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:16px 18px;margin-bottom:8px;display:flex;align-items:center;justify-content:space-between}
.prop-name{font-size:.85rem;font-weight:600;color:#e8e8e4}
.prop-units{font-size:.72rem;color:rgba(255,255,255,.35)}
.prop-link{font-size:.72rem;color:var(--gold);text-decoration:none}
.quick-links{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-top:8px}
.quick-link{background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:14px 16px;text-decoration:none;color:#c8c0b0;font-size:.78rem;font-weight:500;transition:border-color .15s}
.quick-link:hover{border-color:rgba(200,164,85,.4);color:#fff}
.quick-link .ql-icon{font-size:1rem;margin-bottom:6px;display:block}
/* Toast */
#toast{position:fixed;bottom:24px;right:24px;background:#1a1a18;border:1px solid rgba(200,164,85,.4);color:var(--gold);font-size:.78rem;padding:10px 16px;border-radius:8px;opacity:0;transition:opacity .25s;pointer-events:none;z-index:9999}
#toast.show{opacity:1}
</style>
</head>
<body>
<div class="topbar">
  <a class="topbar-logo" href="https://solun.art">SOLUNA</a>
  <div class="topbar-nav">
    <a href="/">← 濱田優貴</a>
    <a href="https://solun.art/homes">物件一覧</a>
    <button class="topbar-share" onclick="sharePage()">🔗 このページを共有</button>
  </div>
</div>

<main>
  <!-- Auth card -->
  <div class="auth-card" id="auth-card">
    <div class="auth-logo">SOLUNA</div>
    <h1>ログイン</h1>
    <p class="sub">SOLUNAメンバーの方はメールアドレスでログインできます。</p>
    <div id="step1">
      <label>メールアドレス</label>
      <input type="email" id="email" placeholder="your@email.com" autocomplete="email">
      <button class="btn" onclick="sendOtp()">コードを送信</button>
      <div class="msg" id="msg1"></div>
    </div>
    <div id="step2">
      <label>確認コード（6桁）</label>
      <input type="text" id="code" placeholder="000000" maxlength="6" inputmode="numeric">
      <button class="btn" onclick="verifyOtp()">ログイン</button>
      <button class="back-btn" onclick="document.getElementById('step1').style.display='';document.getElementById('step2').style.display='none'">← メールを変更</button>
      <div class="msg" id="msg2"></div>
    </div>
  </div>

  <!-- Member panel (shown after login) -->
  <div id="member-panel">
    <div class="member-header">
      <div>
        <div class="member-name" id="member-name">—</div>
        <div style="font-size:.72rem;color:rgba(255,255,255,.3);margin-top:3px" id="member-email"></div>
      </div>
      <div style="display:flex;gap:10px;align-items:center">
        <span class="member-tag" id="member-type-tag">MEMBER</span>
        <button class="logout-btn" onclick="logout()">ログアウト</button>
      </div>
    </div>

    <div class="section-title">所有物件</div>
    <div id="properties-list"><p style="font-size:.8rem;color:rgba(255,255,255,.3)">物件情報を取得中...</p></div>

    <div class="section-title">クイックリンク</div>
    <div class="quick-links">
      <a class="quick-link" href="https://solun.art/app.html">
        <span class="ql-icon">🏡</span>予約・管理アプリ
      </a>
      <a class="quick-link" href="https://solun.art/homes">
        <span class="ql-icon">🏘️</span>全物件一覧
      </a>
      <a class="quick-link" href="https://solun.art/community">
        <span class="ql-icon">💬</span>コミュニティ
      </a>
      <a class="quick-link" href="https://solun.art/calendar">
        <span class="ql-icon">📅</span>稼働カレンダー
      </a>
    </div>
  </div>
</main>

<div id="toast"></div>

<script>
const SOLUNA = 'https://solun.art';

function toast(msg, ms=2500) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), ms);
}

function sharePage() {
  const url = location.href;
  if (navigator.share) {
    navigator.share({ title: 'SOLUNA — 濱田優貴', url });
  } else {
    navigator.clipboard.writeText(url).then(() => toast('URLをコピーしました'));
  }
}

function setMsg(id, cls, txt) {
  const el = document.getElementById(id);
  el.className = 'msg' + (cls ? ' ' + cls : '');
  el.textContent = txt;
}

async function sendOtp() {
  const email = document.getElementById('email').value.trim();
  if (!email) { setMsg('msg1', 'err', 'メールアドレスを入力してください'); return; }
  setMsg('msg1', '', '送信中...');
  const r = await fetch(SOLUNA + '/api/soluna/otp', {
    method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({ email })
  });
  const d = await r.json();
  if (r.ok) {
    document.getElementById('step1').style.display = 'none';
    document.getElementById('step2').style.display = '';
    setMsg('msg2', '', '');
  } else {
    setMsg('msg1', 'err', d.error || 'エラーが発生しました');
  }
}

async function verifyOtp() {
  const email = document.getElementById('email').value.trim();
  const code = document.getElementById('code').value.trim();
  if (!code) { setMsg('msg2', 'err', 'コードを入力してください'); return; }
  setMsg('msg2', '', '確認中...');
  const r = await fetch(SOLUNA + '/api/soluna/verify', {
    method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({ email, code })
  });
  const d = await r.json();
  if (r.ok) {
    localStorage.setItem('sln_token', d.token);
    setMsg('msg2', 'ok', 'ログインしました');
    setTimeout(() => showMember(d), 500);
  } else {
    setMsg('msg2', 'err', d.error || 'コードが正しくありません');
  }
}

async function loadMember(token) {
  const r = await fetch(SOLUNA + '/api/soluna/me', {
    headers: { 'Authorization': 'Bearer ' + token }
  });
  if (!r.ok) return null;
  return r.json();
}

function showMember(data) {
  document.getElementById('auth-card').style.display = 'none';
  document.getElementById('member-panel').style.display = '';
  document.getElementById('member-name').textContent = data.name || data.email || '—';
  document.getElementById('member-email').textContent = data.email || '';
  const typeTag = document.getElementById('member-type-tag');
  typeTag.textContent = (data.member_type || 'member').toUpperCase();

  // Properties
  const props = data.purchases || [];
  const pl = document.getElementById('properties-list');
  if (props.length === 0) {
    pl.innerHTML = '<p style="font-size:.8rem;color:rgba(255,255,255,.3)">所有物件なし</p>';
  } else {
    pl.innerHTML = props.map(p => `
      <div class="prop-card">
        <div>
          <div class="prop-name">${p.property_slug?.toUpperCase() || '—'}</div>
          <div class="prop-units">${p.units || 1}口 · ${p.status || ''}</div>
        </div>
        <a class="prop-link" href="${SOLUNA}/${p.property_slug || ''}">詳細 →</a>
      </div>`).join('');
  }
}

function logout() {
  localStorage.removeItem('sln_token');
  location.reload();
}

// Auto-login if token exists
(async () => {
  const token = localStorage.getItem('sln_token');
  if (!token) return;
  const data = await loadMember(token);
  if (data) showMember(data);
})();

document.getElementById('email').addEventListener('keydown', e => { if (e.key === 'Enter') sendOtp(); });
document.getElementById('code').addEventListener('keydown', e => { if (e.key === 'Enter') verifyOtp(); });
</script>
</body>
</html>"#)
}

async fn sitemap(State(state): State<Arc<AppState>>) -> Response {
    let base = "https://yukihamada.jp";
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    xml.push_str(&format!("  <url><loc>{base}/</loc><lastmod>{today}</lastmod><changefreq>weekly</changefreq><priority>1.0</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/about</loc><lastmod>{today}</lastmod><changefreq>monthly</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/blog</loc><lastmod>{today}</lastmod><changefreq>daily</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/projects</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/contact</loc><changefreq>yearly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/now</loc><lastmod>{today}</lastmod><changefreq>weekly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mcp</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    // Anime pages
    xml.push_str(&format!("  <url><loc>{base}/anime/</loc><changefreq>monthly</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep1.html</loc><changefreq>monthly</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep2.html</loc><changefreq>monthly</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep3.html</loc><changefreq>monthly</changefreq><priority>0.9</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep1-en.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep2-en.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/anime/ep3-en.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    // MV pages
    xml.push_str(&format!("  <url><loc>{base}/mv/</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/jiujitsu.html</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/tap.html</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/attention.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/hack.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/musubinaosu.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/claude-code.html</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/local-ai.html</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));
    xml.push_str(&format!("  <url><loc>{base}/mv/local-ai-ja.html</loc><changefreq>monthly</changefreq><priority>0.8</priority></url>\n"));

    for post in &state.posts {
        let loc = format!("{base}/blog/{}", post.slug);
        if post.date.is_empty() {
            xml.push_str(&format!(
                "  <url><loc>{loc}</loc><changefreq>yearly</changefreq><priority>0.7</priority></url>\n"
            ));
        } else {
            xml.push_str(&format!(
                "  <url><loc>{loc}</loc><lastmod>{date}</lastmod><changefreq>yearly</changefreq><priority>0.7</priority></url>\n",
                date = post.date,
            ));
        }
    }

    xml.push_str("</urlset>\n");
    ([("content-type", "application/xml; charset=utf-8")], xml).into_response()
}

fn date_to_rfc822(date: &str) -> String {
    // "2026-03-28" -> "Sat, 28 Mar 2026 00:00:00 +0900"
    if let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        use chrono::Datelike;
        let weekday = match d.weekday() {
            chrono::Weekday::Mon => "Mon", chrono::Weekday::Tue => "Tue",
            chrono::Weekday::Wed => "Wed", chrono::Weekday::Thu => "Thu",
            chrono::Weekday::Fri => "Fri", chrono::Weekday::Sat => "Sat",
            chrono::Weekday::Sun => "Sun",
        };
        let month = match d.month() {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
            5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
            9 => "Sep", 10 => "Oct", 11 => "Nov", _ => "Dec",
        };
        format!("{weekday}, {:02} {month} {} 00:00:00 +0900", d.day(), d.year())
    } else {
        date.to_string()
    }
}

async fn rss_feed(State(state): State<Arc<AppState>>) -> Response {
    let base = "https://yukihamada.jp";
    let mut items = String::new();
    for post in &state.posts {
        let pub_date = date_to_rfc822(&post.date);
        items.push_str(&format!(
            "    <item>\n      <title><![CDATA[{}]]></title>\n      <link>{base}/blog/{}</link>\n      <guid>{base}/blog/{}</guid>\n      <pubDate>{pub_date}</pubDate>\n      <description><![CDATA[{}]]></description>\n    </item>\n",
            post.title, post.slug, post.slug, post.description,
        ));
    }
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>濱田優貴</title>
    <link>{base}</link>
    <description>技術、起業、柔術について</description>
    <language>ja</language>
    <atom:link href="{base}/feed.xml" rel="self" type="application/rss+xml" />
{items}  </channel>
</rss>
"#
    );
    ([("content-type", "application/rss+xml; charset=utf-8")], xml).into_response()
}

async fn robots() -> Response {
    let body = "User-agent: *\nAllow: /\nSitemap: https://yukihamada.jp/sitemap.xml\n";
    ([("content-type", "text/plain; charset=utf-8")], body).into_response()
}

async fn health() -> &'static str {
    "ok"
}

async fn redirect_hamada_tokyo(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let host = req.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if host.contains("hamada.tokyo") {
        let path = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
        return Redirect::permanent(&format!("https://yukihamada.jp{path}")).into_response();
    }
    next.run(req).await
}

async fn security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert("strict-transport-security", "max-age=63072000; includeSubDomains; preload".parse().unwrap());
    h.insert("x-frame-options", "SAMEORIGIN".parse().unwrap());
    h.insert("x-content-type-options", "nosniff".parse().unwrap());
    h.insert("referrer-policy", "strict-origin-when-cross-origin".parse().unwrap());
    h.insert("permissions-policy", "camera=self, microphone=self, geolocation=()".parse().unwrap());
    h.insert("content-security-policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com https://enabler-analytics.fly.dev; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com; font-src https://fonts.gstatic.com https://cdnjs.cloudflare.com; img-src 'self' data: blob: https:; frame-src https://koe.live https://uta.live https://solun.art https://soluna-web.fly.dev https://chatweb.ai https://teai.io https://jiuflow.art https://stayflowapp.com https://banto.work https://pasha.run https://enablerdao.com https://misebanai.com https://news.xyz https://flow-anime.com https://kokon.tokyo https://yukihamada.jp https://yukihamada-jp.fly.dev https://enabler-analytics.fly.dev https://m5-dashboard.chatweb.ai; media-src 'self' blob:; connect-src 'self' https://enabler-analytics.fly.dev wss://yukihamada.jp wss://yukihamada-jp.fly.dev ws://localhost:8080 https://solun.art https://chatweb.ai wss://chatweb.ai"
        .parse().unwrap());
    // Cache-Control for static assets
    if path.starts_with("/assets/") || path.starts_with("/blog/images/") {
        h.insert("cache-control", "public, max-age=31536000, immutable".parse().unwrap()); // 1 year + immutable
    } else if path.starts_with("/audio/") {
        h.insert("cache-control", "public, max-age=3600".parse().unwrap()); // 1 hour — audio can be regenerated
    }
    res
}

// ── Fan Club API ──

fn cors_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "https://yukihamada.jp".parse().unwrap());
    h.insert(header::ACCESS_CONTROL_ALLOW_METHODS, "GET,POST,OPTIONS".parse().unwrap());
    h.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, "content-type".parse().unwrap());
    h
}

async fn options_cors() -> Response {
    (StatusCode::NO_CONTENT, cors_headers()).into_response()
}

#[derive(serde::Deserialize)]
struct EmailReq { email: String }

#[derive(serde::Serialize)]
struct FanclubRes { pro: bool, customer_id: Option<String>, plan: Option<String> }

async fn fanclub_verify(State(state): State<Arc<AppState>>, axum::Json(body): axum::Json<EmailReq>) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let Some(ref sk) = state.stripe_key else {
        return (cors_headers(), Json(FanclubRes { pro: false, customer_id: None, plan: None })).into_response();
    };

    let client = reqwest::Client::new();
    // Search customers by email
    let cust_res = client.get("https://api.stripe.com/v1/customers")
        .basic_auth(sk, None::<&str>)
        .query(&[("email", &email), ("limit", &"1".to_string())])
        .send().await;
    let Ok(cust_resp) = cust_res else {
        return (cors_headers(), Json(FanclubRes { pro: false, customer_id: None, plan: None })).into_response();
    };
    let cust_json: serde_json::Value = cust_resp.json().await.unwrap_or_default();
    let customers = cust_json["data"].as_array();
    let Some(custs) = customers else {
        return (cors_headers(), Json(FanclubRes { pro: false, customer_id: None, plan: None })).into_response();
    };
    if custs.is_empty() {
        return (cors_headers(), Json(FanclubRes { pro: false, customer_id: None, plan: None })).into_response();
    }
    let cid = custs[0]["id"].as_str().unwrap_or_default().to_string();

    // Check active subscriptions
    let sub_res = client.get("https://api.stripe.com/v1/subscriptions")
        .basic_auth(sk, None::<&str>)
        .query(&[("customer", &cid), ("status", &"active".to_string()), ("limit", &"1".to_string())])
        .send().await;
    let pro = if let Ok(sub_resp) = sub_res {
        let sub_json: serde_json::Value = sub_resp.json().await.unwrap_or_default();
        sub_json["data"].as_array().map_or(false, |a| !a.is_empty())
    } else { false };

    (cors_headers(), Json(FanclubRes {
        pro,
        customer_id: Some(cid),
        plan: if pro { Some("fanclub_1".into()) } else { None },
    })).into_response()
}

async fn newsletter_post(axum::Json(body): axum::Json<EmailReq>) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let line = format!("{}\t{}\n", chrono::Utc::now().to_rfc3339(), email);
    // Best-effort append
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/data/newsletter.txt")
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

async fn newsletter_get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    // Require Authorization: Bearer <NEWSLETTER_ADMIN_TOKEN>
    let Some(ref expected_token) = state.newsletter_admin_token else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let client_ip = extract_client_ip(&headers);
    // IP-based brute-force protection: max 5 failed attempts per 10 minutes
    {
        let now = now_secs();
        let window = 600u64;
        let mut attempts = state.newsletter_auth_attempts.lock().unwrap();
        if let Some((count, start)) = attempts.get(&client_ip) {
            if now - start < window && *count >= 5 {
                return StatusCode::TOO_MANY_REQUESTS.into_response();
            }
        }
    }
    let provided = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let authorized = constant_time_eq(provided.as_bytes(), expected_token.as_bytes());
    if !authorized {
        // Record failed attempt
        let now = now_secs();
        let window = 600u64;
        let mut attempts = state.newsletter_auth_attempts.lock().unwrap();
        let entry = attempts.entry(client_ip).or_insert((0, now));
        if now - entry.1 >= window { *entry = (0, now); } // reset old window
        entry.0 += 1;
        return StatusCode::UNAUTHORIZED.into_response();
    }
    // Success — clear failed attempts
    state.newsletter_auth_attempts.lock().unwrap().remove(&client_ip);
    let data = std::fs::read_to_string("/data/newsletter.txt").unwrap_or_default();
    let emails: Vec<serde_json::Value> = data.lines().filter(|l| !l.is_empty()).map(|l| {
        let parts: Vec<&str> = l.splitn(2, '\t').collect();
        serde_json::json!({"ts": parts.first().unwrap_or(&""), "email": parts.get(1).unwrap_or(&"")})
    }).collect();
    (cors_headers(), Json(serde_json::json!({"count": emails.len(), "emails": emails}))).into_response()
}

// ── Stripe Webhook ──

/// Verify Stripe webhook signature (Stripe-Signature header, HMAC-SHA256).
/// Returns Ok(()) if valid, Err(status) otherwise.
fn verify_stripe_signature(secret: &str, payload: &str, sig_header: &str) -> Result<(), StatusCode> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Parse timestamp and v1 signatures from "t=...,v1=...,v1=..." format
    let mut ts_str: Option<&str> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        if let Some(v) = part.trim().strip_prefix("t=") {
            ts_str = Some(v);
        } else if let Some(v) = part.trim().strip_prefix("v1=") {
            signatures.push(v);
        }
    }
    let ts = ts_str.ok_or(StatusCode::BAD_REQUEST)?;

    // Signed payload = "<timestamp>.<body>"
    let signed_payload = format!("{}.{}", ts, payload);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    hmac::Mac::update(&mut mac, signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex_encode(&expected);

    // Constant-time comparison against any v1 signature
    let valid = signatures.iter().any(|sig| {
        sig.len() == expected_hex.len() && constant_time_eq(sig.as_bytes(), expected_hex.as_bytes())
    });
    if valid { Ok(()) } else { Err(StatusCode::UNAUTHORIZED) }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn stripe_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Verify Stripe-Signature header using HMAC-SHA256
    let Some(ref secret) = state.stripe_webhook_secret else {
        eprintln!("STRIPE_WEBHOOK_SECRET not configured");
        return (StatusCode::INTERNAL_SERVER_ERROR, "webhook secret not configured").into_response();
    };
    let sig_header = match headers.get("stripe-signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "missing stripe-signature header").into_response(),
    };
    if let Err(status) = verify_stripe_signature(secret, &body, &sig_header) {
        eprintln!("Stripe signature verification failed");
        return (status, "invalid signature").into_response();
    }

    let event: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid json").into_response(),
    };

    let event_type = event["type"].as_str().unwrap_or("");
    println!("Stripe webhook: {event_type}");

    match event_type {
        "checkout.session.completed" => {
            let session = &event["data"]["object"];
            let customer_email = session["customer_details"]["email"].as_str().unwrap_or("");
            let customer_name = session["customer_details"]["name"].as_str().unwrap_or("");
            let amount = session["amount_total"].as_i64().unwrap_or(0);
            let currency = session["currency"].as_str().unwrap_or("jpy").to_uppercase();

            println!("New checkout: {customer_email} ({customer_name}) {amount} {currency}");

            // チャリンアプリに収入データを送信
            if let Some(ref charin_key) = state.charin_api_key {
                let _ = notify_charin(
                    charin_key,
                    &format!("ファンクラブ ({customer_name})"),
                    amount,
                    &currency,
                    "subscription",
                    &format!("Enabler ファンクラブ第1期 — {customer_email}"),
                    customer_email,
                ).await;
            }

            // Log to file
            let _ = std::fs::create_dir_all("/data");
            let line = format!("{}\tcheckout\t{}\t{}\t{}\n",
                chrono::Utc::now().to_rfc3339(), customer_email, customer_name, amount);
            let _ = std::fs::OpenOptions::new().create(true).append(true)
                .open("/data/fanclub_events.txt")
                .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        }
        "customer.subscription.created" => {
            let sub = &event["data"]["object"];
            let customer = sub["customer"].as_str().unwrap_or("");
            let status = sub["status"].as_str().unwrap_or("");
            println!("Subscription created: {customer} status={status}");
        }
        "customer.subscription.deleted" => {
            let sub = &event["data"]["object"];
            let customer = sub["customer"].as_str().unwrap_or("");
            println!("Subscription cancelled: {customer}");
        }
        _ => {
            println!("Unhandled event: {event_type}");
        }
    }

    (StatusCode::OK, "ok").into_response()
}

async fn notify_charin(api_key: &str, source: &str, amount: i64, currency: &str, category: &str, memo: &str, email: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client.post("https://kacha-server.fly.dev/api/v1/charin/income")
        .json(&serde_json::json!({
            "api_key": api_key,
            "source": source,
            "amount": amount,
            "currency": currency,
            "category": category,
            "memo": memo,
            "email": email,
        }))
        .send().await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        println!("Charin notified: {source} {amount} {currency}");
        Ok(())
    } else {
        let err = res.text().await.unwrap_or_default();
        println!("Charin notify failed: {err}");
        Err(err)
    }
}

async fn send_notification_email(resend_key: &str, subject: &str, body: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client.post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {resend_key}"))
        .json(&serde_json::json!({
            "from": "Enabler <info@enablerdao.com>",
            "to": ["info@enablerdao.com"],
            "subject": format!("[Enabler] {subject}"),
            "text": body,
        }))
        .send().await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        println!("Email sent: {subject}");
        Ok(())
    } else {
        let err = res.text().await.unwrap_or_default();
        println!("Email failed: {err}");
        Err(err)
    }
}

// ── Newsletter with email notification ──

async fn newsletter_post_with_notify(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<EmailReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let line = format!("{}\t{}\n", chrono::Utc::now().to_rfc3339(), email);
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::OpenOptions::new().create(true).append(true).open("/data/newsletter.txt")
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));

    // Send notification
    if let Some(ref resend_key) = state.resend_key {
        let _ = send_notification_email(
            resend_key,
            "新規メルマガ登録",
            &format!("新しいメルマガ登録: {email}\n時刻: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")),
        ).await;
    }

    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

// ── Fanclub Member Portal ──

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn get_fanclub_session(headers: &HeaderMap, sessions: &Mutex<HashMap<String, (String, u64)>>) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(token) = part.strip_prefix("fanclub_auth=") {
            let store = sessions.lock().unwrap();
            if let Some((email, exp)) = store.get(token) {
                if now_secs() < *exp {
                    return Some(email.clone());
                }
            }
        }
    }
    None
}

async fn fanclub_login_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    // Already logged in → redirect to members
    if get_fanclub_session(&headers, &state.fanclub_sessions).is_some() {
        return Redirect::to("/fanclub/members").into_response();
    }
    let html = r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>濱田優貴のファンクラブ — AIで1人チームを作る方法、全部教えます</title>
<meta name="description" content="2年と数百万円を費やして習得したAIの使い方。7本のプロダクトが従業員ゼロで動いている現実。その全てをファンクラブメンバーに公開します。">
<style>
*{margin:0;padding:0;box-sizing:border-box;}
:root{--gold:#E8B64A;--gold2:#d4a43e;--bg:#080808;--bg2:#0f0f0f;--bg3:#161616;--border:#1e1e1e;--text:#e0e0e0;--muted:#666;--red:#e8410a;}
body{background:var(--bg);color:var(--text);font-family:-apple-system,BlinkMacSystemFont,'Hiragino Kaku Gothic ProN','Noto Sans JP',sans-serif;line-height:1.7;}
a{color:var(--gold);text-decoration:none;}

/* ── HERO ── */
.hero{min-height:100vh;display:flex;flex-direction:column;justify-content:center;align-items:center;
  text-align:center;padding:80px 24px 60px;position:relative;overflow:hidden;}
.hero::before{content:'';position:absolute;inset:0;
  background:radial-gradient(ellipse 80% 60% at 50% 40%,rgba(232,182,74,.06) 0%,transparent 70%);pointer-events:none;}
.hero-label{font-size:.65rem;letter-spacing:.2em;color:var(--gold);text-transform:uppercase;
  border:1px solid rgba(232,182,74,.3);padding:5px 14px;border-radius:20px;display:inline-block;margin-bottom:32px;}
.hero h1{font-size:clamp(2.2rem,6vw,4rem);font-weight:700;line-height:1.15;letter-spacing:-.02em;
  color:#fff;margin-bottom:20px;max-width:800px;}
.hero h1 em{color:var(--gold);font-style:normal;}
.hero-sub{font-size:clamp(.9rem,2vw,1.1rem);color:#999;max-width:560px;margin-bottom:48px;line-height:1.8;}
.hero-stats{display:flex;gap:40px;justify-content:center;flex-wrap:wrap;margin-bottom:56px;}
.stat{text-align:center;}
.stat-num{font-size:clamp(1.8rem,4vw,2.8rem);font-weight:700;color:#fff;letter-spacing:-.03em;display:block;}
.stat-num span{color:var(--gold);}
.stat-label{font-size:.72rem;color:var(--muted);letter-spacing:.05em;}
.hero-cta{display:inline-block;background:linear-gradient(135deg,var(--gold),var(--gold2));
  color:#080808;font-weight:700;font-size:1rem;padding:16px 36px;border-radius:12px;
  transition:transform .2s,box-shadow .2s;cursor:pointer;border:none;}
.hero-cta:hover{transform:translateY(-2px);box-shadow:0 12px 40px rgba(232,182,74,.25);}

/* ── SECTION ── */
section{padding:80px 24px;max-width:760px;margin:0 auto;}
.section-label{font-size:.65rem;letter-spacing:.2em;color:var(--gold);text-transform:uppercase;margin-bottom:16px;}
h2{font-size:clamp(1.5rem,3.5vw,2.2rem);font-weight:700;color:#fff;margin-bottom:20px;line-height:1.3;}
p{color:#aaa;margin-bottom:16px;font-size:.95rem;}
p strong{color:#ddd;}

/* ── PROOF GRID ── */
.proof-section{background:var(--bg2);padding:80px 24px;}
.proof-inner{max-width:760px;margin:0 auto;}
.proof-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:1px;
  background:var(--border);border:1px solid var(--border);border-radius:12px;overflow:hidden;margin-top:32px;}
.proof-item{background:var(--bg2);padding:24px 20px;}
.proof-icon{font-size:1.5rem;margin-bottom:8px;}
.proof-name{font-size:.85rem;font-weight:600;color:#ddd;margin-bottom:4px;}
.proof-desc{font-size:.72rem;color:var(--muted);}

/* ── NUMBERS ── */
.numbers{background:var(--bg3);padding:80px 24px;}
.numbers-inner{max-width:760px;margin:0 auto;}
.compare{display:grid;grid-template-columns:1fr 1fr;gap:1px;background:var(--border);
  border:1px solid var(--border);border-radius:16px;overflow:hidden;margin-top:32px;}
.compare-item{padding:32px 28px;background:var(--bg3);}
.compare-item.accent{background:#100c04;}
.compare-label{font-size:.65rem;letter-spacing:.15em;color:var(--muted);text-transform:uppercase;margin-bottom:12px;}
.compare-price{font-size:clamp(1.4rem,3vw,2rem);font-weight:700;color:#fff;margin-bottom:6px;}
.compare-price .unit{font-size:.8rem;font-weight:400;color:var(--muted);}
.compare-price.gold{color:var(--gold);}
.compare-note{font-size:.75rem;color:var(--muted);line-height:1.6;}
@media(max-width:500px){.compare{grid-template-columns:1fr;}}

/* ── SECRET SECTION ── */
.secret{border:1px solid rgba(232,182,74,.2);border-radius:20px;padding:48px 40px;
  background:linear-gradient(135deg,rgba(232,182,74,.04) 0%,transparent 60%);margin:80px auto;max-width:760px;}
.secret-label{font-size:.65rem;letter-spacing:.2em;color:var(--gold);text-transform:uppercase;margin-bottom:20px;}
.secret h2{font-size:clamp(1.4rem,3vw,2rem);color:#fff;margin-bottom:20px;}
.secret p{color:#999;font-size:.92rem;}
.secret-items{margin-top:28px;display:flex;flex-direction:column;gap:12px;}
.secret-item{display:flex;align-items:flex-start;gap:12px;padding:14px 16px;
  background:rgba(255,255,255,.03);border-radius:10px;border:1px solid rgba(255,255,255,.05);}
.secret-item-icon{font-size:1.1rem;flex-shrink:0;margin-top:2px;}
.secret-item-text{font-size:.85rem;color:#bbb;line-height:1.5;}
.secret-item-text strong{color:#fff;}
.secret-lock{text-align:center;margin-top:32px;padding-top:24px;border-top:1px solid var(--border);}
.secret-lock p{font-size:.85rem;color:var(--muted);}
.secret-lock strong{color:var(--gold);}

/* ── PRODUCTS CTA ── */
.products-cta{background:var(--bg2);padding:60px 24px;}
.products-cta-inner{max-width:760px;margin:0 auto;text-align:center;}
.products-cta h2{font-size:1.4rem;margin-bottom:12px;}
.products-cta p{font-size:.88rem;color:var(--muted);margin-bottom:28px;}
.product-links{display:flex;flex-wrap:wrap;gap:10px;justify-content:center;}
.product-link{padding:10px 20px;border:1px solid var(--border);border-radius:8px;
  font-size:.8rem;color:#bbb;transition:all .2s;}
.product-link:hover{border-color:var(--gold);color:var(--gold);}

/* ── LOGIN CARD ── */
.login-section{padding:80px 24px 100px;max-width:480px;margin:0 auto;}
.login-label{text-align:center;font-size:.65rem;letter-spacing:.2em;color:var(--gold);
  text-transform:uppercase;margin-bottom:16px;}
.login-title{text-align:center;font-size:1.3rem;font-weight:700;color:#fff;margin-bottom:8px;}
.login-sub{text-align:center;font-size:.8rem;color:var(--muted);margin-bottom:32px;line-height:1.6;}
.card{background:#111;border:1px solid #222;border-radius:20px;padding:36px 32px;}
.step{display:none;}
.step.active{display:block;}
label{display:block;text-align:left;font-size:.75rem;color:#888;margin-bottom:6px;}
input{width:100%;padding:14px 16px;background:#0a0a0a;border:1px solid #2a2a2a;border-radius:10px;
  color:#e0e0e0;font-size:1rem;margin-bottom:16px;outline:none;transition:border .2s;}
input:focus{border-color:var(--gold);}
.btn{width:100%;padding:14px;background:linear-gradient(135deg,var(--gold),var(--gold2));color:#080808;
  border:none;border-radius:10px;font-weight:700;font-size:.95rem;cursor:pointer;transition:opacity .2s;}
.btn:hover{opacity:.9;}
.btn:disabled{opacity:.5;cursor:not-allowed;}
.msg{font-size:.8rem;margin-top:12px;min-height:20px;color:var(--gold);}
.msg.err{color:#ef4444;}
.back{background:none;border:none;color:#888;font-size:.75rem;cursor:pointer;margin-top:12px;text-decoration:underline;}
.hint{font-size:.72rem;color:#666;margin-top:12px;line-height:1.5;}
</style>
<script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body>

<!-- HERO -->
<div class="hero">
  <div class="hero-label">Yuki Hamada — Fanclub</div>
  <h1>2年と<em>数百万円</em>を費やして<br>習得したAIの使い方、<br>全部教えます。</h1>
  <p class="hero-sub">チームより強い、1人がいる。<br>7本のプロダクトが従業員ゼロで動いている現実。<br>その方法は、プロダクトを使ってくれる人にだけ教えます。</p>
  <div class="hero-stats">
    <div class="stat">
      <span class="stat-num">7<span>本</span></span>
      <span class="stat-label">稼働中プロダクト</span>
    </div>
    <div class="stat">
      <span class="stat-num">0<span>人</span></span>
      <span class="stat-label">従業員</span>
    </div>
    <div class="stat">
      <span class="stat-num">2<span>年</span></span>
      <span class="stat-label">試行錯誤の時間</span>
    </div>
    <div class="stat">
      <span class="stat-num">¥<span>数百万</span></span>
      <span class="stat-label">費やしたAIコスト</span>
    </div>
  </div>
  <button class="hero-cta" onclick="document.getElementById('login-section').scrollIntoView({behavior:'smooth'})">
    入会してメソッドを受け取る
  </button>
</div>

<!-- PROOF -->
<div class="proof-section">
  <div class="proof-inner">
    <div class="section-label">Proof — 証拠</div>
    <h2>今日も動いている、7本のプロダクト。</h2>
    <p>信じてもらえないかもしれないけど、全部1人で作った。エンジニアもデザイナーも雇っていない。</p>
    <div class="proof-grid">
      <div class="proof-item">
        <div class="proof-icon">💬</div>
        <div class="proof-name">Chatweb.ai</div>
        <div class="proof-desc">AIチャットプラットフォーム</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">🥋</div>
        <div class="proof-name">JiuFlow</div>
        <div class="proof-desc">柔術管理アプリ</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">📄</div>
        <div class="proof-name">パシャ</div>
        <div class="proof-desc">レシート経費管理 iOS</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">🏠</div>
        <div class="proof-name">StayFlow</div>
        <div class="proof-desc">民泊管理ツール</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">🎙️</div>
        <div class="proof-name">Koe Device</div>
        <div class="proof-desc">ハードウェアAI入力デバイス</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">📊</div>
        <div class="proof-name">ミセバンAI</div>
        <div class="proof-desc">店舗AI分析ツール</div>
      </div>
      <div class="proof-item">
        <div class="proof-icon">✍️</div>
        <div class="proof-name">BANTO</div>
        <div class="proof-desc">電子契約・署名サービス</div>
      </div>
    </div>
  </div>
</div>

<!-- NUMBERS -->
<div class="numbers">
  <div class="numbers-inner">
    <div class="section-label">Numbers — 数字</div>
    <h2>人を雇うより安くて、速い。</h2>
    <p>これが今の現実のコスト比較。同じアウトプットを出すのに、チームを組むと月340万以上かかっていた。</p>
    <div class="compare">
      <div class="compare-item">
        <div class="compare-label">チームを組む場合</div>
        <div class="compare-price">¥340万<span class="unit">〜 / 月</span></div>
        <div class="compare-note">エンジニア2名 + デザイナー1名 + PM1名の最低構成。採用コスト・管理コスト別途。</div>
      </div>
      <div class="compare-item accent">
        <div class="compare-label">今の俺のコスト</div>
        <div class="compare-price gold">¥20〜100万<span class="unit"> / 月</span></div>
        <div class="compare-note">通常運用は月20万以下。新プロダクト開発・動画制作などフル稼働時が100万前後。自分でコントロールできる。</div>
      </div>
    </div>
  </div>
</div>

<!-- SECRET -->
<div style="padding:0 24px;">
<div class="secret">
  <div class="secret-label">Inside — 中身</div>
  <h2>正直に言うと、教えたくないんですよ。</h2>
  <p>2年間、毎日のようにAIと格闘してきた。失敗も数え切れないほどある。3本作って3本とも誰にも使われなかった時期もある。そのうえで今の形にたどり着いた。</p>
  <p style="margin-top:12px;">それをあっさり公開するのは、正直もったいない。でも、俺のプロダクトを使ってくれる人には全部話したい。使ってくれる人だけに。</p>
  <div class="secret-items">
    <div class="secret-item">
      <div class="secret-item-icon">🔒</div>
      <div class="secret-item-text"><strong>具体的なAIツールの使い方</strong><br>どのツールをどの順番でどう使うか。再現可能な形で全部書く。</div>
    </div>
    <div class="secret-item">
      <div class="secret-item-icon">🔒</div>
      <div class="secret-item-text"><strong>プロダクト開発の実際のフロー</strong><br>アイデアから公開まで、何時間でどう動くか。リアルな話をする。</div>
    </div>
    <div class="secret-item">
      <div class="secret-item-icon">🔒</div>
      <div class="secret-item-text"><strong>コストコントロールの具体的な方法</strong><br>月100万を月20万に戻すとき、何をどう削るか。</div>
    </div>
    <div class="secret-item">
      <div class="secret-item-icon">🔒</div>
      <div class="secret-item-text"><strong>失敗から学んだこと全部</strong><br>うまくいかなかったパターンをそのまま話す。同じ失敗を繰り返さないために。</div>
    </div>
  </div>
  <div class="secret-lock">
    <p>これらのコンテンツは<strong>ファンクラブメンバー限定</strong>で公開します。<br>入会条件は、僕のプロダクトを使い続けてくれること。それだけです。</p>
  </div>
</div>
</div>

<!-- PRODUCTS CTA -->
<div class="products-cta">
  <div class="products-cta-inner">
    <h2>まずはプロダクトを使ってみてください。</h2>
    <p>使ってみて、良ければファンクラブへ。<br>メソッドはそこで全部話します。</p>
    <div class="product-links">
      <a href="https://chatweb.ai" class="product-link" target="_blank">Chatweb.ai →</a>
      <a href="https://jiuflow.art" class="product-link" target="_blank">JiuFlow →</a>
      <a href="https://pasha.run" class="product-link" target="_blank">パシャ →</a>
      <a href="https://stayflowapp.com" class="product-link" target="_blank">StayFlow →</a>
      <a href="https://koe.live" class="product-link" target="_blank">Koe Device →</a>
      <a href="https://misebanai.com" class="product-link" target="_blank">ミセバンAI →</a>
      <a href="https://banto.work" class="product-link" target="_blank">BANTO →</a>
    </div>
  </div>
</div>

<!-- LOGIN -->
<div class="login-section" id="login-section">
  <div class="login-label">Members Only</div>
  <div class="login-title">ファンクラブにログイン</div>
  <div class="login-sub">メールアドレスを入力してください。<br>認証コードをお送りします。</div>
  <div class="card">

  <!-- Step 1: Email -->
  <div class="step active" id="step1">
    <label>メールアドレス</label>
    <input type="email" id="email" placeholder="you@example.com" autocomplete="email">
    <button class="btn" id="sendBtn" onclick="sendOtp()">認証コードを送る</button>
    <div class="msg" id="msg1"></div>
    <p class="hint">未登録の方もそのまま入力してください。入会案内に進みます。</p>
  </div>

  <!-- Step 2: OTP -->
  <div class="step" id="step2">
    <label>認証コード（6桁）</label>
    <input type="text" id="otp" placeholder="123456" inputmode="numeric" maxlength="6" autocomplete="one-time-code">
    <button class="btn" id="verifyBtn" onclick="verifyOtp()">ログイン</button>
    <div class="msg" id="msg2"></div>
    <button class="back" onclick="goBack()">← メールアドレスを変更</button>
  </div>

  </div>
</div>

<script>
let currentEmail = '';

async function sendOtp() {
  const email = document.getElementById('email').value.trim();
  if (!email || !email.includes('@')) {
    setMsg('msg1', '有効なメールアドレスを入力してください', true);
    return;
  }
  const btn = document.getElementById('sendBtn');
  btn.disabled = true; btn.textContent = '送信中...';
  setMsg('msg1', '');
  try {
    const r = await fetch('/api/fanclub/otp/send', {
      method: 'POST', headers: {'Content-Type':'application/json'},
      body: JSON.stringify({email})
    });
    const d = await r.json();
    if (d.ok) {
      currentEmail = email;
      document.getElementById('step1').classList.remove('active');
      document.getElementById('step2').classList.add('active');
      document.getElementById('otp').focus();
    } else {
      setMsg('msg1', d.error || 'ご登録のメールアドレスが見つかりませんでした。', true);
      btn.disabled = false; btn.textContent = '認証コードを送る';
    }
  } catch(e) {
    setMsg('msg1', '通信エラーが発生しました。', true);
    btn.disabled = false; btn.textContent = '認証コードを送る';
  }
}

async function verifyOtp() {
  const code = document.getElementById('otp').value.trim();
  if (!code || code.length !== 6) {
    setMsg('msg2', '6桁のコードを入力してください', true);
    return;
  }
  const btn = document.getElementById('verifyBtn');
  btn.disabled = true; btn.textContent = '確認中...';
  setMsg('msg2', '');
  try {
    const r = await fetch('/api/fanclub/otp/verify', {
      method: 'POST', headers: {'Content-Type':'application/json'},
      body: JSON.stringify({email: currentEmail, code})
    });
    const d = await r.json();
    if (d.ok) {
      setMsg('msg2', 'ログイン成功！ リダイレクト中...');
      setTimeout(() => location.href = '/fanclub/members', 500);
    } else if (d.needs_subscription && d.checkout_url) {
      setMsg('msg2', '入会ページに移動します...');
      setTimeout(() => location.href = d.checkout_url, 800);
    } else {
      setMsg('msg2', d.error || 'コードが正しくありません。', true);
      btn.disabled = false; btn.textContent = 'ログイン';
    }
  } catch(e) {
    setMsg('msg2', '通信エラーが発生しました。', true);
    btn.disabled = false; btn.textContent = 'ログイン';
  }
}

function goBack() {
  document.getElementById('step2').classList.remove('active');
  document.getElementById('step1').classList.add('active');
  document.getElementById('sendBtn').disabled = false;
  document.getElementById('sendBtn').textContent = '認証コードを送る';
  setMsg('msg1', '');
}

function setMsg(id, text, isErr) {
  const el = document.getElementById(id);
  el.textContent = text;
  el.className = 'msg' + (isErr ? ' err' : '');
}

document.getElementById('email').addEventListener('keydown', e => {
  if (e.key === 'Enter') sendOtp();
});
document.getElementById('otp').addEventListener('keydown', e => {
  if (e.key === 'Enter') verifyOtp();
});
</script>
</body>
</html>"#;
    ([("content-type", "text/html; charset=utf-8")], html).into_response()
}

#[derive(serde::Deserialize)]
struct OtpVerifyReq { email: String, code: String }

/// Extract client IP from headers (Fly.io / reverse-proxy aware).
fn extract_client_ip(headers: &HeaderMap) -> String {
    // Fly.io sets Fly-Client-IP; fall back to X-Forwarded-For, then X-Real-IP
    if let Some(v) = headers.get("fly-client-ip").and_then(|v| v.to_str().ok()) {
        return v.split(',').next().unwrap_or("unknown").trim().to_string();
    }
    if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        return v.split(',').next().unwrap_or("unknown").trim().to_string();
    }
    if let Some(v) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return v.trim().to_string();
    }
    "unknown".to_string()
}

async fn fanclub_send_otp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<EmailReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();

    if !email.contains('@') || email.len() < 5 {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "有効なメールアドレスを入力してください"}))).into_response();
    }

    // IP-based rate limit: max 3 requests per 5 minutes
    let client_ip = extract_client_ip(&headers);
    {
        let window = 300u64; // 5 minutes in seconds
        let max_requests = 3usize;
        let now = now_secs();
        let mut rl = state.otp_rate_limit.lock().unwrap();
        let timestamps = rl.entry(client_ip.clone()).or_default();
        // Remove timestamps older than the window
        timestamps.retain(|&ts| now - ts < window);
        if timestamps.len() >= max_requests {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({"ok": false, "error": "しばらくしてからもう一度お試しください"})),
            ).into_response();
        }
        timestamps.push(now);
    }

    // Generate 6-digit OTP
    use rand::Rng;
    let code = format!("{:06}", rand::thread_rng().gen_range(100000u32..=999999));
    let exp = now_secs() + 600; // 10 minutes
    {
        let mut store = state.otp_store.lock().unwrap();
        store.insert(email.clone(), (code.clone(), exp, 0u8));
    }

    // Send via Resend
    if let Some(ref resend_key) = state.resend_key {
        let client = reqwest::Client::new();
        let send_res = client.post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {resend_key}"))
            .json(&serde_json::json!({
                "from": "Enabler ファンクラブ <info@enablerdao.com>",
                "to": [&email],
                "subject": format!("【ファンクラブ】認証コード: {code}"),
                "html": format!(r#"<div style="font-family:sans-serif;max-width:480px;margin:0 auto;padding:32px;background:#080808;color:#e0e0e0;border-radius:16px;">
<h2 style="color:#E8B64A;margin-bottom:8px;">ファンクラブ ログイン認証</h2>
<p style="color:#aaa;margin-bottom:24px;">以下の認証コードを入力してください。</p>
<div style="background:#111;border:1px solid #333;border-radius:12px;padding:24px;text-align:center;margin-bottom:24px;">
  <span style="font-size:2.5rem;font-weight:700;letter-spacing:.2em;color:#E8B64A;">{code}</span>
</div>
<p style="color:#666;font-size:.75rem;">このコードは10分間有効です。身に覚えのない場合は無視してください。</p>
</div>"#),
            }))
            .send().await;
        if let Ok(r) = send_res {
            if !r.status().is_success() {
                let err: String = r.text().await.unwrap_or_default();
                println!("Resend error: {err}");
            }
        }
    } else {
        // Dev mode: print code to logs
        println!("FANCLUB OTP for {email}: {code}");
    }

    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

async fn fanclub_verify_otp(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OtpVerifyReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let code = body.code.trim().to_string();

    const MAX_OTP_ATTEMPTS: u8 = 5;

    let valid = {
        let mut store = state.otp_store.lock().unwrap();
        if let Some((stored_code, exp, attempts)) = store.get_mut(&email) {
            if *attempts >= MAX_OTP_ATTEMPTS {
                return (cors_headers(), Json(serde_json::json!({
                    "ok": false,
                    "error": "試行回数が多すぎます。新しいコードを送信してください。"
                }))).into_response();
            }
            if now_secs() >= *exp {
                false
            } else if *stored_code == code {
                true
            } else {
                *attempts += 1;
                false
            }
        } else {
            false
        }
    };

    if !valid {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "コードが正しくないか期限切れです。"}))).into_response();
    }

    // Remove used OTP
    state.otp_store.lock().unwrap().remove(&email);

    // Check Stripe subscription
    let has_subscription = if let Some(ref sk) = state.stripe_key {
        let client = reqwest::Client::new();
        let cust_res = client.get("https://api.stripe.com/v1/customers")
            .basic_auth(sk, None::<&str>)
            .query(&[("email", &email), ("limit", &"1".to_string())])
            .send().await;
        if let Ok(r) = cust_res {
            let cust_json: serde_json::Value = r.json().await.unwrap_or_default();
            let customers = cust_json["data"].as_array().cloned().unwrap_or_default();
            if let Some(cust) = customers.first() {
                let cid = cust["id"].as_str().unwrap_or_default().to_string();
                let sub_res = client.get("https://api.stripe.com/v1/subscriptions")
                    .basic_auth(sk, None::<&str>)
                    .query(&[("customer", &cid), ("status", &"active".to_string()), ("limit", &"1".to_string())])
                    .send().await;
                if let Ok(sr) = sub_res {
                    let sub_json: serde_json::Value = sr.json().await.unwrap_or_default();
                    sub_json["data"].as_array().map_or(false, |a| !a.is_empty())
                } else { false }
            } else { false }
        } else { false }
    } else { true }; // dev mode: allow all

    if !has_subscription {
        // Create Stripe checkout session so the user can subscribe
        let checkout_url = if let Some(ref sk) = state.stripe_key {
            let client = reqwest::Client::new();
            let res = client.post("https://api.stripe.com/v1/checkout/sessions")
                .basic_auth(sk, None::<&str>)
                .form(&[
                    ("mode", "subscription"),
                    ("line_items[0][price]", "price_1TF6xJDqLakc8NxkMpr9KyGB"),
                    ("line_items[0][quantity]", "1"),
                    ("customer_email", &email),
                    ("success_url", "https://yukihamada.jp/fanclub"),
                    ("cancel_url", "https://yukihamada.jp/fanclub"),
                ])
                .send().await;
            if let Ok(r) = res {
                let json: serde_json::Value = r.json().await.unwrap_or_default();
                json["url"].as_str().unwrap_or("https://yukihamada.jp/fanclub").to_string()
            } else {
                "https://yukihamada.jp/fanclub".to_string()
            }
        } else {
            "https://yukihamada.jp/fanclub".to_string()
        };
        return (cors_headers(), Json(serde_json::json!({
            "ok": false,
            "needs_subscription": true,
            "checkout_url": checkout_url
        }))).into_response();
    }

    // Issue session token
    use rand::Rng;
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    let exp = now_secs() + 7 * 24 * 3600; // 7 days
    {
        let mut fc = state.fanclub_sessions.lock().unwrap();
        fc.insert(token.clone(), (email.clone(), exp));
        let dash = state.dash_sessions.lock().unwrap();
        persist_sessions(&fc, &dash);
    }

    let cookie = format!("fanclub_auth={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800");
    let mut headers = cors_headers();
    headers.insert("set-cookie", cookie.parse().unwrap());
    (headers, Json(serde_json::json!({"ok": true}))).into_response()
}

// ── Admin Login (OTP, no Stripe) ──

async fn admin_send_otp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<EmailReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    if email != "mail@yukihamada.jp" && email != "yuki@hamada.tokyo" {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "このメールアドレスは登録されていません"}))).into_response();
    }
    let client_ip = extract_client_ip(&headers);
    {
        let window = 300u64;
        let max_requests = 5usize;
        let now = now_secs();
        let mut rl = state.otp_rate_limit.lock().unwrap();
        let ts = rl.entry(client_ip).or_default();
        ts.retain(|&t| now - t < window);
        if ts.len() >= max_requests {
            return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({"ok": false, "error": "しばらくしてから再試行してください"}))).into_response();
        }
        ts.push(now);
    }
    use rand::Rng;
    let code = format!("{:06}", rand::thread_rng().gen_range(100000u32..=999999));
    let exp = now_secs() + 600;
    state.otp_store.lock().unwrap().insert(email.clone(), (code.clone(), exp, 0u8));

    if let Some(ref key) = state.resend_key {
        let client = reqwest::Client::new();
        let _ = client.post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "from": "yukihamada.jp <info@enablerdao.com>",
                "to": [&email],
                "subject": format!("【Admin Terminal】認証コード: {code}"),
                "html": format!(r#"<div style="font-family:sans-serif;padding:32px;background:#080808;color:#e0e0e0;border-radius:16px;max-width:480px;margin:0 auto;"><h2 style="color:#e8a04c;margin-bottom:8px;">Admin Terminal ログイン</h2><p style="color:#aaa;margin-bottom:24px;">yukihamada.jp の管理者ターミナルへのログインコードです。</p><div style="background:#111;border:1px solid #333;border-radius:12px;padding:24px;text-align:center;margin-bottom:24px;"><span style="font-size:2.5rem;font-weight:700;letter-spacing:.2em;color:#e8a04c;">{code}</span></div><p style="color:#666;font-size:.75rem;">このコードは10分間有効です。</p></div>"#),
            }))
            .send().await;
    } else {
        println!("ADMIN OTP for {email}: {code}");
    }
    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

async fn admin_verify_otp(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OtpVerifyReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let code = body.code.trim().to_string();
    if email != "mail@yukihamada.jp" && email != "yuki@hamada.tokyo" {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "不正なアクセス"}))).into_response();
    }
    let valid = {
        let mut store = state.otp_store.lock().unwrap();
        if let Some((stored_code, exp, attempts)) = store.get_mut(&email) {
            if *attempts >= 5 {
                return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "試行回数超過。新しいコードを送信してください。"}))).into_response();
            }
            if now_secs() >= *exp { false }
            else if *stored_code == code { true }
            else { *attempts += 1; false }
        } else { false }
    };
    if !valid {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "コードが正しくないか期限切れです。"}))).into_response();
    }
    state.otp_store.lock().unwrap().remove(&email);
    use rand::Rng;
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let exp = now_secs() + 30 * 24 * 3600; // 30 days
    state.admin_sessions.lock().unwrap().insert(token.clone(), (email.clone(), exp));
    (cors_headers(), Json(serde_json::json!({"ok": true, "token": token, "email": email}))).into_response()
}

async fn admin_me(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let token = params.get("token").cloned()
        .or_else(|| headers.get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string)))
        .unwrap_or_default();
    let sessions = state.admin_sessions.lock().unwrap();
    if let Some((email, exp)) = sessions.get(&token) {
        if *exp > now_secs() {
            return (cors_headers(), Json(serde_json::json!({"ok": true, "email": email, "is_admin": true}))).into_response();
        }
    }
    (cors_headers(), Json(serde_json::json!({"ok": false}))).into_response()
}

// ── General User Login (any email) ──

#[derive(serde::Deserialize)]
struct UserOtpReq { email: String }

#[derive(serde::Deserialize)]
struct UserVerifyReq { email: String, code: String }

async fn user_send_otp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<UserOtpReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "メールアドレスが正しくありません"}))).into_response();
    }
    let client_ip = extract_client_ip(&headers);
    {
        let window = 300u64;
        let max_requests = 5usize;
        let now = now_secs();
        let mut rl = state.otp_rate_limit.lock().unwrap();
        let ts = rl.entry(format!("user:{client_ip}")).or_default();
        ts.retain(|&t| now - t < window);
        if ts.len() >= max_requests {
            return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({"ok": false, "error": "しばらくしてから再試行してください"}))).into_response();
        }
        ts.push(now);
    }
    use rand::Rng;
    let code = format!("{:06}", rand::thread_rng().gen_range(100000u32..=999999));
    let exp = now_secs() + 600;
    state.otp_store.lock().unwrap().insert(format!("user:{}", email), (code.clone(), exp, 0u8));

    if let Some(ref key) = state.resend_key {
        let client = reqwest::Client::new();
        let _ = client.post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "from": "yukihamada.jp <info@enablerdao.com>",
                "to": [&email],
                "subject": format!("【yukihamada.jp】認証コード: {code}"),
                "html": format!(r#"<div style="font-family:sans-serif;padding:32px;background:#080808;color:#e0e0e0;border-radius:16px;max-width:480px;margin:0 auto;"><h2 style="color:#e8a04c;margin-bottom:8px;">yukihamada.jp ログイン</h2><p style="color:#aaa;margin-bottom:24px;">以下のコードを入力してログインしてください。</p><div style="background:#111;border:1px solid #333;border-radius:12px;padding:24px;text-align:center;margin-bottom:24px;"><span style="font-size:2.5rem;font-weight:700;letter-spacing:.2em;color:#e8a04c;">{code}</span></div><p style="color:#666;font-size:.75rem;">このコードは10分間有効です。心当たりがない場合は無視してください。</p></div>"#),
            }))
            .send().await;
    } else {
        println!("USER OTP for {email}: {code}");
    }
    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

async fn user_verify_otp(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<UserVerifyReq>,
) -> Response {
    let email = body.email.to_lowercase().trim().to_string();
    let code = body.code.trim().to_string();
    if email.is_empty() || !email.contains('@') {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "不正なリクエスト"}))).into_response();
    }
    let otp_key = format!("user:{}", email);
    let valid = {
        let mut store = state.otp_store.lock().unwrap();
        if let Some((stored_code, exp, attempts)) = store.get_mut(&otp_key) {
            if *attempts >= 5 {
                return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "試行回数超過。新しいコードを送信してください。"}))).into_response();
            }
            if now_secs() >= *exp { false }
            else if *stored_code == code { true }
            else { *attempts += 1; false }
        } else { false }
    };
    if !valid {
        return (cors_headers(), Json(serde_json::json!({"ok": false, "error": "コードが正しくないか期限切れです。"}))).into_response();
    }
    state.otp_store.lock().unwrap().remove(&otp_key);
    use rand::Rng;
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let exp = now_secs() + 30 * 24 * 3600; // 30 days
    let is_admin = email == "mail@yukihamada.jp" || email == "yuki@hamada.tokyo";
    if is_admin {
        state.admin_sessions.lock().unwrap().insert(token.clone(), (email.clone(), exp));
    }
    state.user_sessions.lock().unwrap().insert(token.clone(), (email.clone(), exp));
    (cors_headers(), Json(serde_json::json!({"ok": true, "token": token, "email": email, "is_admin": is_admin}))).into_response()
}

async fn user_me(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let token = params.get("token").cloned()
        .or_else(|| headers.get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string)))
        .unwrap_or_default();
    let now = now_secs();
    {
        let sessions = state.user_sessions.lock().unwrap();
        if let Some((email, exp)) = sessions.get(&token) {
            if *exp > now {
                let is_admin = email == "mail@yukihamada.jp" || email == "yuki@hamada.tokyo";
                return (cors_headers(), Json(serde_json::json!({"ok": true, "email": email, "is_admin": is_admin}))).into_response();
            }
        }
    }
    (cors_headers(), Json(serde_json::json!({"ok": false}))).into_response()
}

// ── WebSocket Admin Terminal ──

fn validate_admin_token(state: &AppState, token: &str) -> bool {
    let sessions = state.admin_sessions.lock().unwrap();
    sessions.get(token)
        .map(|(email, exp)| (email == "mail@yukihamada.jp" || email == "yuki@hamada.tokyo") && *exp > now_secs())
        .unwrap_or(false)
}

async fn ws_terminal(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let pty = state.shared_pty.clone();
    let ak = state.anthropic_key.clone();
    pty.ensure_started(ak);
    ws.on_upgrade(move |socket| handle_ws_terminal(socket, pty))
}

async fn handle_ws_terminal(mut socket: WebSocket, pty: Arc<SharedPty>) {
    let mut rx = pty.tx.subscribe();
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(data) => {
                        if socket.send(Message::Binary(data.into())).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        // Check for JSON control messages (resize)
                        if data.first() == Some(&b'{') {
                            if let Ok(s) = std::str::from_utf8(&data) {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
                                    if v["t"] == "sz" {
                                        let c = v["c"].as_u64().unwrap_or(80) as u16;
                                        let r = v["r"].as_u64().unwrap_or(24) as u16;
                                        pty.resize(c, r);
                                    }
                                    continue;
                                }
                            }
                        }
                        pty.write_input(&data);
                    }
                    Some(Ok(Message::Text(s))) => {
                        // JSON control or raw text
                        if s.starts_with('{') {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                                if v["t"] == "sz" {
                                    let c = v["c"].as_u64().unwrap_or(80) as u16;
                                    let r = v["r"].as_u64().unwrap_or(24) as u16;
                                    pty.resize(c, r);
                                    continue;
                                }
                            }
                        }
                        pty.write_input(s.as_bytes());
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

// ── yukiterm CLI download ──
async fn yukiterm_script() -> impl IntoResponse {
    let script = r#"#!/usr/bin/env python3
"""yukiterm — CLI access to yukihamada.jp terminal
Usage: YUKITERM_TOKEN=your_token python3 yukiterm.py

Requirements: pip3 install websockets
Get token:    Login at https://yukihamada.jp/ → open Terminal app
"""
import asyncio, os, sys, tty, termios, signal, struct, fcntl

TOKEN = os.environ.get('YUKITERM_TOKEN', '')
HOST  = 'yukihamada.jp'

if not TOKEN:
    print('Set YUKITERM_TOKEN=<your token>')
    print('Login at https://yukihamada.jp/ to get your token')
    sys.exit(1)

try:
    import websockets
except ImportError:
    print('pip3 install websockets')
    sys.exit(1)

def get_winsize():
    buf = fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b'\x00'*8)
    rows, cols, _, _ = struct.unpack('HHHH', buf)
    return cols, rows

async def run():
    url = f'wss://{HOST}/ws/terminal?token={TOKEN}'
    print(f'Connecting to {HOST}…', flush=True)
    try:
        async with websockets.connect(url) as ws:
            print('Connected. Press Ctrl+\\ to exit.\n', flush=True)
            old_attrs = termios.tcgetattr(sys.stdin.fileno())
            tty.setraw(sys.stdin.fileno())

            async def send_resize():
                c, r = get_winsize()
                import json
                await ws.send(json.dumps({'t':'sz','c':c,'r':r}))

            loop = asyncio.get_event_loop()
            loop.add_signal_handler(signal.SIGWINCH,
                lambda: asyncio.create_task(send_resize()))

            try:
                await send_resize()

                async def recv_loop():
                    async for msg in ws:
                        data = msg if isinstance(msg, bytes) else msg.encode()
                        sys.stdout.buffer.write(data)
                        sys.stdout.buffer.flush()

                async def send_loop():
                    while True:
                        ch = await loop.run_in_executor(None, sys.stdin.buffer.read, 1)
                        if not ch:
                            break
                        if ch == b'\x1c':  # Ctrl+\
                            break
                        await ws.send(ch)

                done, pending = await asyncio.wait(
                    [asyncio.create_task(recv_loop()),
                     asyncio.create_task(send_loop())],
                    return_when=asyncio.FIRST_COMPLETED)
                for t in pending:
                    t.cancel()
            finally:
                termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, old_attrs)
                print('\r\nDisconnected.', flush=True)
    except Exception as e:
        print(f'\r\nError: {e}', flush=True)

asyncio.run(run())
"#;
    ([
        ("content-type", "text/x-python; charset=utf-8"),
        ("content-disposition", "attachment; filename=\"yukiterm.py\""),
        ("cache-control", "no-store"),
    ], script)
}

async fn fanclub_members(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let email = match get_fanclub_session(&headers, &state.fanclub_sessions) {
        Some(e) => e,
        None => return Redirect::to("/fanclub").into_response(),
    };

    let html = format!(r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>ファンクラブ — メンバーページ</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{background:#080808;color:#e0e0e0;font-family:-apple-system,BlinkMacSystemFont,'Hiragino Kaku Gothic ProN',sans-serif;}}
.header{{background:#111;border-bottom:1px solid #1a1a1a;padding:16px 24px;display:flex;align-items:center;justify-content:space-between;}}
.header h1{{font-size:1rem;color:#E8B64A;}}
.header .info{{font-size:.75rem;color:#888;}}
.header a{{color:#888;font-size:.75rem;text-decoration:none;}}
.header a:hover{{color:#E8B64A;}}
.container{{max-width:900px;margin:0 auto;padding:40px 24px;}}
.badge{{display:inline-block;background:linear-gradient(135deg,#E8B64A,#d4a43e);color:#080808;
  font-size:.65rem;font-weight:700;letter-spacing:.1em;padding:4px 12px;border-radius:20px;margin-bottom:20px;}}
h2{{font-size:1.6rem;margin-bottom:8px;}}
p.sub{{color:#888;font-size:.85rem;margin-bottom:40px;}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:20px;}}
.card{{background:#111;border:1px solid #1a1a1a;border-radius:16px;overflow:hidden;transition:transform .2s,border-color .2s;}}
.card:hover{{transform:translateY(-2px);border-color:#333;}}
.card-thumb{{aspect-ratio:16/9;background:linear-gradient(135deg,#1a1a1a,#0a0a0a);display:flex;align-items:center;justify-content:center;}}
.card-thumb span{{font-size:2.5rem;}}
.card-body{{padding:20px;}}
.card-body h3{{font-size:.95rem;margin-bottom:6px;color:#fff;}}
.card-body p{{font-size:.78rem;color:#888;line-height:1.5;}}
.card-body .tag{{display:inline-block;background:#1a1a1a;color:#E8B64A;font-size:.65rem;padding:3px 8px;border-radius:6px;margin-top:10px;}}
.coming{{opacity:.5;}}
.coming .card-body .tag{{color:#666;}}
</style>
<script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body>
<div class="header">
  <h1>🌟 ファンクラブ</h1>
  <div class="info">
    <span style="margin-right:16px;">{email}</span>
    <a href="/fanclub/logout">ログアウト</a>
  </div>
</div>
<div class="container">
  <div class="badge">MEMBER EXCLUSIVE</div>
  <h2>メンバー限定コンテンツ</h2>
  <p class="sub">ご支援ありがとうございます。こちらのコンテンツはファンクラブ会員専用です。</p>
  <div class="grid">
    <a class="card" href="/anime/behind.html" style="text-decoration:none;color:inherit;">
      <div class="card-thumb"><span>🎬</span></div>
      <div class="card-body">
        <h3>メイキング映像</h3>
        <p>アニメ制作の舞台裏。プロダクションノートと未公開シーンをお届けします。</p>
        <span class="tag">BEHIND THE SCENES</span>
      </div>
    </a>
    <div class="card coming">
      <div class="card-thumb"><span>🎵</span></div>
      <div class="card-body">
        <h3>未公開楽曲</h3>
        <p>制作中の楽曲デモや未リリーストラックをいち早くお届けします。</p>
        <span class="tag">COMING SOON</span>
      </div>
    </div>
    <div class="card coming">
      <div class="card-thumb"><span>💬</span></div>
      <div class="card-body">
        <h3>会員限定レター</h3>
        <p>月1回のプライベートニュースレター。プロジェクトの最新情報と思考をシェア。</p>
        <span class="tag">COMING SOON</span>
      </div>
    </div>
    <div class="card coming">
      <div class="card-thumb"><span>🎤</span></div>
      <div class="card-body">
        <h3>Q&Aセッション</h3>
        <p>会員限定のオンラインQ&Aに参加できます。直接質問してください。</p>
        <span class="tag">COMING SOON</span>
      </div>
    </div>
  </div>
</div>
</body>
</html>"#, email = email);

    ([("content-type", "text/html; charset=utf-8")], html).into_response()
}

async fn fanclub_logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    // Remove session if exists
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            for part in cookies.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("fanclub_auth=") {
                    let mut fc = state.fanclub_sessions.lock().unwrap();
                    fc.remove(token);
                    let dash = state.dash_sessions.lock().unwrap();
                    persist_sessions(&fc, &dash);
                }
            }
        }
    }
    Response::builder()
        .status(302)
        .header("set-cookie", "fanclub_auth=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
        .header("location", "/fanclub")
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Html(NotFoundTemplate.render().unwrap_or_default()))
}

// ── Main ──

// ── Analytics ──

#[derive(serde::Deserialize)]
struct AnalyticsEvent {
    path: String,
    #[serde(default)] referrer: String,
    #[serde(default)] ua: String,
    #[serde(default)] duration: Option<u32>, // seconds spent on page
}

async fn analytics_log(body: axum::body::Bytes) -> Response {
    let body: AnalyticsEvent = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(_) => return (cors_headers(), Json(serde_json::json!({"ok": false}))).into_response(),
    };
    let ts = chrono::Utc::now().to_rfc3339();
    let dur = body.duration.unwrap_or(0);
    let line = format!("{}\t{}\t{}\t{}\t{}\n", ts, body.path, body.referrer, body.ua, dur);
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::OpenOptions::new().create(true).append(true)
        .open("/data/analytics.tsv")
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

/// Generate a cryptographically random session token (32 bytes, hex-encoded = 64 chars).
fn generate_session_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

/// Check dashboard auth using session cookie stored in `dash_sessions`.
/// Returns `Some(Response)` if the request is NOT authorized (caller should return it),
/// or `None` if the request IS authorized.
fn check_dashboard_auth_state(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<Response> {
    if std::env::var("DASHBOARD_PASSWORD").ok().filter(|s| !s.is_empty()).is_none() {
        return None; // no password set = open access
    }

    // Check cookie — look up token in session store
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            for part in cookies.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("dash_auth=") {
                    let store = state.dash_sessions.lock().unwrap();
                    if let Some(&exp) = store.get(token) {
                        if now_secs() < exp {
                            return None; // authorized
                        }
                    }
                }
            }
        }
    }

    // Not authorized — show login form (POST, so password never appears in URL/logs)
    let html = r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Login — Dashboard</title>
<style>*{margin:0;padding:0;box-sizing:border-box;}body{background:#080808;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;}
.login{background:#111;border:1px solid #1a1a1a;border-radius:16px;padding:40px;max-width:360px;width:90%;text-align:center;}
h1{font-size:1.2rem;color:#E8B64A;margin-bottom:8px;}p{font-size:.75rem;color:#666;margin-bottom:24px;}
input{width:100%;padding:12px;background:#0a0a0a;border:1px solid #222;border-radius:8px;color:#e0e0e0;font-size:.9rem;margin-bottom:12px;outline:none;}
input:focus{border-color:#E8B64A;}
button{width:100%;padding:12px;background:#E8B64A;color:#080808;border:none;border-radius:8px;font-weight:600;font-size:.9rem;cursor:pointer;}
button:hover{background:#d4a43e;}</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head><body>
<div class="login"><h1>Dashboard</h1><p>yukihamada.jp</p>
<form method="POST" action="/dashboard/login"><input type="password" name="pw" placeholder="Password" autocomplete="current-password">
<button type="submit">Login</button></form></div></body></html>"#;
    Some(([("content-type", "text/html; charset=utf-8")], html).into_response())
}

#[derive(serde::Deserialize)]
struct DashLoginForm { pw: String }

async fn dashboard_login_post(
    State(state): State<Arc<AppState>>,
    axum::Form(body): axum::Form<DashLoginForm>,
) -> Response {
    let password = match std::env::var("DASHBOARD_PASSWORD").ok().filter(|s| !s.is_empty()) {
        Some(p) => p,
        None => return Redirect::to("/dashboard").into_response(),
    };
    if constant_time_eq(body.pw.as_bytes(), password.as_bytes()) {
        let token = generate_session_token();
        let exp = now_secs() + 7 * 24 * 3600; // 7 days
        {
            let mut dash = state.dash_sessions.lock().unwrap();
            dash.insert(token.clone(), exp);
            let fc = state.fanclub_sessions.lock().unwrap();
            persist_sessions(&fc, &dash);
        }
        let cookie = format!("dash_auth={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800");
        Response::builder()
            .status(302)
            .header("set-cookie", cookie)
            .header("location", "/dashboard")
            .body(axum::body::Body::empty()).unwrap()
    } else {
        // Wrong password — back to login form with error hint
        Redirect::to("/dashboard").into_response()
    }
}


#[derive(serde::Deserialize)]
struct DashboardQuery {
    pw: Option<String>,
}

fn parse_ua_device(ua: &str) -> (&'static str, &'static str, &'static str) {
    // (device_type, os, app)
    let device = if ua.contains("iPhone") || (ua.contains("Android") && ua.contains("Mobile")) {
        "Mobile"
    } else if ua.contains("iPad") || (ua.contains("Android") && !ua.contains("Mobile")) {
        "Tablet"
    } else { "Desktop" };

    let os = if ua.contains("iPhone") || ua.contains("iPad") { "iOS" }
        else if ua.contains("Android") { "Android" }
        else if ua.contains("Mac OS X") { "macOS" }
        else if ua.contains("Windows") { "Windows" }
        else if ua.contains("Linux") { "Linux" }
        else { "Other" };

    let app = if ua.contains("FBAN/FBIOS") || ua.contains("FB_IAB/FB4A") || ua.contains("MetaIAB Facebook") { "Facebook" }
        else if ua.contains("Instagram") { "Instagram" }
        else if ua.contains("Safari Line/") || ua.contains(" Line/") { "LINE" }
        else if ua.contains("Twitter") || ua.contains("Twitterbot") { "X/Twitter" }
        else if ua.contains("Chrome/") && !ua.contains("Edg/") { "Chrome" }
        else if ua.contains("Safari/") && !ua.contains("Chrome/") { "Safari" }
        else if ua.contains("Edg/") { "Edge" }
        else if ua.contains("Firefox/") { "Firefox" }
        else { "Other" };

    (device, os, app)
}

fn extract_lang_from_ua(ua: &str) -> &str {
    // Facebook UA contains FBLC/ja_JP or FBLC/en_US
    if let Some(pos) = ua.find("FBLC/") {
        let rest = &ua[pos+5..];
        if rest.len() >= 2 {
            let lang = &rest[..2];
            if lang == "ja" { return "ja"; }
            if lang == "en" { return "en"; }
        }
    }
    if ua.contains("ja") { "ja" } else if ua.contains("en") { "en" } else { "other" }
}

async fn admin_analytics(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    // Check admin_auth cookie against admin_sessions
    let authed = headers.get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                part.trim().strip_prefix("admin_auth=").map(|t| t.to_string())
            })
        })
        .map(|token| {
            let sessions = state.admin_sessions.lock().unwrap();
            sessions.get(&token)
                .map(|(email, exp)| {
                    (email == "mail@yukihamada.jp" || email == "yuki@hamada.tokyo") && *exp > now_secs()
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !authed {
        let html = r#"<!DOCTYPE html><html lang="ja"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Analytics — yukihamada.jp</title>
<style>*{margin:0;padding:0;box-sizing:border-box;}body{background:#080808;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;}
.box{background:#111;border:1px solid #1a1a1a;border-radius:16px;padding:40px;max-width:380px;width:90%;text-align:center;}
h1{font-size:1.1rem;color:#E8B64A;margin-bottom:6px;}p{font-size:.75rem;color:#666;margin-bottom:24px;}
input{width:100%;padding:12px;background:#0a0a0a;border:1px solid #222;border-radius:8px;color:#e0e0e0;font-size:.9rem;margin-bottom:10px;outline:none;text-align:center;letter-spacing:.15em;}
input:focus{border-color:#E8B64A;}
button{width:100%;padding:12px;background:#E8B64A;color:#080808;border:none;border-radius:8px;font-weight:700;font-size:.9rem;cursor:pointer;margin-bottom:8px;}
button:hover{opacity:.85;}.msg{font-size:.8rem;color:#E8B64A;margin-top:8px;min-height:20px;}
#step2{display:none;}</style></head><body>
<div class="box">
  <h1>Analytics</h1><p>yukihamada.jp</p>
  <div id="step1">
    <button onclick="sendOtp()">メールでコードを送る</button>
    <div class="msg" id="msg1"></div>
  </div>
  <div id="step2">
    <p style="margin-bottom:14px;font-size:.8rem;color:#aaa;">mail@yukihamada.jp に送られたコードを入力</p>
    <input id="code" type="text" inputmode="numeric" placeholder="000000" maxlength="6">
    <button onclick="verify()">ログイン</button>
    <div class="msg" id="msg2"></div>
  </div>
</div>
<script>
async function sendOtp() {
  document.getElementById('msg1').textContent = '送信中...';
  const r = await fetch('/api/login/otp', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:'mail@yukihamada.jp'})});
  const d = await r.json();
  if (d.ok) { document.getElementById('step1').style.display='none'; document.getElementById('step2').style.display='block'; }
  else { document.getElementById('msg1').textContent = d.error || 'エラー'; }
}
async function verify() {
  const code = document.getElementById('code').value.trim();
  document.getElementById('msg2').textContent = '確認中...';
  const r = await fetch('/api/login/verify', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:'mail@yukihamada.jp',code})});
  const d = await r.json();
  if (d.ok && d.token) {
    document.cookie = `admin_auth=${d.token}; path=/; max-age=${30*24*3600}; SameSite=Lax`;
    location.reload();
  } else { document.getElementById('msg2').textContent = d.error || 'コードが正しくありません'; }
}
document.addEventListener('keydown', e => { if(e.key==='Enter') { if(document.getElementById('step2').style.display==='block') verify(); else sendOtp(); }});
</script></body></html>"#;
        return ([("content-type", "text/html; charset=utf-8")], html).into_response();
    }

    let html = r#"<!DOCTYPE html><html lang="ja"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Analytics — yukihamada.jp</title>
<style>
*{margin:0;padding:0;box-sizing:border-box;}
body{background:#080808;color:#e0e0e0;font-family:system-ui;padding:24px;}
h1{font-size:1.1rem;color:#E8B64A;margin-bottom:4px;}
.sub{font-size:.75rem;color:#555;margin-bottom:24px;}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px;margin-bottom:24px;}
.card{background:#111;border:1px solid #1a1a1a;border-radius:12px;padding:18px;}
.card-label{font-size:.65rem;letter-spacing:.12em;text-transform:uppercase;color:#555;margin-bottom:6px;}
.card-val{font-size:1.8rem;font-weight:700;color:#E8B64A;}
table{width:100%;border-collapse:collapse;font-size:.8rem;}
th{text-align:left;padding:8px 10px;color:#555;font-weight:400;border-bottom:1px solid #1a1a1a;}
td{padding:8px 10px;border-bottom:1px solid #111;}
tr:hover td{background:#111;}
.section{background:#0d0d0d;border:1px solid #1a1a1a;border-radius:12px;padding:18px;margin-bottom:16px;}
.section h2{font-size:.8rem;color:#aaa;margin-bottom:14px;letter-spacing:.08em;}
.logout{float:right;font-size:.7rem;color:#444;text-decoration:none;cursor:pointer;}
.logout:hover{color:#E8B64A;}
.days{display:flex;gap:8px;margin-bottom:20px;}
.days button{padding:6px 14px;background:#111;border:1px solid #222;border-radius:6px;color:#aaa;font-size:.75rem;cursor:pointer;}
.days button.active{background:#E8B64A;color:#080808;border-color:#E8B64A;}
</style></head><body>
<div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px;">
  <h1>Analytics</h1>
  <a class="logout" onclick="logout()">ログアウト</a>
</div>
<div class="sub">yukihamada.jp + 全サイト</div>
<div class="days">
  <button class="active" onclick="load(7,this)">7日</button>
  <button onclick="load(30,this)">30日</button>
  <button onclick="load(90,this)">90日</button>
</div>
<div class="grid" id="stats"></div>
<div class="section"><h2>サイト別 PV</h2><table id="sites-table"><tr><th>サイト</th><th>今日</th><th>7d</th><th>30d</th></tr></table></div>
<div class="section"><h2>ページ別 PV（全サイト）</h2><table id="pages-table"><tr><th>ページ</th><th>PV</th></tr></table></div>
<div class="section"><h2>流入元</h2><table id="ref-table"><tr><th>Referrer</th><th>PV</th></tr></table></div>
<script>
const BASE = 'https://enabler-analytics.fly.dev';
function fmt(n){return n>=1000?(n/1000).toFixed(1)+'k':n;}
async function load(days, btn) {
  document.querySelectorAll('.days button').forEach(b=>b.classList.remove('active'));
  if(btn) btn.classList.add('active');
  const [stats, breakdown, pages, refs] = await Promise.all([
    fetch(`${BASE}/api/stats?days=${days}`).then(r=>r.json()).catch(()=>({})),
    fetch(`${BASE}/api/site-breakdown?days=${days}`).then(r=>r.json()).catch(()=>[]),
    fetch(`${BASE}/api/pages?days=${days}`).then(r=>r.json()).catch(()=>[]),
    fetch(`${BASE}/api/referrers?days=${days}`).then(r=>r.json()).catch(()=>[]),
  ]);
  document.getElementById('stats').innerHTML = `
    <div class="card"><div class="card-label">今日</div><div class="card-val">${fmt(stats.today||0)}</div></div>
    <div class="card"><div class="card-label">今週</div><div class="card-val">${fmt(stats.this_week||0)}</div></div>
    <div class="card"><div class="card-label">今月</div><div class="card-val">${fmt(stats.this_month||0)}</div></div>
    <div class="card"><div class="card-label">${days}日合計</div><div class="card-val">${fmt(stats.total||0)}</div></div>
  `;
  const sites = Array.isArray(breakdown)?breakdown:[];
  document.getElementById('sites-table').innerHTML = '<tr><th>サイト</th><th>今日</th><th>7d</th><th>30d</th></tr>'
    + sites.map(s=>`<tr><td>${s.site||s.domain||'-'}</td><td>${fmt(s.today||0)}</td><td>${fmt(s.last_7d||0)}</td><td>${fmt(s.last_30d||0)}</td></tr>`).join('');
  const pg = Array.isArray(pages)?pages.slice(0,30):[];
  document.getElementById('pages-table').innerHTML = '<tr><th>ページ</th><th>PV</th></tr>'
    + pg.map(p=>`<tr><td style="max-width:320px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${p.path||p.page||'-'}</td><td>${fmt(p.count||p.pv||0)}</td></tr>`).join('');
  const rf = Array.isArray(refs)?refs.slice(0,20):[];
  document.getElementById('ref-table').innerHTML = '<tr><th>Referrer</th><th>PV</th></tr>'
    + rf.map(r=>`<tr><td>${r.referrer||r.ref||'-'}</td><td>${fmt(r.count||r.pv||0)}</td></tr>`).join('');
}
function logout(){document.cookie='admin_auth=;path=/;max-age=0';location.reload();}
load(7);
</script></body></html>"#;
    ([("content-type", "text/html; charset=utf-8")], html).into_response()
}

async fn analytics_dashboard(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    // Auth check
    if let Some(resp) = check_dashboard_auth_state(&state, &headers) { return resp; }

    let data = std::fs::read_to_string("/data/analytics.tsv").unwrap_or_default();
    let mut page_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut referrer_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut daily_counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut hourly: [usize; 24] = [0; 24];
    let mut total = 0usize;
    let mut today_count = 0usize;
    let mut yesterday_count = 0usize;
    let now = chrono::Utc::now();
    let jst = chrono::FixedOffset::east_opt(9*3600).unwrap();
    let today_str = now.with_timezone(&jst).format("%Y-%m-%d").to_string();
    let yesterday_str = (now - chrono::Duration::days(1)).with_timezone(&jst).format("%Y-%m-%d").to_string();
    let week_ago_str = (now - chrono::Duration::days(7)).with_timezone(&jst).format("%Y-%m-%d").to_string();
    let two_weeks_ago_str = (now - chrono::Duration::days(14)).with_timezone(&jst).format("%Y-%m-%d").to_string();
    let month_ago_str = (now - chrono::Duration::days(30)).with_timezone(&jst).format("%Y-%m-%d").to_string();
    let mut recent: Vec<(String,String,String)> = Vec::new();
    let mut section_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // New KPI tracking
    let mut unique_uas: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut today_uas: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut device_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut os_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut app_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut lang_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut week_count = 0usize;
    let mut prev_week_count = 0usize;
    let mut month_count = 0usize;
    // Session tracking: group by UA, track pages per session
    let mut session_pages: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    // Referrer source grouping
    let mut source_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    // Duration tracking
    let mut total_duration = 0u64;
    let mut duration_count = 0usize;
    let mut page_durations: std::collections::HashMap<String, (u64, usize)> = std::collections::HashMap::new();

    for line in data.lines().rev() {
        let parts: Vec<&str> = line.splitn(5, '\t').collect();
        if parts.len() < 2 { continue; }
        let ts = parts[0];
        let path = parts[1];
        let referrer = if parts.len() > 2 { parts[2] } else { "" };
        let ua_raw = if parts.len() > 3 { parts[3] } else { "" };
        // Duration is 5th field (may not exist for old data)
        let dur: u32 = if parts.len() > 4 { parts[4].trim().parse().unwrap_or(0) } else { 0 };
        let ua = ua_raw;
        if dur > 0 && dur < 3600 {
            total_duration += dur as u64;
            duration_count += 1;
            let e = page_durations.entry(path.to_string()).or_insert((0, 0));
            e.0 += dur as u64;
            e.1 += 1;
        }
        total += 1;
        *page_counts.entry(path.to_string()).or_insert(0) += 1;

        // UA-based metrics
        let ua_key = if ua.len() > 80 { &ua[..80] } else { ua };
        unique_uas.insert(ua_key.to_string());

        let (device, os, app) = parse_ua_device(ua);
        *device_counts.entry(device).or_insert(0) += 1;
        *os_counts.entry(os).or_insert(0) += 1;
        *app_counts.entry(app).or_insert(0) += 1;

        let lang = extract_lang_from_ua(ua);
        *lang_counts.entry(lang).or_insert(0) += 1;

        // Session pages
        session_pages.entry(ua_key.to_string()).or_default().push(path.to_string());

        // Section grouping
        let section = if path.starts_with("/anime") { "Anime" }
            else if path.starts_with("/mv") { "MV" }
            else if path.starts_with("/blog") { "Blog" }
            else if path == "/" { "Home" }
            else { "Other" };
        *section_counts.entry(section.to_string()).or_insert(0) += 1;

        // Referrer source grouping
        let source = if referrer.contains("facebook.com") || referrer.contains("fb.com") { "Facebook" }
            else if referrer.contains("instagram.com") { "Instagram" }
            else if referrer.contains("twitter.com") || referrer.contains("t.co") || referrer.contains("x.com") { "X/Twitter" }
            else if referrer.contains("google.") { "Google" }
            else if referrer.contains("yahoo.") { "Yahoo" }
            else if referrer.contains("line.me") || referrer.contains("line.naver") { "LINE" }
            else if referrer.contains("yukihamada.jp") { "Internal" }
            else if referrer.is_empty() { "Direct" }
            else { "Other" };
        *source_counts.entry(source).or_insert(0) += 1;

        // Raw referrer
        if !referrer.is_empty() && referrer != "/" {
            let r = if referrer.len() > 60 { &referrer[..60] } else { referrer };
            *referrer_counts.entry(r.to_string()).or_insert(0) += 1;
        }

        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            use chrono::Timelike;
            let jst_dt = dt.with_timezone(&jst);
            let h = jst_dt.hour() as usize;
            hourly[h] += 1;
            let day = jst_dt.format("%Y-%m-%d").to_string();
            *daily_counts.entry(day.clone()).or_insert(0) += 1;
            if day == today_str {
                today_count += 1;
                today_uas.insert(ua_key.to_string());
            }
            if day == yesterday_str { yesterday_count += 1; }
            if day >= week_ago_str { week_count += 1; }
            if day >= two_weeks_ago_str && day < week_ago_str { prev_week_count += 1; }
            if day >= month_ago_str { month_count += 1; }
        }

        if recent.len() < 50 {
            let short_ts = if ts.len() >= 19 { &ts[11..19] } else { ts };
            recent.push((short_ts.to_string(), path.to_string(), referrer.to_string()));
        }
    }

    // Computed KPIs
    let unique_visitors = unique_uas.len();
    let today_unique = today_uas.len();
    let pages_per_session = if unique_visitors > 0 { total as f64 / unique_visitors as f64 } else { 0.0 };
    let bounce_sessions = session_pages.values().filter(|pages| pages.len() == 1).count();
    let bounce_rate = if !session_pages.is_empty() { bounce_sessions as f64 / session_pages.len() as f64 * 100.0 } else { 0.0 };
    let wow_growth = if prev_week_count > 0 { ((week_count as f64 / prev_week_count as f64 - 1.0) * 100.0) as i64 } else { 0 };
    let avg_duration = if duration_count > 0 { total_duration as f64 / duration_count as f64 } else { 0.0 };
    let avg_dur_min = (avg_duration / 60.0) as u32;
    let avg_dur_sec = (avg_duration % 60.0) as u32;

    // Top pages
    let mut sorted_pages: Vec<_> = page_counts.iter().collect();
    sorted_pages.sort_by(|a, b| b.1.cmp(a.1));

    // Top referrers
    let mut sorted_refs: Vec<_> = referrer_counts.iter().collect();
    sorted_refs.sort_by(|a, b| b.1.cmp(a.1));

    // Sorted sources
    let mut sorted_sources: Vec<_> = source_counts.iter().collect();
    sorted_sources.sort_by(|a, b| b.1.cmp(a.1));

    let mut sorted_sections: Vec<_> = section_counts.iter().collect();
    sorted_sections.sort_by(|a, b| b.1.cmp(a.1));

    let mut sorted_devices: Vec<_> = device_counts.iter().collect();
    sorted_devices.sort_by(|a, b| b.1.cmp(a.1));

    let mut sorted_os: Vec<_> = os_counts.iter().collect();
    sorted_os.sort_by(|a, b| b.1.cmp(a.1));

    let mut sorted_apps: Vec<_> = app_counts.iter().collect();
    sorted_apps.sort_by(|a, b| b.1.cmp(a.1));

    let mut sorted_langs: Vec<_> = lang_counts.iter().collect();
    sorted_langs.sort_by(|a, b| b.1.cmp(a.1));

    let hourly_max = *hourly.iter().max().unwrap_or(&1).max(&1);
    let daily_max = daily_counts.values().cloned().max().unwrap_or(1).max(1);
    let daily_recent: Vec<_> = daily_counts.iter().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect();

    // Growth
    let growth = if yesterday_count > 0 { ((today_count as f64 / yesterday_count as f64 - 1.0) * 100.0) as i64 } else { 0 };
    let growth_str = if growth >= 0 { format!("+{}%", growth) } else { format!("{}%", growth) };
    let growth_color = if growth >= 0 { "#3fb950" } else { "#f85149" };
    let wow_str = if wow_growth >= 0 { format!("+{}%", wow_growth) } else { format!("{}%", wow_growth) };
    let wow_color = if wow_growth >= 0 { "#3fb950" } else { "#f85149" };

    // Peak hour
    let peak_hour = hourly.iter().enumerate().max_by_key(|(_, c)| *c).map(|(h, _)| h).unwrap_or(0);

    // Build HTML sections
    let top_pages_html: String = sorted_pages.iter().take(25).enumerate()
        .map(|(i, (p, c))| {
            let bar_w = **c as f64 / *sorted_pages[0].1 as f64 * 100.0;
            let icon = if p.contains("anime") { "🎬" } else if p.contains("mv") { "🎵" } else if p.contains("blog") { "📝" } else if **p == "/" { "🏠" } else { "📄" };
            let dur_str = page_durations.get(p.as_str()).map(|(total, cnt)| {
                let avg = *total as f64 / *cnt as f64;
                format!("<span style='font-size:.6rem;color:#666;margin-left:6px;'>{}:{:02}</span>", avg as u32 / 60, avg as u32 % 60)
            }).unwrap_or_default();
            format!("<div class='pg'><span class='pg-rank'>{}</span><span class='pg-icon'>{}</span><div class='pg-info'><div class='pg-path'>{}{}</div><div class='pg-bar' style='width:{}%'></div></div><span class='pg-count'>{}</span></div>", i+1, icon, p, dur_str, bar_w, c)
        }).collect();

    let hourly_html: String = hourly.iter().enumerate()
        .map(|(h, c)| {
            let pct = *c as f64 / hourly_max as f64 * 100.0;
            let is_now = { use chrono::Timelike; now.with_timezone(&jst).hour() as usize == h };
            let is_peak = h == peak_hour && *c > 0;
            let style = if is_now { ";background:#E8B64A;" } else if is_peak { ";background:#D4764A;" } else { "" };
            format!("<div class='hb'><div class='hb-fill' style='height:{}%{}'></div><div class='hb-label'>{}</div></div>", pct, style, h)
        }).collect();

    let daily_html: String = daily_recent.iter()
        .map(|(d, c)| {
            let pct = **c as f64 / daily_max as f64 * 100.0;
            let is_today = **d == today_str;
            format!("<div class='db'><div class='db-fill' style='height:{}%{}'></div><div class='db-label'>{}</div></div>",
                pct, if is_today { ";background:#E8B64A;" } else { "" }, &d[5..])
        }).collect();

    let sections_html: String = sorted_sections.iter()
        .map(|(s, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            let color = match s.as_str() { "Anime" => "#E8B64A", "MV" => "#D4764A", "Blog" => "#58a6ff", "Home" => "#3fb950", _ => "#666" };
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:{}'></div><span class='sec-name'>{}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, color, s, c, pct)
        }).collect();

    let sources_html: String = sorted_sources.iter()
        .map(|(s, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            let color = match **s { "Facebook" => "#1877f2", "Instagram" => "#e4405f", "X/Twitter" => "#1da1f2", "Google" => "#34a853", "LINE" => "#06c755", "Direct" => "#E8B64A", "Internal" => "#666", _ => "#888" };
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:{}'></div><span class='sec-name'>{}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, color, s, c, pct)
        }).collect();

    let devices_html: String = sorted_devices.iter()
        .map(|(d, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            let icon = match **d { "Mobile" => "📱", "Desktop" => "💻", "Tablet" => "📟", _ => "?" };
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:#E8B64A'></div><span class='sec-name'>{} {}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, icon, d, c, pct)
        }).collect();

    let os_html: String = sorted_os.iter()
        .map(|(o, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:#58a6ff'></div><span class='sec-name'>{}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, o, c, pct)
        }).collect();

    let apps_html: String = sorted_apps.iter()
        .map(|(a, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            let color = match **a { "Facebook" => "#1877f2", "Instagram" => "#e4405f", "Chrome" => "#4285f4", "Safari" => "#006cff", "LINE" => "#06c755", _ => "#888" };
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:{}'></div><span class='sec-name'>{}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, color, a, c, pct)
        }).collect();

    let langs_html: String = sorted_langs.iter()
        .map(|(l, c)| {
            let pct = **c as f64 / total.max(1) as f64 * 100.0;
            let label = match **l { "ja" => "🇯🇵 Japanese", "en" => "🇺🇸 English", _ => "🌐 Other" };
            format!("<div class='sec'><div class='sec-bar' style='width:{}%;background:#3fb950'></div><span class='sec-name'>{}</span><span class='sec-count'>{} ({:.0}%)</span></div>", pct, label, c, pct)
        }).collect();

    let refs_html: String = sorted_refs.iter().take(10)
        .map(|(r, c)| format!("<div class='ref'><span class='ref-url'>{}</span><span class='ref-count'>{}</span></div>", r, c))
        .collect();

    let recent_html: String = recent.iter()
        .map(|(ts, path, _)| {
            let icon = if path.contains("anime") { "🎬" } else if path.contains("mv") { "🎵" } else if path.contains("blog") { "📝" } else { "·" };
            format!("<div class='rv'><span class='rv-time'>{}</span><span class='rv-icon'>{}</span><span class='rv-path'>{}</span></div>", ts, icon, path)
        }).collect();

    // ── Member Stats ──
    let newsletter_data = std::fs::read_to_string("/data/newsletter.txt").unwrap_or_default();
    let fanclub_data = std::fs::read_to_string("/data/fanclub_events.txt").unwrap_or_default();
    let mut nl_total = 0usize;
    let mut nl_today = 0usize;
    let mut nl_week = 0usize;
    let mut nl_recent: Vec<(String, String)> = Vec::new();
    for line in newsletter_data.lines().rev() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() < 2 { continue; }
        nl_total += 1;
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(parts[0]) {
            let day = dt.with_timezone(&jst).format("%Y-%m-%d").to_string();
            if day == today_str { nl_today += 1; }
            if day >= week_ago_str { nl_week += 1; }
        }
        if nl_recent.len() < 10 {
            let short_ts = if parts[0].len() >= 10 { &parts[0][..10] } else { parts[0] };
            let masked = if parts[1].contains('@') {
                let at = parts[1].find('@').unwrap();
                format!("{}***@{}", &parts[1][..std::cmp::min(3, at)], &parts[1][at+1..])
            } else { parts[1].to_string() };
            nl_recent.push((short_ts.to_string(), masked));
        }
    }
    let mut fc_total = 0usize;
    let mut fc_today = 0usize;
    let mut fc_revenue = 0f64;
    let mut fc_recent: Vec<(String, String, String)> = Vec::new();
    for line in fanclub_data.lines().rev() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 { continue; }
        fc_total += 1;
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(parts[0]) {
            let day = dt.with_timezone(&jst).format("%Y-%m-%d").to_string();
            if day == today_str { fc_today += 1; }
        }
        if let Ok(amt) = parts.get(4).unwrap_or(&"0").parse::<f64>() {
            fc_revenue += amt;
        }
        if fc_recent.len() < 10 {
            let short_ts = if parts[0].len() >= 10 { &parts[0][..10] } else { parts[0] };
            let name = parts.get(3).unwrap_or(&"");
            let masked_email = if parts[2].contains('@') {
                let at = parts[2].find('@').unwrap();
                format!("{}***@{}", &parts[2][..std::cmp::min(3, at)], &parts[2][at+1..])
            } else { parts[2].to_string() };
            fc_recent.push((short_ts.to_string(), masked_email, name.to_string()));
        }
    }

    let members_html = format!(r#"<div class="panel"><h2>Members</h2>
<div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-bottom:12px;">
<div style="background:#0a0a0a;border-radius:8px;padding:10px;text-align:center;"><div style="font-size:1.3rem;font-weight:700;color:#E8B64A;">{nl_total}</div><div style="font-size:.55rem;color:#666;">Newsletter</div></div>
<div style="background:#0a0a0a;border-radius:8px;padding:10px;text-align:center;"><div style="font-size:1.3rem;font-weight:700;color:#22c55e;">{fc_total}</div><div style="font-size:.55rem;color:#666;">Fanclub</div></div>
<div style="background:#0a0a0a;border-radius:8px;padding:10px;text-align:center;"><div style="font-size:1rem;font-weight:700;color:#3b82f6;">{nl_today}</div><div style="font-size:.55rem;color:#666;">Today NL</div></div>
<div style="background:#0a0a0a;border-radius:8px;padding:10px;text-align:center;"><div style="font-size:1rem;font-weight:700;color:#3b82f6;">{fc_today}</div><div style="font-size:.55rem;color:#666;">Today FC</div></div>
</div>
<div style="font-size:.65rem;color:#888;margin-bottom:6px;">Revenue: ¥{rev:.0}</div>
<div style="font-size:.65rem;color:#666;margin-bottom:8px;">Newsletter (7d): {nl_week}</div>
<h3 style="font-size:.7rem;color:#E8B64A;margin:8px 0 4px;">Recent Newsletter</h3>
{nl_list}
<h3 style="font-size:.7rem;color:#22c55e;margin:8px 0 4px;">Recent Fanclub</h3>
{fc_list}
</div>"#,
        nl_total = nl_total, fc_total = fc_total, nl_today = nl_today, fc_today = fc_today,
        rev = fc_revenue, nl_week = nl_week,
        nl_list = nl_recent.iter().map(|(ts, email)| format!(
            "<div style='display:flex;justify-content:space-between;padding:3px 0;border-bottom:1px solid #111;font-size:.65rem;'><span style='color:#555;'>{}</span><span style='color:#999;'>{}</span></div>", ts, email
        )).collect::<String>(),
        fc_list = fc_recent.iter().map(|(ts, email, name)| format!(
            "<div style='display:flex;justify-content:space-between;padding:3px 0;border-bottom:1px solid #111;font-size:.65rem;'><span style='color:#555;'>{}</span><span style='color:#999;'>{} {}</span></div>", ts, name, email
        )).collect::<String>(),
    );

    let html = format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Dashboard — yukihamada.jp</title>
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{background:#080808;color:#e0e0e0;font-family:system-ui,-apple-system,sans-serif;}}
.wrap{{max-width:1200px;margin:0 auto;padding:16px;}}
header{{padding:20px 0 12px;display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:8px;}}
header h1{{font-size:1.2rem;color:#E8B64A;}}
header .meta{{font-size:.7rem;color:#666;}}
header a{{color:#E8B64A;text-decoration:none;font-size:.75rem;}}
.cards{{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px;margin:12px 0;}}
.card{{background:#111;border:1px solid #1a1a1a;border-radius:12px;padding:14px;}}
.card .num{{font-size:clamp(1.3rem,3.5vw,1.8rem);font-weight:700;color:#E8B64A;font-variant-numeric:tabular-nums;}}
.card .label{{font-size:.55rem;color:#666;margin-top:2px;letter-spacing:.05em;text-transform:uppercase;}}
.card .sub{{font-size:.65rem;margin-top:4px;}}
.grid{{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin:12px 0;}}
.grid3{{display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;margin:12px 0;}}
@media(max-width:900px){{.grid3{{grid-template-columns:1fr 1fr;}}}}
@media(max-width:700px){{.grid{{grid-template-columns:1fr;}}.grid3{{grid-template-columns:1fr;}}}}
.panel{{background:#111;border:1px solid #1a1a1a;border-radius:12px;padding:14px;}}
.panel h2{{font-size:.75rem;color:#E8B64A;margin-bottom:10px;letter-spacing:.05em;}}
.hourly{{display:flex;gap:2px;height:80px;align-items:flex-end;}}
.hb{{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;height:100%;}}
.hb-fill{{width:100%;background:#333;border-radius:2px 2px 0 0;min-height:1px;transition:height .3s;}}
.hb-label{{font-size:7px;color:#555;margin-top:2px;}}
.daily{{display:flex;gap:2px;height:80px;align-items:flex-end;}}
.db{{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;height:100%;}}
.db-fill{{width:100%;background:#333;border-radius:2px 2px 0 0;min-height:1px;}}
.db-label{{font-size:6px;color:#555;margin-top:2px;}}
.pg{{display:flex;align-items:center;gap:8px;padding:5px 0;border-bottom:1px solid #151515;}}
.pg-rank{{font-size:.65rem;color:#555;width:18px;text-align:right;}}
.pg-icon{{font-size:.75rem;}}
.pg-info{{flex:1;min-width:0;}}
.pg-path{{font-size:.75rem;color:#ccc;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}}
.pg-bar{{height:2px;background:#E8B64A;border-radius:1px;margin-top:3px;opacity:.3;}}
.pg-count{{font-size:.75rem;color:#E8B64A;font-weight:600;min-width:30px;text-align:right;}}
.sec{{display:flex;align-items:center;gap:8px;margin:5px 0;}}
.sec-bar{{height:6px;border-radius:3px;min-width:4px;}}
.sec-name{{font-size:.7rem;color:#ccc;min-width:70px;}}
.sec-count{{font-size:.65rem;color:#888;}}
.ref{{display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid #151515;}}
.ref-url{{font-size:.65rem;color:#888;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:70%;}}
.ref-count{{font-size:.7rem;color:#E8B64A;}}
.recent{{max-height:300px;overflow-y:auto;}}
.rv{{display:flex;gap:6px;padding:3px 0;border-bottom:1px solid #111;font-size:.7rem;}}
.rv-time{{color:#555;font-family:monospace;min-width:55px;}}
.rv-icon{{min-width:16px;text-align:center;}}
.rv-path{{color:#999;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}}
.kpi-grid{{display:grid;grid-template-columns:1fr 1fr;gap:6px;}}
.kpi{{text-align:center;padding:8px;background:#0a0a0a;border-radius:8px;}}
.kpi .kv{{font-size:1.1rem;font-weight:700;color:#E8B64A;}}
.kpi .kl{{font-size:.55rem;color:#666;margin-top:2px;text-transform:uppercase;}}
</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head><body><div class="wrap">
<header>
<div><h1>Dashboard</h1><div class="meta">yukihamada.jp — {today} JST</div></div>
<a href="/">← Back to site</a>
</header>

<div class="cards">
<div class="card"><div class="num">{total}</div><div class="label">Total PV</div></div>
<div class="card"><div class="num">{unique}</div><div class="label">Unique Visitors</div></div>
<div class="card"><div class="num">{today_pv}</div><div class="label">Today PV</div><div class="sub" style="color:{growth_color}">{growth} vs yesterday</div></div>
<div class="card"><div class="num">{today_uv}</div><div class="label">Today UV</div></div>
<div class="card"><div class="num">{week}</div><div class="label">7-day PV</div><div class="sub" style="color:{wow_color}">{wow} WoW</div></div>
<div class="card"><div class="num">{month}</div><div class="label">30-day PV</div></div>
<div class="card"><div class="num">{pages_per_s:.1}</div><div class="label">Pages/Session</div></div>
<div class="card"><div class="num">{bounce:.0}%</div><div class="label">Bounce Rate</div></div>
<div class="card"><div class="num">{dur_min}:{dur_sec:02}</div><div class="label">Avg Duration</div></div>
</div>

<div class="grid">
<div class="panel"><h2>Hourly Distribution (JST) — Peak: {peak}:00</h2><div class="hourly">{hourly}</div></div>
<div class="panel"><h2>Daily Trend (30 days)</h2><div class="daily">{daily}</div></div>
</div>

<div class="grid3">
<div class="panel"><h2>Traffic Sources</h2>{sources}</div>
<div class="panel"><h2>Device Type</h2>{devices}<div style="margin-top:12px;"><h2 style="font-size:.75rem;color:#E8B64A;margin-bottom:8px;">OS</h2>{os_breakdown}</div></div>
<div class="panel"><h2>Browser / App</h2>{apps}<div style="margin-top:12px;"><h2 style="font-size:.75rem;color:#E8B64A;margin-bottom:8px;">Language</h2>{langs}</div></div>
</div>

<div class="grid">
<div class="panel"><h2>Top Pages</h2>{top_pages}</div>
<div class="panel">
<div style="margin-bottom:14px;"><h2 style="font-size:.75rem;color:#E8B64A;margin-bottom:8px;">Content Sections</h2>{sections}</div>
<div style="margin-bottom:14px;"><h2 style="font-size:.75rem;color:#E8B64A;margin-bottom:8px;">Raw Referrers</h2>{refs}</div>
<div><h2 style="font-size:.75rem;color:#E8B64A;margin-bottom:8px;">Live Feed</h2><div class="recent">{recent}</div></div>
</div>
</div>

<div class="grid" style="grid-template-columns:1fr 1fr;">
{members}
<div class="panel" style="text-align:center;">
<h2>Links</h2>
<div style="display:flex;flex-direction:column;gap:8px;margin-top:12px;">
<a href="/dashboard/x" style="display:block;padding:10px;background:rgba(232,182,74,.08);border:1px solid rgba(232,182,74,.2);border-radius:8px;text-decoration:none;color:#E8B64A;font-size:.8rem;">X Posts Manager</a>
<a href="/" style="display:block;padding:10px;background:rgba(255,255,255,.03);border:1px solid #1a1a1a;border-radius:8px;text-decoration:none;color:#888;font-size:.8rem;">View Site</a>
</div>
</div>
</div>

<div style="text-align:center;padding:20px;font-size:.6rem;color:#333;">Auto-refresh 30s | {unique_pages} pages tracked</div>
</div>
<script>setTimeout(()=>location.reload(),30000);</script>
</body></html>"#,
        today = today_str,
        total = total,
        unique = unique_visitors,
        today_pv = today_count,
        today_uv = today_unique,
        growth = growth_str,
        growth_color = growth_color,
        week = week_count,
        month = month_count,
        wow = wow_str,
        wow_color = wow_color,
        pages_per_s = pages_per_session,
        bounce = bounce_rate,
        peak = peak_hour,
        hourly = hourly_html,
        daily = daily_html,
        sources = sources_html,
        devices = devices_html,
        os_breakdown = os_html,
        apps = apps_html,
        langs = langs_html,
        top_pages = top_pages_html,
        sections = sections_html,
        refs = refs_html,
        recent = recent_html,
        unique_pages = sorted_pages.len(),
        members = members_html,
        dur_min = avg_dur_min,
        dur_sec = avg_dur_sec,
    );

    ([("content-type", "text/html; charset=utf-8")], html).into_response()
}

// ── X (Twitter) Integration ──

use base64::Engine;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct XTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct XDraft {
    id: String,
    text: String,
    status: String, // "draft", "approved", "published"
    created_at: String,
    published_at: Option<String>,
    tweet_id: Option<String>,
    category: String,
}

fn load_x_tokens() -> Option<XTokens> {
    std::fs::read_to_string("/data/x_tokens.json").ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn save_x_tokens(tokens: &XTokens) {
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::write("/data/x_tokens.json", serde_json::to_string(tokens).unwrap_or_default());
}

fn load_x_drafts() -> Vec<XDraft> {
    std::fs::read_to_string("/data/x_drafts.json").ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_x_drafts(drafts: &[XDraft]) {
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::write("/data/x_drafts.json", serde_json::to_string(drafts).unwrap_or_default());
}

// OAuth 2.0 PKCE: Step 1 - Redirect to X authorization
async fn x_auth_start(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if check_dashboard_auth_state(&state, &headers).is_some() {
        return Redirect::to("/dashboard").into_response();
    }
    let client_id = std::env::var("X_CLIENT_ID").unwrap_or_default();
    let redirect_uri = "https://yukihamada.jp/api/x/callback";
    // Generate PKCE verifier
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let verifier: String = (0..64).map(|_| {
        let idx = rng.gen_range(0..62);
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"[idx] as char
    }).collect();
    // S256 challenge using real SHA-256
    use sha2::Digest;
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash);
    // Save verifier
    let _ = std::fs::create_dir_all("/data");
    let _ = std::fs::write("/data/x_pkce_verifier.txt", &verifier);
    let state: String = (0..16).map(|_| format!("{:x}", rng.gen_range(0u8..16))).collect();
    let _ = std::fs::write("/data/x_oauth_state.txt", &state);

    let url = format!(
        "https://x.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope=tweet.read+tweet.write+users.read+offline.access&state={}&code_challenge={}&code_challenge_method=S256",
        urlenc(&client_id), urlenc(redirect_uri), urlenc(&state), urlenc(&challenge)
    );
    Redirect::to(&url).into_response()
}

fn urlenc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => { out.push('%'); out.push_str(&format!("{:02X}", b)); }
        }
    }
    out
}

// OAuth 2.0 PKCE: Step 2 - Handle callback
#[derive(serde::Deserialize)]
struct XCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn x_auth_callback(Query(q): Query<XCallbackQuery>) -> Response {
    if let Some(err) = q.error {
        return Html(format!("<h2>X Auth Error: {}</h2><a href='/dashboard'>Back</a>", err)).into_response();
    }
    let code = match q.code {
        Some(c) => c,
        None => return Html("<h2>No code received</h2><a href='/dashboard'>Back</a>".to_string()).into_response(),
    };
    // Verify state
    let saved_state = std::fs::read_to_string("/data/x_oauth_state.txt").unwrap_or_default();
    if q.state.as_deref() != Some(saved_state.trim()) {
        return Html("<h2>State mismatch</h2><a href='/dashboard'>Back</a>".to_string()).into_response();
    }
    let verifier = std::fs::read_to_string("/data/x_pkce_verifier.txt").unwrap_or_default();
    let client_id = std::env::var("X_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("X_CLIENT_SECRET").unwrap_or_default();

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("code", code.as_str());
    params.insert("grant_type", "authorization_code");
    params.insert("redirect_uri", "https://yukihamada.jp/api/x/callback");
    params.insert("code_verifier", verifier.trim());

    let resp = client.post("https://api.x.com/2/oauth2/token")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&params)
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            if let (Some(at), Some(rt)) = (body["access_token"].as_str(), body["refresh_token"].as_str()) {
                let expires_in = body["expires_in"].as_i64().unwrap_or(7200);
                let tokens = XTokens {
                    access_token: at.to_string(),
                    refresh_token: rt.to_string(),
                    expires_at: chrono::Utc::now().timestamp() + expires_in,
                };
                save_x_tokens(&tokens);
                Redirect::to("/dashboard?tab=x&msg=connected").into_response()
            } else {
                Html(format!("<h2>Token error ({})</h2><pre>{}</pre><a href='/dashboard'>Back</a>", status, body)).into_response()
            }
        }
        Err(e) => Html(format!("<h2>Request error: {}</h2><a href='/dashboard'>Back</a>", e)).into_response(),
    }
}

// Refresh access token if expired
async fn refresh_x_token() -> Option<String> {
    let mut tokens = load_x_tokens()?;
    if chrono::Utc::now().timestamp() < tokens.expires_at - 60 {
        return Some(tokens.access_token);
    }
    let client_id = std::env::var("X_CLIENT_ID").ok()?;
    let client_secret = std::env::var("X_CLIENT_SECRET").ok()?;
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("grant_type".to_string(), "refresh_token".to_string());
    params.insert("refresh_token".to_string(), tokens.refresh_token.clone());

    let resp = client.post("https://api.x.com/2/oauth2/token")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&params)
        .send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    if let (Some(at), Some(rt)) = (body["access_token"].as_str(), body["refresh_token"].as_str()) {
        tokens.access_token = at.to_string();
        tokens.refresh_token = rt.to_string();
        tokens.expires_at = chrono::Utc::now().timestamp() + body["expires_in"].as_i64().unwrap_or(7200);
        save_x_tokens(&tokens);
        Some(tokens.access_token)
    } else {
        None
    }
}

// OAuth 1.0a signature generation
fn oauth1_sign(method: &str, url: &str, params: &[(String, String)], consumer_secret: &str, token_secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    let mut sorted = params.to_vec();
    sorted.sort();
    let param_str: String = sorted.iter().map(|(k, v)| format!("{}={}", urlenc(k), urlenc(v))).collect::<Vec<_>>().join("&");
    let base = format!("{}&{}&{}", method, urlenc(url), urlenc(&param_str));
    let key = format!("{}&{}", urlenc(consumer_secret), urlenc(token_secret));
    let mut mac = Hmac::<Sha1>::new_from_slice(key.as_bytes()).unwrap();
    hmac::Mac::update(&mut mac, base.as_bytes());
    let result = mac.finalize().into_bytes();
    base64::engine::general_purpose::STANDARD.encode(&result)
}

fn x_is_disabled() -> bool {
    std::path::Path::new("/data/x_disabled").exists()
}

// Publish a tweet using OAuth 1.0a
async fn publish_tweet(text: &str) -> Result<String, String> {
    if x_is_disabled() { return Err("X connection is disabled. Re-enable from dashboard.".to_string()); }
    let consumer_key = std::env::var("X_CONSUMER_KEY").map_err(|_| "X_CONSUMER_KEY not set")?;
    let consumer_secret = std::env::var("X_CONSUMER_SECRET").map_err(|_| "X_CONSUMER_SECRET not set")?;
    let access_token = std::env::var("X_ACCESS_TOKEN").map_err(|_| "X_ACCESS_TOKEN not set")?;
    let access_token_secret = std::env::var("X_ACCESS_TOKEN_SECRET").map_err(|_| "X_ACCESS_TOKEN_SECRET not set")?;

    let url = "https://api.x.com/2/tweets";
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce: String = (0..32).map(|_| format!("{:x}", rand::random::<u8>() % 16)).collect();

    let oauth_params = vec![
        ("oauth_consumer_key".to_string(), consumer_key.clone()),
        ("oauth_nonce".to_string(), nonce.clone()),
        ("oauth_signature_method".to_string(), "HMAC-SHA1".to_string()),
        ("oauth_timestamp".to_string(), timestamp.clone()),
        ("oauth_token".to_string(), access_token.clone()),
        ("oauth_version".to_string(), "1.0".to_string()),
    ];

    let signature = oauth1_sign("POST", url, &oauth_params, &consumer_secret, &access_token_secret);

    let auth_header = format!(
        r#"OAuth oauth_consumer_key="{}",oauth_nonce="{}",oauth_signature="{}",oauth_signature_method="HMAC-SHA1",oauth_timestamp="{}",oauth_token="{}",oauth_version="1.0""#,
        urlenc(&consumer_key), urlenc(&nonce), urlenc(&signature), urlenc(&timestamp), urlenc(&access_token)
    );

    let client = reqwest::Client::new();
    let body = serde_json::json!({ "text": text });
    let resp = client.post(url)
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.unwrap_or_default();
    if status.is_success() {
        Ok(json["data"]["id"].as_str().unwrap_or("").to_string())
    } else {
        Err(format!("X API error {}: {}", status, json))
    }
}

// API: List drafts
#[derive(serde::Deserialize)]
struct XConnectionAction { action: String } // "disconnect" or "reconnect"

async fn x_connection(State(state): State<Arc<AppState>>, headers: HeaderMap, axum::Json(body): axum::Json<XConnectionAction>) -> Response {
    if check_dashboard_auth_state(&state, &headers).is_some() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let _ = std::fs::create_dir_all("/data");
    match body.action.as_str() {
        "disconnect" => {
            let _ = std::fs::write("/data/x_disabled", "1");
            Json(serde_json::json!({"ok": true, "status": "disabled"})).into_response()
        }
        "reconnect" => {
            let _ = std::fs::remove_file("/data/x_disabled");
            Json(serde_json::json!({"ok": true, "status": "enabled"})).into_response()
        }
        _ => Json(serde_json::json!({"error": "invalid action"})).into_response()
    }
}

async fn x_drafts_list(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if check_dashboard_auth_state(&state, &headers).is_some() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let drafts = load_x_drafts();
    Json(drafts).into_response()
}

// API: Add/update drafts
#[derive(serde::Deserialize)]
struct XDraftAction {
    action: String, // "add", "approve", "publish", "delete", "bulk_add"
    id: Option<String>,
    text: Option<String>,
    category: Option<String>,
    drafts: Option<Vec<XDraftInput>>,
}

#[derive(serde::Deserialize)]
struct XDraftInput {
    text: String,
    category: String,
}

async fn x_drafts_action(State(state): State<Arc<AppState>>, headers: HeaderMap, axum::Json(body): axum::Json<XDraftAction>) -> Response {
    if check_dashboard_auth_state(&state, &headers).is_some() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let mut drafts = load_x_drafts();
    match body.action.as_str() {
        "add" => {
            if let Some(text) = body.text {
                let id = format!("{:x}", chrono::Utc::now().timestamp_millis());
                drafts.push(XDraft {
                    id,
                    text,
                    status: "draft".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    published_at: None,
                    tweet_id: None,
                    category: body.category.unwrap_or_else(|| "general".to_string()),
                });
            }
        }
        "bulk_add" => {
            if let Some(inputs) = body.drafts {
                for input in inputs {
                    let id = format!("{:x}{:x}", chrono::Utc::now().timestamp_millis(), rand::random::<u32>());
                    drafts.push(XDraft {
                        id,
                        text: input.text,
                        status: "draft".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        published_at: None,
                        tweet_id: None,
                        category: input.category,
                    });
                }
            }
        }
        "approve" => {
            if let Some(id) = &body.id {
                if let Some(d) = drafts.iter_mut().find(|d| &d.id == id) {
                    d.status = "approved".to_string();
                }
            }
        }
        "publish" => {
            if let Some(id) = &body.id {
                if let Some(d) = drafts.iter_mut().find(|d| &d.id == id) {
                    match publish_tweet(&d.text).await {
                        Ok(tweet_id) => {
                            d.status = "published".to_string();
                            d.published_at = Some(chrono::Utc::now().to_rfc3339());
                            d.tweet_id = Some(tweet_id);
                        }
                        Err(e) => {
                            save_x_drafts(&drafts);
                            return Json(serde_json::json!({"error": e})).into_response();
                        }
                    }
                }
            }
        }
        "delete" => {
            if let Some(id) = &body.id {
                drafts.retain(|d| &d.id != id);
            }
        }
        _ => {}
    }
    save_x_drafts(&drafts);
    Json(serde_json::json!({"ok": true, "count": drafts.len()})).into_response()
}

// X Posts Dashboard UI
async fn x_dashboard(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(resp) = check_dashboard_auth_state(&state, &headers) { return resp; }

    let has_tokens = std::env::var("X_ACCESS_TOKEN").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("X_CONSUMER_KEY").ok().filter(|s| !s.is_empty()).is_some();
    let disabled = x_is_disabled();
    let connected = has_tokens && !disabled;
    let drafts = load_x_drafts();
    let draft_count = drafts.iter().filter(|d| d.status == "draft").count();
    let approved_count = drafts.iter().filter(|d| d.status == "approved").count();
    let published_count = drafts.iter().filter(|d| d.status == "published").count();

    let drafts_html: String = drafts.iter().map(|d| {
        let status_badge = match d.status.as_str() {
            "draft" => "<span style='background:rgba(255,255,255,.1);color:#888;padding:2px 8px;border-radius:4px;font-size:.6rem;'>Draft</span>",
            "approved" => "<span style='background:rgba(59,130,246,.2);color:#3b82f6;padding:2px 8px;border-radius:4px;font-size:.6rem;'>Approved</span>",
            "published" => "<span style='background:rgba(34,197,94,.2);color:#22c55e;padding:2px 8px;border-radius:4px;font-size:.6rem;'>Published</span>",
            _ => "",
        };
        let cat_color = match d.category.as_str() {
            "tech" => "#3b82f6", "anime" => "#E8B64A", "bjj" => "#D4764A",
            "philosophy" => "#a855f7", "product" => "#22c55e", _ => "#888"
        };
        let actions = if d.status == "draft" {
            format!(r#"<div style="display:flex;gap:6px;margin-top:8px;"><button onclick="xAction('approve','{id}')" style="padding:4px 12px;background:rgba(59,130,246,.15);border:1px solid rgba(59,130,246,.3);border-radius:6px;color:#3b82f6;font-size:.65rem;cursor:pointer;">Approve</button><button onclick="xAction('delete','{id}')" style="padding:4px 12px;background:rgba(239,68,68,.1);border:1px solid rgba(239,68,68,.2);border-radius:6px;color:#ef4444;font-size:.65rem;cursor:pointer;">Delete</button></div>"#, id=d.id)
        } else if d.status == "approved" {
            format!(r#"<div style="display:flex;gap:6px;margin-top:8px;"><button onclick="xAction('publish','{id}')" style="padding:4px 12px;background:rgba(232,182,74,.15);border:1px solid rgba(232,182,74,.3);border-radius:6px;color:#E8B64A;font-size:.7rem;font-weight:600;cursor:pointer;">Post Now</button><button onclick="xAction('delete','{id}')" style="padding:4px 12px;background:rgba(239,68,68,.1);border:1px solid rgba(239,68,68,.2);border-radius:6px;color:#ef4444;font-size:.65rem;cursor:pointer;">Delete</button></div>"#, id=d.id)
        } else {
            let tweet_link = d.tweet_id.as_deref().map(|tid| format!(" <a href='https://x.com/yukihamada/status/{}' target='_blank' style='color:#3b82f6;font-size:.6rem;'>View</a>", tid)).unwrap_or_default();
            format!("<div style='margin-top:6px;font-size:.6rem;color:#555;'>{}{}  </div>", d.published_at.as_deref().unwrap_or(""), tweet_link)
        };
        format!(r#"<div style="background:#0a0a0a;border:1px solid #1a1a1a;border-radius:10px;padding:14px;margin-bottom:8px;">
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
  <span style="font-size:.6rem;color:{cat_color};border:1px solid {cat_color};padding:1px 6px;border-radius:4px;">{cat}</span>
  {status}
</div>
<div style="font-size:.8rem;color:#ccc;line-height:1.6;white-space:pre-wrap;">{text}</div>
{actions}
</div>"#, cat_color=cat_color, cat=d.category, status=status_badge, text=d.text, actions=actions)
    }).collect();

    let qr_section = r#"<div class="panel" style="text-align:center;">
<h2>QR Code</h2>
<div id="qr" style="display:inline-block;background:#fff;padding:12px;border-radius:8px;margin:10px 0;"></div>
<div style="font-size:.65rem;color:#666;margin-top:6px;">yukihamada.jp</div>
<script src="https://cdn.jsdelivr.net/npm/qrcode-generator@1.4.4/qrcode.min.js"></script>
<script>var q=qrcode(0,'M');q.addData('https://yukihamada.jp');q.make();document.getElementById('qr').innerHTML=q.createSvgTag(4);</script>
</div>"#;

    let html = format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>X Posts — yukihamada.jp</title>
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}body{{background:#080808;color:#e0e0e0;font-family:system-ui,-apple-system,sans-serif;}}
.wrap{{max-width:900px;margin:0 auto;padding:16px;}}
header{{padding:20px 0 12px;display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:8px;}}
header h1{{font-size:1.2rem;color:#E8B64A;}}header .meta{{font-size:.7rem;color:#666;}}
header a{{color:#E8B64A;text-decoration:none;font-size:.75rem;}}
.cards{{display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:10px;margin:12px 0;}}
.card{{background:#111;border:1px solid #1a1a1a;border-radius:12px;padding:14px;text-align:center;}}
.card .num{{font-size:1.5rem;font-weight:700;color:#E8B64A;}}.card .label{{font-size:.55rem;color:#666;margin-top:2px;text-transform:uppercase;}}
.panel{{background:#111;border:1px solid #1a1a1a;border-radius:12px;padding:14px;margin:12px 0;}}
.panel h2{{font-size:.8rem;color:#E8B64A;margin-bottom:12px;}}
.grid2{{display:grid;grid-template-columns:2fr 1fr;gap:12px;}}
@media(max-width:700px){{.grid2{{grid-template-columns:1fr;}}}}
.tabs{{display:flex;gap:8px;margin-bottom:12px;}}.tabs a{{padding:6px 14px;border-radius:8px;text-decoration:none;font-size:.75rem;color:#888;background:#111;border:1px solid #1a1a1a;}}.tabs a.ac{{color:#E8B64A;border-color:rgba(232,182,74,.3);background:rgba(232,182,74,.08);}}
</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head><body><div class="wrap">
<header>
<div><h1>X Posts Manager</h1><div class="meta">yukihamada.jp</div></div>
<div><a href="/dashboard" style="margin-right:12px;">Analytics</a><a href="/">Site</a></div>
</header>

<div class="tabs">
<a href="/dashboard" >Analytics</a>
<a href="/dashboard/x" class="ac">X Posts</a>
</div>

<div class="cards">
<div class="card"><div class="num">{draft}</div><div class="label">Drafts</div></div>
<div class="card"><div class="num">{approved}</div><div class="label">Approved</div></div>
<div class="card"><div class="num">{published}</div><div class="label">Published</div></div>
<div class="card"><div class="num" style="color:{conn_color}">{conn}</div><div class="label">X Account</div></div>
</div>

{auth_section}

<div class="grid2">
<div class="panel">
<h2>Posts</h2>
<div style="display:flex;gap:6px;margin-bottom:12px;">
<button onclick="filterPosts('all')" class="fbtn active" style="padding:4px 10px;background:rgba(232,182,74,.1);border:1px solid rgba(232,182,74,.2);border-radius:6px;color:#E8B64A;font-size:.65rem;cursor:pointer;">All</button>
<button onclick="filterPosts('draft')" class="fbtn" style="padding:4px 10px;background:#111;border:1px solid #222;border-radius:6px;color:#888;font-size:.65rem;cursor:pointer;">Drafts</button>
<button onclick="filterPosts('approved')" class="fbtn" style="padding:4px 10px;background:#111;border:1px solid #222;border-radius:6px;color:#888;font-size:.65rem;cursor:pointer;">Approved</button>
<button onclick="filterPosts('published')" class="fbtn" style="padding:4px 10px;background:#111;border:1px solid #222;border-radius:6px;color:#888;font-size:.65rem;cursor:pointer;">Published</button>
</div>
<div id="posts-list">{posts}</div>
</div>
<div>
{qr}
<div class="panel">
<h2>Quick Add</h2>
<textarea id="new-post" rows="4" style="width:100%;background:#0a0a0a;border:1px solid #222;border-radius:8px;padding:10px;color:#e0e0e0;font-size:.8rem;resize:vertical;" placeholder="Write a post..."></textarea>
<select id="new-cat" style="width:100%;margin-top:6px;background:#0a0a0a;border:1px solid #222;border-radius:6px;padding:6px;color:#ccc;font-size:.75rem;">
<option value="tech">Tech</option><option value="anime">Anime</option><option value="bjj">BJJ</option><option value="philosophy">Philosophy</option><option value="product">Product</option>
</select>
<div style="display:flex;justify-content:space-between;align-items:center;margin-top:8px;">
<span id="char-count" style="font-size:.65rem;color:#555;">0/280</span>
<button onclick="addPost()" style="padding:6px 16px;background:#E8B64A;color:#080808;border:none;border-radius:6px;font-size:.75rem;font-weight:600;cursor:pointer;">Add Draft</button>
</div>
</div>
</div>
</div>
</div>
<script>
document.getElementById('new-post').addEventListener('input',function(){{document.getElementById('char-count').textContent=this.value.length+'/280';document.getElementById('char-count').style.color=this.value.length>280?'#ef4444':'#555';}});
function xAction(action,id){{
  if(action==='publish'&&!confirm('Post this to X now?'))return;
  if(action==='delete'&&!confirm('Delete this draft?'))return;
  fetch('/api/x/drafts',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{action:action,id:id}})}})
  .then(r=>r.json()).then(d=>{{if(d.error)alert(d.error);else location.reload();}});
}}
function addPost(){{
  var t=document.getElementById('new-post').value.trim();if(!t)return;
  var c=document.getElementById('new-cat').value;
  fetch('/api/x/drafts',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{action:'add',text:t,category:c}})}})
  .then(r=>r.json()).then(()=>location.reload());
}}
function xConn(action){{fetch('/api/x/connection',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{action:action}})}}).then(r=>r.json()).then(()=>location.reload());}}
function filterPosts(f){{document.querySelectorAll('.fbtn').forEach(b=>b.classList.remove('active'));event.target.classList.add('active');
  document.querySelectorAll('#posts-list > div').forEach(d=>{{var s=d.querySelector('span[style*="border-radius:4px"]');if(!s)return;var st=s.textContent.toLowerCase();d.style.display=(f==='all'||st===f)?'':'none';}});}}
</script>
</body></html>"#,
        draft = draft_count,
        approved = approved_count,
        published = published_count,
        conn_color = if connected { "#22c55e" } else { "#ef4444" },
        conn = if connected { "Connected" } else { "Not Connected" },
        auth_section = if disabled {
            r#"<div class="panel" style="text-align:center;"><h2>X Connection Disabled</h2><p style="font-size:.75rem;color:#888;margin-bottom:12px;">X APIとの接続が無効化されています</p><button onclick="xConn('reconnect')" style="padding:10px 24px;background:#22c55e;color:#fff;border:none;border-radius:8px;font-weight:600;cursor:pointer;">Reconnect</button></div>"#
        } else if connected {
            r#"<div class="panel" style="text-align:center;"><h2>X Connected</h2><p style="font-size:.75rem;color:#22c55e;margin-bottom:12px;">@yukihamada — 投稿可能</p><button onclick="if(confirm('X APIとの接続を無効にしますか？'))xConn('disconnect')" style="padding:8px 20px;background:rgba(239,68,68,.15);color:#ef4444;border:1px solid rgba(239,68,68,.3);border-radius:8px;font-size:.75rem;cursor:pointer;">Disconnect</button></div>"#
        } else {
            r#"<div class="panel" style="text-align:center;"><h2>Connect X Account</h2><p style="font-size:.75rem;color:#888;margin-bottom:12px;">X APIと連携して投稿できるようにします</p><a href="/api/x/auth" style="display:inline-block;padding:10px 24px;background:#E8B64A;color:#080808;border-radius:8px;text-decoration:none;font-weight:600;">Connect @yukihamada</a></div>"#
        },
        posts = drafts_html,
        qr = qr_section,
    );

    ([("content-type", "text/html; charset=utf-8")], html).into_response()
}

// ── AI Chat ──

#[derive(serde::Deserialize)]
struct ChatReq {
    messages: Vec<ChatMsg>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    user_token: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

const USER_MEMORY_FILE: &str = "/data/user_memory.json";
const USER_MEMORY_LIMIT: usize = 40;
const MAX_USERS_IN_MEMORY: usize = 5000;

fn load_user_memory() -> HashMap<String, Vec<ChatMsg>> {
    std::fs::read_to_string(USER_MEMORY_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// Atomic write: write to .tmp then rename. Crash-safe.
fn save_user_memory(mem: &HashMap<String, Vec<ChatMsg>>) {
    let Ok(s) = serde_json::to_string(mem) else { return; };
    let tmp = format!("{}.tmp", USER_MEMORY_FILE);
    if std::fs::write(&tmp, s).is_ok() {
        let _ = std::fs::rename(&tmp, USER_MEMORY_FILE);
    }
}

fn sanitize_user_id(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() || raw.len() > 64 { return None; }
    if !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(raw.to_string())
}

// HMAC-sign a user_id with the server's anthropic_key (reused as signing secret)
// or fall back to a derived value. Returns first 32 hex chars of the HMAC-SHA256.
fn sign_user_id(state: &AppState, user_id: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let secret = state.anthropic_key.as_deref()
        .or(state.newsletter_admin_token.as_deref())
        .unwrap_or("yukihamada-jp-fallback-secret-2026");
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes())
        .expect("HMAC can accept any key size");
    mac.update(b"user_memory:");
    mac.update(user_id.as_bytes());
    let bytes = mac.finalize().into_bytes();
    bytes.iter().take(16).map(|b| format!("{:02x}", b)).collect()
}

// Constant-time string comparison.
fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() { return false; }
    let mut diff: u8 = 0;
    for i in 0..a.len() { diff |= a[i] ^ b[i]; }
    diff == 0
}

fn verify_user_token(state: &AppState, user_id: &str, token: &str) -> bool {
    ct_eq(&sign_user_id(state, user_id), token)
}

// Enforce a max number of users in memory (FIFO eviction by oldest entry position).
fn enforce_user_limit(mem: &mut HashMap<String, Vec<ChatMsg>>) {
    if mem.len() <= MAX_USERS_IN_MEMORY { return; }
    // Simple eviction: remove N oldest by sorted key (deterministic but not truly LRU).
    // For a personal site, this avoids unbounded growth under DoS.
    let to_remove = mem.len() - MAX_USERS_IN_MEMORY;
    let keys: Vec<String> = {
        let mut ks: Vec<String> = mem.keys().cloned().collect();
        ks.sort();
        ks.into_iter().take(to_remove).collect()
    };
    for k in keys { mem.remove(&k); }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct ChatMsg {
    role: String,
    content: String,
}

// Returns true if the IP has exceeded the limit (max_reqs within window_secs)
fn rate_limited(store: &Mutex<HashMap<String, Vec<u64>>>, ip: &str, max_reqs: usize, window_secs: u64) -> bool {
    let now = now_secs();
    let mut map = store.lock().unwrap();
    let timestamps = map.entry(ip.to_string()).or_default();
    timestamps.retain(|&t| now - t < window_secs);
    if timestamps.len() >= max_reqs {
        return true;
    }
    timestamps.push(now);
    false
}

// Sanitize incoming messages: enforce max count, max length, proper alternation.
// Conversation must start with a user message — prevents assistant-role injection.
fn sanitize_messages(msgs: Vec<ChatMsg>) -> Vec<ChatMsg> {
    let cleaned: Vec<ChatMsg> = msgs.into_iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| ChatMsg {
            role: m.role,
            content: m.content.chars().take(1000).collect(),
        })
        .collect::<Vec<_>>()
        .into_iter().rev().take(20).rev().collect();

    // Drop leading assistant messages, then enforce strict user→assistant alternation
    let mut result: Vec<ChatMsg> = Vec::new();
    for msg in cleaned.into_iter().skip_while(|m| m.role == "assistant") {
        let last_role = result.last().map(|m| m.role.as_str());
        match (last_role, msg.role.as_str()) {
            (None, "user") | (Some("user"), "assistant") | (Some("assistant"), "user") => {
                result.push(msg);
            }
            (Some("user"), "user") => {
                // Merge consecutive user messages into the last one
                if let Some(last) = result.last_mut() {
                    last.content = format!("{}\n{}", last.content, msg.content)
                        .chars().take(1000).collect();
                }
            }
            _ => {} // Skip injected assistant messages
        }
    }
    result
}

// Append a chat log entry to /data/chat_logs.jsonl
fn log_chat(ip: &str, question: &str, answer: &str) {
    // Char-based truncation to avoid panicking on multi-byte UTF-8 (e.g. Japanese)
    let truncated: String = answer.chars().take(500).collect();
    let entry = serde_json::json!({
        "ts": now_secs(),
        "ip": ip,
        "q": question,
        "a": truncated,
    });
    let line = format!("{}\n", entry);
    let _ = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/data/chat_logs.jsonl")
        .and_then(|mut f| { use std::io::Write; f.write_all(line.as_bytes()) });
}

// Try m5 HITL first, fall back to Anthropic Claude.
async fn run_agentic_chat(
    api_key: &str,
    system: &str,
    tools: &serde_json::Value,
    initial_msgs: Vec<serde_json::Value>,
    state: &Arc<AppState>,
    query: &str,
) -> Result<String, String> {
    run_agentic_chat_with_progress(api_key, system, tools, initial_msgs, state, query, None, None).await
}

async fn run_agentic_chat_with_progress(
    api_key: &str,
    system: &str,
    tools: &serde_json::Value,
    initial_msgs: Vec<serde_json::Value>,
    state: &Arc<AppState>,
    query: &str,
    user_id: Option<String>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String, String> {
    let query_user_id: Option<String> = user_id;
    let progress_tx = progress;
    // ── Try m5 HITL first ──
    let m5_url = state.m5_url.lock().unwrap().clone();
    if let Some(url) = m5_url {
        let rag = rag_context(&state.posts, &query.to_lowercase());
        let history = initial_msgs.iter()
            .rev().skip(1).take(4).rev()
            .filter(|m| m["role"].as_str() == Some("user") || m["role"].as_str() == Some("assistant"))
            .filter_map(|m| {
                let role = m["role"].as_str().unwrap_or("");
                let content = m["content"].as_str()?;
                Some(format!("{}: {}", role, content.chars().take(200).collect::<String>()))
            })
            .collect::<Vec<_>>().join("\n");

        let mut m5_context = String::new();
        m5_context.push_str("# 濱田優貴について\n");
        m5_context.push_str("- Enabler CEO、元メルカリ CPO、元NOT A HOTEL 共同創業者、柔術青帯\n");
        m5_context.push_str("- モットー: 建てて、残して、いいやつと。\n");
        m5_context.push_str("- プロダクト: Soluna(solun.art), JiuFlow(jiuflow.art), Koe Device(koe.live), chatweb.ai, パシャ(pasha.run)\n");
        m5_context.push_str("- 連絡: mail@yukihamada.jp / X: @yukihamada\n\n");
        if !rag.is_empty() {
            m5_context.push_str("# 関連ブログ記事（RAG）\n");
            m5_context.push_str(&rag);
            m5_context.push_str("\n");
        }
        if !history.is_empty() {
            m5_context.push_str("# 直近の会話\n");
            m5_context.push_str(&history);
            m5_context.push_str("\n");
        }

        let http = reqwest::Client::new();
        let m5_base = url.trim_end_matches('/').to_string();
        let body = serde_json::json!({
            "question": query,
            "context": m5_context,
            "site": "yuki",
            "user_id": query_user_id.clone(),
        });

        // Spawn a status poller that forwards "waiting_slow" / "editing" notifications
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        if let Some(tx) = progress_tx.clone() {
            let poll_url = format!("{}/state", m5_base);
            let http2 = http.clone();
            tokio::spawn(async move {
                let mut last = String::new();
                loop {
                    tokio::select! {
                        _ = &mut stop_rx => break,
                        _ = tokio::time::sleep(std::time::Duration::from_millis(1200)) => {}
                    }
                    if let Ok(r) = http2.get(&poll_url).timeout(std::time::Duration::from_secs(3)).send().await {
                        if let Ok(v) = r.json::<serde_json::Value>().await {
                            let status = v["current"]["status"].as_str().unwrap_or("").to_string();
                            if status != last {
                                let msg = match status.as_str() {
                                    "waiting_slow" => Some("濱田が賢いモデルで考え中です…".to_string()),
                                    "editing" => Some("濱田が回答を編集中です…".to_string()),
                                    _ => None,
                                };
                                if let Some(m) = msg { let _ = tx.send(m); }
                                last = status;
                            }
                        }
                    }
                }
            });
        }

        let mut m5_req = http.post(format!("{}/ask", m5_base))
            .timeout(std::time::Duration::from_secs(700))
            .json(&body);
        if let Some(tok) = state.m5_hitl_token.as_deref() {
            m5_req = m5_req.header("Authorization", format!("Bearer {}", tok));
        }
        let m5_result = m5_req.send().await;
        let _ = stop_tx.send(());

        match m5_result {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if let Some(text) = v["text"].as_str() {
                        if !text.trim().is_empty() {
                            return Ok(text.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Fallback: Anthropic Claude with tools ──
    let client = reqwest::Client::new();
    let mut msgs = initial_msgs;

    for _ in 0..3 {
        let resp: serde_json::Value = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .timeout(std::time::Duration::from_secs(30))
            .json(&serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 512,
                "system": system,
                "tools": tools,
                "messages": msgs,
            }))
            .send().await.map_err(|e| e.to_string())?
            .json().await.map_err(|e| e.to_string())?;

        // Surface API-level errors
        if resp.get("type").and_then(|t| t.as_str()) == Some("error") {
            return Err(format!("Anthropic error: {}", resp["error"]["message"].as_str().unwrap_or("unknown")));
        }

        if resp["stop_reason"].as_str() != Some("tool_use") {
            return Ok(resp["content"].as_array()
                .and_then(|c| c.iter().find(|b| b["type"] == "text"))
                .and_then(|b| b["text"].as_str())
                .unwrap_or("すみません、うまく答えられませんでした。")
                .to_string());
        }

        let tool_blocks: Vec<serde_json::Value> = resp["content"].as_array()
            .map(|c| c.iter().filter(|b| b["type"] == "tool_use").cloned().collect())
            .unwrap_or_default();

        let mut tool_results: Vec<serde_json::Value> = Vec::new();
        for tb in &tool_blocks {
            let tool_name = tb["name"].as_str().unwrap_or("");
            let tool_id   = tb["id"].as_str().unwrap_or("tool_0");
            let tool_input = &tb["input"];

            let result_text = match tool_name {
                "search_posts" => {
                    let kw = tool_input["query"].as_str().unwrap_or("").to_lowercase();
                    let ctx = rag_context(&state.posts, &kw);
                    if ctx.is_empty() {
                        let words: Vec<&str> = kw.split(|c: char| !c.is_alphanumeric() && c != 'ー')
                            .filter(|w| w.len() > 1).collect();
                        let found = state.posts.iter()
                            .filter(|p| {
                                let hay = format!("{} {} {}", p.title, p.description, p.tags.join(" ")).to_lowercase();
                                words.iter().any(|w| hay.contains(w))
                            })
                            .take(5)
                            .map(|p| format!("- {} ({}) — {}", p.title, p.date, p.description))
                            .collect::<Vec<_>>().join("\n");
                        if found.is_empty() { format!("「{}」に関する記事は見つかりませんでした", kw) } else { found }
                    } else { ctx }
                },
                "ask_soluna" => {
                    let q = tool_input["question"].as_str().unwrap_or(query).to_string();
                    match reqwest::Client::new()
                        .post("https://solun.art/api/a2a")
                        .timeout(std::time::Duration::from_secs(15))
                        .json(&serde_json::json!({
                            "id": "chat-tool",
                            "message": {"role": "user", "parts": [{"type": "text", "text": q}]}
                        }))
                        .send().await {
                        Ok(r) => r.json::<serde_json::Value>().await.ok()
                            .and_then(|v| v["status"]["message"]["parts"][0]["text"].as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| "Solunaから情報を取得できませんでした".to_string()),
                        Err(_) => "Solunaサーバーに接続できませんでした".to_string(),
                    }
                },
                _ => format!("Unknown tool: {}", tool_name),
            };

            tool_results.push(serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_id,
                "content": result_text
            }));
        }

        msgs.push(serde_json::json!({"role": "assistant", "content": resp["content"]}));
        msgs.push(serde_json::json!({"role": "user", "content": tool_results}));
    }

    Ok("すみません、うまく答えられませんでした。".to_string())
}

fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; out.push(' '); }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

// Extract search terms from a query: ASCII words + Japanese char 2/3-grams.
fn extract_terms(query: &str) -> Vec<String> {
    let lc = query.to_lowercase();
    let words: Vec<String> = lc.split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| w.chars().count() > 1 && w.chars().any(|c| c.is_ascii_alphanumeric()))
        .map(|w| w.to_string())
        .collect();
    let chars: Vec<char> = lc.chars()
        .filter(|c| {
            let b = *c as u32;
            (0x3040..=0x309F).contains(&b) || (0x30A0..=0x30FF).contains(&b) ||
            (0x4E00..=0x9FFF).contains(&b) || c.is_ascii_alphanumeric()
        })
        .collect();
    let mut ngrams: Vec<String> = Vec::new();
    for i in 0..chars.len() {
        if i + 2 <= chars.len() { ngrams.push(chars[i..i+2].iter().collect()); }
        if i + 3 <= chars.len() { ngrams.push(chars[i..i+3].iter().collect()); }
    }
    words.into_iter().chain(ngrams.into_iter()).collect()
}

// TF-IDF style scoring: terms that appear in MANY posts (high doc frequency) are
// down-weighted automatically, so Japanese particles like "です" or "して" stop
// contributing to the score without needing a hardcoded stopword list.
fn rag_context(posts: &[blog::BlogPost], query: &str) -> String {
    let terms = extract_terms(query);
    if terms.is_empty() { return String::new(); }

    // Pre-compute lowercased haystack per post
    let hays: Vec<(String, String, String)> = posts.iter().map(|p| {
        (
            p.title.to_lowercase(),
            p.tags.join(" ").to_lowercase(),
            format!("{} {} {} {}", p.title, p.description, p.tags.join(" "), strip_html(&p.html)).to_lowercase(),
        )
    }).collect();

    // IDF per term: ln(N / (1 + df))
    let n = posts.len() as f64;
    let term_idf: Vec<(String, f64)> = terms.iter().map(|t| {
        let df = hays.iter().filter(|(_, _, full)| full.contains(t.as_str())).count();
        let idf = ((n + 1.0) / (df as f64 + 1.0)).ln().max(0.0);
        (t.clone(), idf)
    }).collect();

    let mut scored: Vec<(f64, &blog::BlogPost)> = posts.iter().enumerate().map(|(i, p)| {
        let (title_lc, tag_lc, full) = &hays[i];
        let mut score = 0.0_f64;
        for (term, idf) in &term_idf {
            if *idf < 0.1 { continue; } // universal terms get ignored
            if full.contains(term.as_str())  { score += idf; }
            if title_lc.contains(term.as_str()) { score += idf * 3.0; }
            if tag_lc.contains(term.as_str())   { score += idf * 2.0; }
        }
        (score, p)
    }).collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.iter().filter(|(s, _)| *s > 0.0).take(3)
        .map(|(_, p)| {
            let text = strip_html(&p.html);
            let preview: String = text.chars().take(900).collect();
            format!("### {} ({})\n{}\n{}\n", p.title, p.date, p.description, preview)
        })
        .collect::<Vec<_>>().join("\n")
}

// ── m5 HITL registration ──

#[derive(serde::Deserialize)]
struct M5RegisterReq { url: String }

async fn m5_register_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<M5RegisterReq>,
) -> impl IntoResponse {
    let token = match &state.m5_register_token {
        Some(t) => t.clone(),
        None => return (StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "m5 registration disabled"}))).into_response(),
    };
    let provided = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if provided != token.as_str() {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let url = req.url.trim().to_string();
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return (StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid url"}))).into_response();
    }
    *state.m5_url.lock().unwrap() = Some(url.clone());
    println!("[m5] registered URL: {}", url);
    Json(serde_json::json!({"ok": true, "url": url})).into_response()
}

async fn m5_status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let url = state.m5_url.lock().unwrap().clone();
    match url {
        Some(u) => Json(serde_json::json!({"registered": true, "url": u})),
        None => Json(serde_json::json!({"registered": false})),
    }
}

// ── User memory management ──
// Auth: require BOTH X-User-ID and X-User-Token headers (HMAC-verified).
// user_id never appears in URL query strings (would leak into access logs).

fn verify_user_auth(state: &AppState, headers: &HeaderMap) -> Result<String, (StatusCode, &'static str)> {
    let uid = headers.get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| sanitize_user_id(Some(v)))
        .ok_or((StatusCode::BAD_REQUEST, "X-User-ID header required"))?;
    let tok = headers.get("x-user-token")
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "X-User-Token header required"))?;
    if !verify_user_token(state, &uid, tok) {
        return Err((StatusCode::UNAUTHORIZED, "invalid token"));
    }
    Ok(uid)
}

async fn user_memory_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Rate limit memory reads
    let ip = extract_ip(&headers);
    if rate_limited(&state.chat_rate_limit, &ip, 30, 60) {
        return (StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "rate limit"}))).into_response();
    }
    let uid = match verify_user_auth(&state, &headers) {
        Ok(u) => u,
        Err((s, m)) => return (s, Json(serde_json::json!({"error": m}))).into_response(),
    };
    let msgs = state.user_memory.lock().unwrap().get(&uid).cloned().unwrap_or_default();
    Json(serde_json::json!({"count": msgs.len(), "messages": msgs})).into_response()
}

async fn user_memory_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    if rate_limited(&state.chat_rate_limit, &ip, 30, 60) {
        return (StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "rate limit"}))).into_response();
    }
    let uid = match verify_user_auth(&state, &headers) {
        Ok(u) => u,
        Err((s, m)) => return (s, Json(serde_json::json!({"error": m}))).into_response(),
    };
    let mut mem = state.user_memory.lock().unwrap();
    let existed = mem.remove(&uid).is_some();
    save_user_memory(&mem);
    Json(serde_json::json!({"ok": true, "deleted": existed})).into_response()
}

fn make_chat_system(all_titles: &str, relevant: &str) -> String {
    format!(
        "あなたは濱田優貴（Yuki Hamada）のパーソナルサイト yukihamada.jp のAIアシスタントです。\
        訪問者の質問に、濱田優貴の言葉として自然な日本語で簡潔に答えてください。\
        英語で質問されたら英語で答えてください。\
        マークダウン記法は使わず、普通のテキストで答えてください。\
        Solunaやフェスに関しては ask_soluna ツールを使って委譲してください。\
        ブログ記事を探すときは search_posts ツールを使ってください。\
        \n\n# 濱田優貴について\
        \n- Enabler（イネブラ）代表取締役CEO\
        \n- 元メルカリ 取締役 CPO/CINO（2014〜2021）\
        \n- 元NOT A HOTEL 共同創業者（2018〜2024）\
        \n- 柔術青帯\
        \n- 「建てて、残して、いいやつと。」がモットー\
        \n- Rust・Swift・ESP32を自分で書く実装型CEO\
        \n\n# 主なプロジェクト\
        \n- Soluna: 北海道弟子屈の宿泊空間 + SOLUNA FEST HAWAII → solun.art\
        \n- JiuFlow: 柔術テクニックマッピングアプリ → jiuflow.art\
        \n- Koe Device: 群衆を楽器にするデバイス → koe.live\
        \n- chatweb.ai: マルチモデルAIチャット\
        \n- パシャ: AI OCR経費管理 → pasha.run\
        \n\n# 連絡先\
        \n- メール: mail@yukihamada.jp（返信率100%、24時間以内）\
        \n- X: @yukihamada\
        \n\n# 仕事・コラボ依頼が来たときの対応スタイル\
        \n仕事の相談には「ユーモアを交えつつ、真剣に向き合う」スタンスで答えること。\
        \n重要: 最後は必ず mail@yukihamada.jp へのメールを案内すること。\
        \n\n依頼種別ごとの反応方針:\
        \n【登壇・講演依頼】\
        \n  好き。基本受ける。「喜んで！どんな場ですか？」と前のめりに。\
        \n  ユーモア例: 「登壇はRustより得意かもしれない（ぎりぎり）」\
        \n\n【顧問・アドバイザー依頼】\
        \n  プロダクト戦略・AI・スマートホーム系なら前向き。週1〜2時間程度が現実的。\
        \n  ユーモア例: 「Rustしか書けなくなった人間でよければ」\
        \n\n【開発・技術コンサル依頼】\
        \n  Rust/Swift/ESP32/AIエージェント案件は話を聞く。受託開発はしないが相談は乗る。\
        \n  ユーモア例: 「TypeScriptは書けません（書きません）」\
        \n\n【投資・出資相談】\
        \n  Enablerとして検討。特に柔術・音楽・不動産・AIの交点が好き。\
        \n  ユーモア例: 「青帯が出資するやつ、世界広しといえど多くない」\
        \n\n【採用・スカウト（自分への】\
        \n  今は自分のプロダクトに全集中。ただ面白い話なら聞く。\
        \n  ユーモア例: 「今11個プロダクト運営中なので、そっちを手伝ってくれる人を探してます（逆オファー）」\
        \n\n【メディア取材・インタビュー】\
        \n  基本OK。プロダクトや技術の話が好き。\
        \n\n# ブログ記事一覧\n{}\
        \n\n# 関連記事（RAG）\n{}",
        all_titles, relevant
    )
}

fn make_chat_tools() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "search_posts",
            "description": "ブログ記事をキーワード検索する",
            "input_schema": {
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }
        },
        {
            "name": "ask_soluna",
            "description": "Soluna FEST HAWAIIやsolun.artのリゾートに関する質問をSoluna AIに委譲する",
            "input_schema": {
                "type": "object",
                "properties": {"question": {"type": "string"}},
                "required": ["question"]
            }
        }
    ])
}

fn extract_ip(headers: &HeaderMap) -> String {
    headers.get("x-forwarded-for")
        .or_else(|| headers.get("fly-client-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',').next().unwrap_or("unknown")
        .trim().to_string()
}

async fn chat_page() -> impl IntoResponse {
    Redirect::to("/#chat").into_response()
}


async fn chat_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ChatReq>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    // Validate eagerly before spawning the stream
    let ip = extract_ip(&headers);
    // Capture session_id early (sanitize: alphanumeric + hyphen, max 64 chars)
    let session_id: String = req.session_id.as_deref()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();

    let raw_uid = sanitize_user_id(req.user_id.as_deref());
    // If client sent a user_id, they can only use it IF:
    // (a) no memory exists for it yet (first-time use) → server trusts and issues token
    // (b) they also sent a valid HMAC token → verified continuation
    // Otherwise, we reject the user_id (attacker probing with someone else's ID).
    let (user_id, issued_token) = match &raw_uid {
        Some(uid) => {
            let existing = state.user_memory.lock().unwrap().contains_key(uid);
            match (req.user_token.as_deref(), existing) {
                (Some(tok), _) if verify_user_token(&state, uid, tok) => {
                    (Some(uid.clone()), None)
                }
                (_, false) => {
                    // First time — bless the ID and return a token
                    let token = sign_user_id(&state, uid);
                    (Some(uid.clone()), Some(token))
                }
                _ => {
                    // user_id claim without valid token for existing memory → reject
                    (None, None)
                }
            }
        }
        None => (None, None),
    };

    // Validate + build context synchronously (errors → single event stream)
    let prepared: Result<(String, String, serde_json::Value, Vec<serde_json::Value>, String, Option<String>), String> = (|| {
        if rate_limited(&state.chat_rate_limit, &ip, 15, 60) {
            return Err("リクエストが多すぎます。少し待ってからお試しください。".into());
        }
        if req.messages.is_empty() { return Err("メッセージを入力してください".into()); }
        let messages = sanitize_messages(req.messages);
        if !messages.iter().any(|m| m.role == "user") {
            return Err("ユーザーメッセージが必要です".into());
        }
        let api_key = state.anthropic_key.clone().ok_or_else(|| "AI not configured".to_string())?;
        let query = messages.iter().rev()
            .find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
        let all_titles = state.posts.iter()
            .map(|p| format!("- {} ({}) — {}", p.title, p.date, p.description))
            .collect::<Vec<_>>().join("\n");
        let relevant  = rag_context(&state.posts, &query.to_lowercase());

        // Build memory context from this user's past conversations
        let user_memory_summary = if let Some(uid) = &user_id {
            let mem = state.user_memory.lock().unwrap();
            mem.get(uid).map(|past| {
                // Take last 10 turns, truncate each to 150 chars
                past.iter().rev().take(10).rev()
                    .map(|m| format!("{}: {}",
                        m.role,
                        m.content.chars().take(150).collect::<String>()))
                    .collect::<Vec<_>>().join("\n")
            }).unwrap_or_default()
        } else {
            String::new()
        };

        let system = if user_memory_summary.is_empty() {
            make_chat_system(&all_titles, &relevant)
        } else {
            format!("{}\n\n# このユーザーとの過去の会話\n{}\n\n上記を踏まえてパーソナライズされた応答をしてください。",
                make_chat_system(&all_titles, &relevant), user_memory_summary)
        };
        let tools = make_chat_tools();
        let init_msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        Ok((api_key, system, tools, init_msgs, query, user_id))
    })();

    // Build the streaming response. Emits:
    //   - user_token (first event, if issued)
    //   - waiting events (from progress channel) while m5 is in waiting_slow/editing
    //   - delta events (final text word by word)
    //   - done event
    let state_clone = state.clone();
    let ip_clone = ip.clone();
    let stream = async_stream::stream! {
        // Emit issued token first (if any) so client can persist it
        if let Some(tok) = &issued_token {
            yield Ok::<_, Infallible>(Event::default()
                .data(serde_json::json!({"user_token": tok}).to_string()));
        }

        let (api_key, system, tools, init_msgs, query, user_id) = match prepared {
            Err(e) => {
                yield Ok::<_, Infallible>(Event::default()
                    .data(serde_json::json!({"error": e, "done": true}).to_string()));
                return;
            }
            Ok(p) => p,
        };

        // 🔔 Notify admin: Telegram + SSE broadcast (fire-and-forget)
        {
            let notify_payload = serde_json::json!({
                "session_id": session_id,
                "message": query.chars().take(300).collect::<String>(),
                "ts": now_secs(),
            }).to_string();
            let _ = state_clone.chat_notify_tx.send(notify_payload);

            if let Some(tok) = state_clone.telegram_token.clone() {
                let q = query.chars().take(280).collect::<String>();
                let sid = session_id.clone();
                tokio::spawn(async move {
                    let text = format!("💬 yukihamada.jp #chat\n\n{}\n\n🔖 {}", q, sid);
                    let url = format!("https://api.telegram.org/bot{}/sendMessage", tok);
                    let _ = reqwest::Client::new()
                        .post(&url)
                        .json(&serde_json::json!({"chat_id": 1136442501_i64, "text": text}))
                        .timeout(std::time::Duration::from_secs(5))
                        .send().await;
                });
            }
        }

        // ── Owner live-chat mode ──
        // If Yuki is online (heartbeat within 90s), wait for his direct reply
        // instead of running the LLM.
        let owner_online = {
            let last = state_clone.owner_last_seen.load(Ordering::Relaxed);
            last > 0 && (now_secs() - last) < 90
        };
        if owner_online {
            let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            state_clone.pending_live_chats.lock().unwrap()
                .insert(session_id.clone(), reply_tx);

            yield Ok::<_, Infallible>(Event::default()
                .data(serde_json::json!({"waiting": "濱田本人がオンラインです。少々お待ちください…"}).to_string()));

            // Wait up to 3 minutes for owner reply
            let owner_reply = tokio::time::timeout(
                tokio::time::Duration::from_secs(180),
                reply_rx.recv()
            ).await;

            // Clean up if timed out
            state_clone.pending_live_chats.lock().unwrap().remove(&session_id);

            let is_live_reply = matches!(owner_reply, Ok(Some(_)));
            let text = match owner_reply {
                Ok(Some(r)) => r,
                _ => {
                    // Timed out — fall through to LLM below by yielding a fallback
                    yield Ok::<_, Infallible>(Event::default()
                        .data(serde_json::json!({"waiting": "AIが代わりにお答えします…"}).to_string()));
                    // Run LLM as fallback
                    run_agentic_chat_with_progress(
                        &api_key, &system, &tools, init_msgs.clone(), &state_clone,
                        &query, user_id.clone(), None
                    ).await.unwrap_or_else(|e| format!("エラー: {}", e))
                }
            };

            let header = if is_live_reply {
                "<span style=\"font-size:10px;color:#5eead4;display:block;margin-bottom:4px;\">✍️ 本人より</span>"
            } else { "" };
            yield Ok::<_, Infallible>(Event::default()
                .data(serde_json::json!({"delta": format!("{}{}", header, text), "done": true}).to_string()));
            return;
        }

        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let uid_for_work = user_id.clone();
        let query_for_work = query.clone();
        let mut work = Box::pin(run_agentic_chat_with_progress(
            &api_key, &system, &tools, init_msgs, &state_clone,
            &query_for_work, uid_for_work, Some(progress_tx)
        ));

        let text: String;
        loop {
            tokio::select! {
                Some(msg) = progress_rx.recv() => {
                    yield Ok::<_, Infallible>(Event::default()
                        .data(serde_json::json!({"waiting": msg}).to_string()));
                }
                res = &mut work => {
                    text = res.unwrap_or_else(|e| format!("エラー: {}", e));
                    break;
                }
            }
        }

        // Persist memory + log
        if let Some(uid) = &user_id {
            let mut mem = state_clone.user_memory.lock().unwrap();
            let entry = mem.entry(uid.clone()).or_default();
            entry.push(ChatMsg { role: "user".to_string(), content: query.clone() });
            entry.push(ChatMsg { role: "assistant".to_string(), content: text.clone() });
            if entry.len() > USER_MEMORY_LIMIT {
                let drop = entry.len() - USER_MEMORY_LIMIT;
                entry.drain(..drop);
            }
            enforce_user_limit(&mut mem);
            save_user_memory(&mem);
        }
        log_chat(&ip_clone, &query, &text);

        // Stream final text word by word
        let words: Vec<&str> = text.split_inclusive(|c: char|
            c.is_whitespace() || c == '。' || c == '、' || c == '.' || c == '\n'
        ).collect();
        for (i, chunk) in words.iter().enumerate() {
            let is_last = i == words.len() - 1;
            yield Ok::<_, Infallible>(Event::default()
                .data(serde_json::json!({"delta": chunk, "done": is_last}).to_string()));
            if !is_last {
                tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
            }
        }
        yield Ok::<_, Infallible>(Event::default()
            .data(serde_json::json!({"done": true}).to_string()));
    };
    Sse::new(stream)
}

// ── Admin chat notification stream ──

async fn admin_chat_stream(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let mut rx = state.chat_notify_tx.subscribe();
    let stream = async_stream::stream! {
        yield Ok::<_, Infallible>(Event::default()
            .data(serde_json::json!({"ping": true}).to_string()));
        loop {
            match rx.recv().await {
                Ok(msg) => yield Ok::<_, Infallible>(Event::default().data(msg)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).into_response()
}

#[derive(serde::Deserialize)]
struct AdminReplyReq {
    session_id: String,
    text: String,
}

async fn admin_chat_reply(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    Json(body): Json<AdminReplyReq>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    if body.session_id.is_empty() || body.text.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "session_id and text required"}))).into_response();
    }
    let text: String = body.text.chars().take(2000).collect();
    // Wake up live-chat SSE if session is waiting
    let live_tx = state.pending_live_chats.lock().unwrap().remove(&body.session_id);
    if let Some(tx) = live_tx {
        let _ = tx.send(text.clone());
    }
    state.pending_admin_replies.lock().unwrap()
        .insert(body.session_id.clone(), text);
    Json(serde_json::json!({"ok": true})).into_response()
}

async fn poll_admin_reply(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let session_id = params.get("session").cloned().unwrap_or_default();
    if session_id.is_empty() {
        return Json(serde_json::json!({"reply": null})).into_response();
    }
    let reply = state.pending_admin_replies.lock().unwrap().remove(&session_id);
    Json(serde_json::json!({"reply": reply})).into_response()
}

// ── Owner presence heartbeat ──

async fn owner_heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return (StatusCode::UNAUTHORIZED, cors_headers(),
            Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    state.owner_last_seen.store(now_secs(), Ordering::Relaxed);
    (cors_headers(), Json(serde_json::json!({"ok": true}))).into_response()
}

async fn owner_online_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let last = state.owner_last_seen.load(Ordering::Relaxed);
    let online = last > 0 && (now_secs() - last) < 90;
    Json(serde_json::json!({"online": online}))
}

async fn transcribe_audio(
    State(state): State<Arc<AppState>>,
    mut multipart: axum::extract::Multipart,
) -> impl IntoResponse {
    // Collect audio from multipart
    let mut audio_data: Option<(Vec<u8>, String)> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "audio" {
            let filename = field.file_name().unwrap_or("audio.webm").to_string();
            if let Ok(data) = field.bytes().await {
                if data.len() < 20_000_000 { // 20MB limit
                    audio_data = Some((data.to_vec(), filename));
                }
            }
        }
    }

    let (audio_bytes, filename) = match audio_data {
        Some(d) => d,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "no audio"}))).into_response(),
    };

    let mime = if filename.ends_with(".mp4") || filename.ends_with(".m4a") { "audio/mp4" }
               else if filename.ends_with(".ogg") { "audio/ogg" }
               else if filename.ends_with(".wav") { "audio/wav" }
               else { "audio/webm" };

    let http = reqwest::Client::new();

    // ── Try m5 first (local Whisper, faster) ──
    let m5_url = state.m5_url.lock().unwrap().clone();
    if let Some(base) = m5_url {
        let form = reqwest::multipart::Form::new()
            .part("audio", reqwest::multipart::Part::bytes(audio_bytes.clone())
                .file_name(filename.clone())
                .mime_str(mime).unwrap_or_else(|_| reqwest::multipart::Part::bytes(vec![])));

        match http.post(format!("{}/transcribe", base.trim_end_matches('/')))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send().await
        {
            Ok(r) if r.status().is_success() => {
                if let Ok(v) = r.json::<serde_json::Value>().await {
                    let text = v["text"].as_str().unwrap_or("").to_string();
                    if !text.is_empty() {
                        return Json(serde_json::json!({"text": text, "source": "m5"})).into_response();
                    }
                }
            }
            _ => {} // fall through to Groq
        }
    }

    // ── Fallback: Groq Whisper ──
    let Some(groq_key) = state.groq_api_key.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "transcription not available (m5 offline, Groq not configured)"}))).into_response();
    };

    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(audio_bytes)
            .file_name(filename)
            .mime_str(mime).unwrap_or_else(|_| reqwest::multipart::Part::bytes(vec![])))
        .text("model", "whisper-large-v3")
        .text("response_format", "text");

    match http.post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", groq_key))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(30))
        .send().await
    {
        Ok(r) => {
            let status = r.status();
            let body_text = r.text().await.unwrap_or_default();
            if status.is_success() {
                Json(serde_json::json!({"text": body_text.trim(), "source": "groq"})).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": body_text}))).into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ── AI reply suggestion for admin ──

#[derive(serde::Deserialize)]
struct AiSuggestReq {
    visitor_message: String,
    transcript: Option<String>,
}

async fn admin_ai_suggest(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    Json(body): Json<AiSuggestReq>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let Some(api_key) = state.anthropic_key.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "ai not configured"}))).into_response();
    };

    let system = "あなたは濱田優貴（yukihamada.jp）のAIアシスタントです。\
        訪問者からの質問に、濱田優貴本人として自然な日本語で返信を書いてください。\
        短く、親しみやすく、具体的に。マークダウン不使用。";

    let prompt = if let Some(t) = &body.transcript {
        format!("訪問者のメッセージ:\n{}\n\n私（ユキ）のボイスメモ文字起こし:\n{}\n\n上記を踏まえて返信文を生成してください。", body.visitor_message, t)
    } else {
        format!("訪問者のメッセージ:\n{}\n\n私（ユキ）からの返信を書いてください。", body.visitor_message)
    };

    let payload = serde_json::json!({
        "model": "claude-opus-4-5",
        "max_tokens": 500,
        "system": system,
        "messages": [{"role": "user", "content": prompt}]
    });

    match reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(20))
        .send().await
    {
        Ok(r) => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let text = body["content"][0]["text"].as_str().unwrap_or("").to_string();
            Json(serde_json::json!({"text": text})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ── MCP Server (Model Context Protocol) ──

#[derive(serde::Deserialize)]
struct McpReq {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

fn mcp_text(text: impl Into<String>) -> serde_json::Value {
    serde_json::json!({"content": [{"type": "text", "text": text.into()}]})
}

fn jsonrpc_ok(id: serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn jsonrpc_err(id: serde_json::Value, code: i32, msg: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}})
}

async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let cors_headers = [
        (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        (header::ACCESS_CONTROL_ALLOW_HEADERS, "content-type, authorization"),
    ];

    // Optional API key authentication
    if let Some(required_key) = &state.mcp_key {
        let provided = headers.get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if provided != required_key.as_str() {
            return (StatusCode::UNAUTHORIZED, cors_headers,
                axum::Json(serde_json::json!({"error": "Unauthorized", "hint": "Provide Authorization: Bearer <MCP_API_KEY>"}))).into_response();
        }
    }

    // Rate limit: 30 requests per minute per IP
    let ip = headers.get("x-forwarded-for")
        .or_else(|| headers.get("fly-client-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',').next().unwrap_or("unknown")
        .trim()
        .to_string();
    if rate_limited(&state.mcp_rate_limit, &ip, 30, 60) {
        return (StatusCode::TOO_MANY_REQUESTS, cors_headers,
            axum::Json(serde_json::json!({"error": "Rate limit exceeded"}))).into_response();
    }

    // Body size limit: 64KB
    if body.len() > 65536 {
        return (StatusCode::PAYLOAD_TOO_LARGE, cors_headers,
            axum::Json(serde_json::json!({"error": "Request too large"}))).into_response();
    }

    let val: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, cors_headers,
            axum::Json(serde_json::json!({"error": "invalid json"}))).into_response(),
    };

    // Batch requests
    if val.is_array() {
        let reqs: Vec<McpReq> = match serde_json::from_value(val) {
            Ok(v) => v, Err(_) => return (StatusCode::BAD_REQUEST).into_response(),
        };
        let mut results = Vec::new();
        for req in reqs {
            results.push(handle_mcp_req(&state, req, &headers).await);
        }
        return (cors_headers, axum::Json(serde_json::Value::Array(results))).into_response();
    }

    let req: McpReq = match serde_json::from_value(val) {
        Ok(v) => v, Err(_) => return (StatusCode::BAD_REQUEST).into_response(),
    };
    let result = handle_mcp_req(&state, req, &headers).await;
    (cors_headers, axum::Json(result)).into_response()
}

async fn handle_mcp_req(state: &Arc<AppState>, req: McpReq, _headers: &HeaderMap) -> serde_json::Value {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);
    let params = req.params.unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "initialize" => jsonrpc_ok(id, serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "yukihamada-jp", "version": "1.0.0"}
        })),
        "notifications/initialized" | "ping" => jsonrpc_ok(id, serde_json::json!({})),

        "tools/list" => jsonrpc_ok(id, serde_json::json!({
            "tools": [
                {
                    "name": "list_posts",
                    "description": "List all blog posts with title, date, description, slug, and tags.",
                    "inputSchema": {"type": "object", "properties": {}}
                },
                {
                    "name": "search_posts",
                    "description": "Search Yuki Hamada's blog posts by keyword (Japanese or English).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Search keyword"}
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "get_post",
                    "description": "Get the full text content of a specific blog post by its slug.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "slug": {"type": "string", "description": "Post slug, e.g. '2026-04-22-why-i-started-coding-again'"}
                        },
                        "required": ["slug"]
                    }
                },
                {
                    "name": "get_profile",
                    "description": "Get Yuki Hamada's profile, career history, current projects, and contact information.",
                    "inputSchema": {"type": "object", "properties": {}}
                },
                {
                    "name": "ask_yuki",
                    "description": "Ask Yuki Hamada's AI a question. RAG over blog posts is applied automatically. Soluna festival questions are delegated to solun.art.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "question": {"type": "string", "description": "Your question"}
                        },
                        "required": ["question"]
                    }
                },
                {
                    "name": "ask_soluna",
                    "description": "Ask about SOLUNA FEST HAWAII 2026 directly. Proxied to solun.art A2A agent.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "question": {"type": "string", "description": "Your question about Soluna festival"}
                        },
                        "required": ["question"]
                    }
                }
            ]
        })),

        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("").to_string();
            let args = &params["arguments"];
            match mcp_call(state, &name, args).await {
                Ok(v) => jsonrpc_ok(id, v),
                Err(msg) => jsonrpc_err(id, -32602, &msg),
            }
        },

        other => jsonrpc_err(id, -32601, &format!("Method not found: {}", other)),
    }
}

async fn mcp_call(state: &Arc<AppState>, name: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    match name {
        "list_posts" => {
            let text = state.posts.iter()
                .map(|p| format!("slug: {}\ntitle: {}\ndate: {}\ntags: {}\ndescription: {}\n",
                    p.slug, p.title, p.date, p.tags.join(", "), p.description))
                .collect::<Vec<_>>().join("\n");
            Ok(mcp_text(text))
        },
        "search_posts" => {
            let query = args["query"].as_str().unwrap_or("");
            let terms = extract_terms(query);
            if terms.is_empty() {
                return Ok(mcp_text("No posts found."));
            }
            let hays: Vec<(String, String, String)> = state.posts.iter().map(|p| (
                p.title.to_lowercase(),
                p.tags.join(" ").to_lowercase(),
                format!("{} {} {} {}", p.title, p.description, p.tags.join(" "), strip_html(&p.html)).to_lowercase(),
            )).collect();
            let n = state.posts.len() as f64;
            let term_idf: Vec<(String, f64)> = terms.iter().map(|t| {
                let df = hays.iter().filter(|(_, _, full)| full.contains(t.as_str())).count();
                let idf = ((n + 1.0) / (df as f64 + 1.0)).ln().max(0.0);
                (t.clone(), idf)
            }).collect();
            let mut scored: Vec<(f64, &blog::BlogPost)> = state.posts.iter().enumerate().map(|(i, p)| {
                let (title_lc, tag_lc, full) = &hays[i];
                let mut s = 0.0_f64;
                for (term, idf) in &term_idf {
                    if *idf < 0.1 { continue; }
                    if full.contains(term.as_str())     { s += idf; }
                    if title_lc.contains(term.as_str()) { s += idf * 3.0; }
                    if tag_lc.contains(term.as_str())   { s += idf * 2.0; }
                }
                (s, p)
            }).collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let hits: Vec<_> = scored.iter().filter(|(s,_)| *s > 0.0).take(10).collect();
            if hits.is_empty() {
                Ok(mcp_text("No posts found."))
            } else {
                let text = hits.iter().map(|(_, p)|
                    format!("### {}\nslug: {} | date: {} | tags: {}\n{}\n",
                        p.title, p.slug, p.date, p.tags.join(", "), p.description)
                ).collect::<Vec<_>>().join("\n");
                Ok(mcp_text(text))
            }
        },
        "get_post" => {
            let slug = args["slug"].as_str().unwrap_or("");
            match state.posts.iter().find(|p| p.slug == slug) {
                None => Err(format!("Post '{}' not found.", slug)),
                Some(p) => {
                    let text = format!("# {}\nDate: {} | Tags: {}\nDescription: {}\n\n{}",
                        p.title, p.date, p.tags.join(", "), p.description, strip_html(&p.html));
                    Ok(mcp_text(text))
                }
            }
        },
        "get_profile" => Ok(mcp_text(
            "# 濱田優貴 (Yuki Hamada)\n\n\
            ## Career\n\
            - Enabler（イネブラ）代表取締役CEO (2024〜)\n\
            - 令和トラベル 社外取締役 (2024〜)\n\
            - NOT A HOTEL 共同創業者・元取締役 (2018〜2024)\n\
            - メルカリ 取締役 CPO/CINO (2014〜2021)\n\
            - サイブリッジ 共同創業者 (2003〜2013)\n\
            - BJJ Blue Belt | Motto: 建てて、残して、いいやつと。\n\n\
            ## Projects\n\
            - Soluna: 北海道弟子屈の宿泊空間 → https://solun.art\n\
            - JiuFlow: 柔術テクニックマッピング → https://jiuflow.art\n\
            - Koe Device: 群衆を楽器にするデバイス → https://koe.live\n\
            - chatweb.ai: マルチモデルAIチャット → https://chatweb.ai\n\
            - パシャ: AI OCR経費管理 → https://pasha.run\n\n\
            ## Contact\n\
            - Email: mail@yukihamada.jp\n\
            - X: @yukihamada\n\
            - GitHub: yukihamada\n\
            - Soluna booking: https://solun.art"
        )),
        "ask_yuki" => {
            let question = args["question"].as_str().unwrap_or("").to_string();
            let api_key = state.anthropic_key.clone()
                .ok_or_else(|| "AI not configured".to_string())?;
            let all_titles = state.posts.iter()
                .map(|p| format!("- {} ({}) — {}", p.title, p.date, p.description))
                .collect::<Vec<_>>().join("\n");
            let relevant = rag_context(&state.posts, &question.to_lowercase());
            let system   = make_chat_system(&all_titles, &relevant);
            let tools    = make_chat_tools();
            let init_msgs = vec![serde_json::json!({"role": "user", "content": question.clone()})];
            let text = run_agentic_chat(&api_key, &system, &tools, init_msgs, state, &question).await?;
            Ok(mcp_text(text))
        },
        "ask_soluna" => {
            let question = args["question"].as_str().unwrap_or("").to_string();
            let client = reqwest::Client::new();
            let resp = client.post("https://solun.art/api/a2a")
                .timeout(std::time::Duration::from_secs(20))
                .json(&serde_json::json!({
                    "id": "yukihamada-mcp",
                    "message": {"role": "user", "parts": [{"type": "text", "text": question}]}
                }))
                .send().await.map_err(|e| e.to_string())?;
            let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let text = v["status"]["message"]["parts"][0]["text"].as_str().unwrap_or("").to_string();
            Ok(mcp_text(text))
        },
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// ── A2A (Agent-to-Agent Protocol) ──

async fn a2a_agent_card() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        axum::Json(serde_json::json!({
            "name": "Yuki Hamada",
            "description": "AI assistant for Yuki Hamada (yukihamada.jp). Searches blog posts, answers questions about Soluna, JiuFlow, Koe Device, and other projects.",
            "url": "https://yukihamada.jp/a2a",
            "version": "1.0.0",
            "provider": {
                "organization": "Enabler Inc.",
                "url": "https://yukihamada.jp"
            },
            "capabilities": {
                "streaming": false,
                "pushNotifications": false,
                "stateTransitionHistory": false
            },
            "defaultInputModes": ["text/plain"],
            "defaultOutputModes": ["text/plain"],
            "skills": [
                {
                    "id": "search_blog",
                    "name": "Search Blog Posts",
                    "description": "Search Yuki Hamada's blog posts by keyword",
                    "tags": ["blog", "search"],
                    "examples": ["Find posts about AI", "柔術についての記事を探して"]
                },
                {
                    "id": "ask_about_projects",
                    "name": "Project Information",
                    "description": "Get information about Soluna, JiuFlow, Koe Device, chatweb.ai, and other projects",
                    "tags": ["projects", "soluna", "jiuflow", "koe"],
                    "examples": ["What is Soluna?", "Tell me about JiuFlow"]
                },
                {
                    "id": "contact",
                    "name": "Contact & Booking",
                    "description": "Get contact info, Soluna booking link, and inquiry guidance",
                    "tags": ["contact", "booking", "invest", "hire"],
                    "examples": ["How do I book Soluna?", "Investment inquiry"]
                }
            ]
        }))
    ).into_response()
}

#[derive(serde::Deserialize)]
struct A2ATaskReq {
    id: String,
    message: serde_json::Value,
}

async fn a2a_tasks_handler(
    State(state): State<Arc<AppState>>,
    Json(task): Json<A2ATaskReq>,
) -> impl IntoResponse {
    let question = task.message["parts"]
        .as_array()
        .and_then(|parts| parts.iter().find(|p| p["type"] == "text"))
        .and_then(|p| p["text"].as_str())
        .unwrap_or("")
        .to_string();

    let answer = if question.is_empty() {
        "Please provide a question or request.".to_string()
    } else {
        match mcp_call(&state, "ask_yuki", &serde_json::json!({"question": question})).await {
            Ok(v) => v["content"][0]["text"].as_str().unwrap_or("").to_string(),
            Err(e) => format!("Error: {}", e),
        }
    };

    Json(serde_json::json!({
        "id": task.id,
        "status": {
            "state": "completed",
            "message": {
                "role": "agent",
                "parts": [{"type": "text", "text": answer}]
            }
        }
    }))
}

async fn mcp_discovery() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        axum::Json(serde_json::json!({
            "mcpServers": {
                "yukihamada": {
                    "url": "https://yukihamada.jp/mcp",
                    "name": "Yuki Hamada",
                    "description": "Blog search, project info, and AI Q&A for yukihamada.jp"
                }
            }
        }))
    )
}

// ── Video handlers ──

async fn video_upload(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    mut multipart: axum::extract::Multipart,
) -> Response {
    let token = params.get("token").cloned().unwrap_or_default();
    let uploader_email = {
        let s = state.user_sessions.lock().unwrap();
        s.get(&token).filter(|(_, exp)| *exp > now_secs()).map(|(e, _)| e.clone())
    };

    let mut video_data: Vec<u8> = Vec::new();
    let mut title = String::new();
    let mut is_public = true;
    let mut mime_type = "video/webm".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                let ct = field.content_type().unwrap_or("").to_string();
                // Only allow real video MIME types — reject anything else
                let allowed = ["video/webm", "video/mp4", "video/quicktime", "video/ogg"];
                if !allowed.iter().any(|&a| ct.starts_with(a)) {
                    return (StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        Json(serde_json::json!({"ok":false,"error":"動画ファイルのみアップロードできます"}))).into_response();
                }
                mime_type = ct;
                let bytes = field.bytes().await.unwrap_or_default();
                if bytes.len() > MAX_VIDEO_BYTES {
                    return (StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({"ok":false,"error":"ファイルサイズが大きすぎます（上限150MB）"}))).into_response();
                }
                // Validate magic bytes (WebM: 0x1A45DFA3, MP4: ftyp at offset 4)
                let valid_magic = bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3])  // WebM/MKV
                    || (bytes.len() > 11 && &bytes[4..8] == b"ftyp")             // MP4
                    || (bytes.len() > 3 && &bytes[0..4] == b"OggS");             // Ogg
                if !valid_magic {
                    return (StatusCode::UNPROCESSABLE_ENTITY,
                        Json(serde_json::json!({"ok":false,"error":"不正なファイル形式です"}))).into_response();
                }
                video_data = bytes.to_vec();
            }
            Some("title")     => { title     = field.text().await.unwrap_or_default().chars().take(120).collect(); }
            Some("is_public") => { is_public = field.text().await.unwrap_or_default() == "true"; }
            _ => {}
        }
    }

    if video_data.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok":false,"error":"動画データがありません"}))).into_response();
    }

    let uploader = uploader_email.unwrap_or_else(|| "anonymous".to_string());

    // Anonymous users can only upload public videos; limit to 3 per day by IP
    if !is_public && uploader == "anonymous" {
        return (StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"ok":false,"error":"プライベート保存にはログインが必要です"}))).into_response();
    }

    // Rate-limit: max 10 videos per uploader (prevents storage exhaustion)
    {
        let videos = state.videos.lock().unwrap();
        let today = now_secs() / 86400;
        let uploads_today = videos.iter()
            .filter(|v| v.uploader == uploader && v.created_at / 86400 == today)
            .count();
        if uploads_today >= 10 {
            return (StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({"ok":false,"error":"1日のアップロード上限（10件）に達しました"}))).into_response();
        }
    }

    use rand::Rng;
    let id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect();
    let ext = if mime_type.contains("mp4") { "mp4" } else { "webm" };
    // Ensure safe filename — id is alphanumeric only, ext is validated above
    let path = format!("{}/{}.{}", VIDEO_DIR, id, ext);

    if let Err(_) = std::fs::write(&path, &video_data) {
        return (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok":false,"error":"保存に失敗しました"}))).into_response();
    }

    let meta = VideoMeta { id: id.clone(), title, is_public, uploader, size_bytes: video_data.len() as u64, created_at: now_secs(), mime_type };
    {
        let mut videos = state.videos.lock().unwrap();
        videos.push(meta);
        save_video_meta(&videos);
    }

    (cors_headers(), Json(serde_json::json!({"ok":true,"id":id}))).into_response()
}

async fn video_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let token = params.get("token").cloned().unwrap_or_default();
    let is_admin = validate_admin_token(&state, &token);
    let videos = state.videos.lock().unwrap();
    let list: Vec<_> = videos.iter()
        .filter(|v| v.is_public || is_admin)
        .map(|v| serde_json::json!({ "id":v.id, "title":v.title, "is_public":v.is_public, "uploader":v.uploader, "size_bytes":v.size_bytes, "created_at":v.created_at, "mime_type":v.mime_type }))
        .collect();
    (cors_headers(), Json(serde_json::json!({"ok":true,"videos":list}))).into_response()
}

async fn video_stream(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let token = params.get("token").cloned().unwrap_or_default();
    let is_admin = validate_admin_token(&state, &token);
    let meta = state.videos.lock().unwrap().iter().find(|v| v.id == id).cloned();
    let meta = match meta {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    if !meta.is_public && !is_admin {
        return StatusCode::FORBIDDEN.into_response();
    }
    let ext = if meta.mime_type.contains("mp4") { "mp4" } else { "webm" };
    let data = match std::fs::read(format!("{}/{}.{}", VIDEO_DIR, id, ext)) {
        Ok(d) => d,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut h = axum::http::HeaderMap::new();
    h.insert(axum::http::header::CONTENT_TYPE, meta.mime_type.parse().unwrap_or_else(|_| "video/webm".parse().unwrap()));
    h.insert(axum::http::header::CACHE_CONTROL, "private, max-age=3600".parse().unwrap());
    (h, data).into_response()
}

async fn video_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let token = params.get("token").cloned().unwrap_or_default();
    if !validate_admin_token(&state, &token) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"ok":false}))).into_response();
    }
    let removed = {
        let mut videos = state.videos.lock().unwrap();
        let pos = videos.iter().position(|v| v.id == id);
        pos.map(|i| { let m = videos.remove(i); save_video_meta(&videos); m })
    };
    if let Some(m) = removed {
        let ext = if m.mime_type.contains("mp4") { "mp4" } else { "webm" };
        std::fs::remove_file(format!("{}/{}.{}", VIDEO_DIR, id, ext)).ok();
    }
    (cors_headers(), Json(serde_json::json!({"ok":true}))).into_response()
}

// ── Gmail API ───────────────────────────────────────────────────────────────

async fn gmail_get_access_token(state: &Arc<AppState>) -> Option<String> {
    let (ci, cs, rt) = match (&state.gmail_client_id, &state.gmail_client_secret, &state.gmail_refresh_token) {
        (Some(a), Some(b), Some(c)) => (a.clone(), b.clone(), c.clone()),
        _ => return None,
    };
    // Return cached token if still valid (5-min buffer)
    {
        let cached = state.gmail_access_token.lock().unwrap();
        if let Some((tok, exp)) = cached.as_ref() {
            if *exp > now_secs() + 300 { return Some(tok.clone()); }
        }
    }
    // Refresh
    let client = reqwest::Client::new();
    let resp = client.post("https://oauth2.googleapis.com/token")
        .form(&[("client_id",ci.as_str()),("client_secret",cs.as_str()),("refresh_token",rt.as_str()),("grant_type","refresh_token")])
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let token = json["access_token"].as_str()?.to_string();
    let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
    let mut cached = state.gmail_access_token.lock().unwrap();
    *cached = Some((token.clone(), now_secs() + expires_in));
    Some(token)
}

async fn mail_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if !validate_admin_token(&state, params.get("token").cloned().unwrap_or_default().as_str()) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"ok":false}))).into_response();
    }
    let access_token = match gmail_get_access_token(&state).await {
        Some(t) => t,
        None => return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"Gmail not configured"}))).into_response(),
    };
    let folder = params.get("folder").map(|s| s.as_str()).unwrap_or("inbox");
    let q = match folder {
        "sent"      => "in:sent",
        "important" => "is:important",
        "unread"    => "is:unread",
        _           => "in:inbox",
    };
    let max = params.get("max").and_then(|s| s.parse::<u32>().ok()).unwrap_or(30).min(50);
    let client = reqwest::Client::new();
    let q_encoded: String = q.chars().map(|c| if c.is_alphanumeric() || c == ':' || c == '.' { c.to_string() } else { format!("%{:02X}", c as u32) }).collect();
    let list_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages?q={}&maxResults={}&fields=messages(id,threadId)",
        q_encoded, max
    );
    let list_resp = client.get(&list_url)
        .bearer_auth(&access_token).send().await;
    let list_json: serde_json::Value = match list_resp {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"ok":false}))).into_response(),
    };
    let msg_ids: Vec<&str> = list_json["messages"].as_array()
        .map(|arr| arr.iter().filter_map(|m| m["id"].as_str()).collect())
        .unwrap_or_default();

    // Fetch each message header in parallel (up to 20)
    let mut tasks = Vec::new();
    for id in msg_ids.iter().take(20) {
        let id = id.to_string();
        let tok = access_token.clone();
        let c = client.clone();
        tasks.push(tokio::spawn(async move {
            let url = format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=To&metadataHeaders=Date",
                id
            );
            let r: serde_json::Value = c.get(&url).bearer_auth(&tok).send().await.ok()?.json().await.ok()?;
            let hdrs: HashMap<String, String> = r["payload"]["headers"].as_array()
                .map(|a| a.iter().filter_map(|h| {
                    Some((h["name"].as_str()?.to_lowercase(), h["value"].as_str()?.to_string()))
                }).collect()).unwrap_or_default();
            let labels: Vec<&str> = r["labelIds"].as_array()
                .map(|a| a.iter().filter_map(|l| l.as_str()).collect()).unwrap_or_default();
            Some(serde_json::json!({
                "id": id,
                "thread_id": r["threadId"].as_str().unwrap_or(""),
                "subject": hdrs.get("subject").cloned().unwrap_or_default(),
                "from": hdrs.get("from").cloned().unwrap_or_default(),
                "to": hdrs.get("to").cloned().unwrap_or_default(),
                "date": hdrs.get("date").cloned().unwrap_or_default(),
                "snippet": r["snippet"].as_str().unwrap_or(""),
                "unread": labels.contains(&"UNREAD"),
                "important": labels.contains(&"IMPORTANT"),
            }))
        }));
    }
    let mut messages = Vec::new();
    for t in tasks { if let Ok(Some(m)) = t.await { messages.push(m); } }
    (cors_headers(), Json(serde_json::json!({"ok":true,"messages":messages}))).into_response()
}

async fn mail_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if !validate_admin_token(&state, params.get("token").cloned().unwrap_or_default().as_str()) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"ok":false}))).into_response();
    }
    let access_token = match gmail_get_access_token(&state).await {
        Some(t) => t,
        None => return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false}))).into_response(),
    };
    let client = reqwest::Client::new();
    let url = format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=full", id);
    let msg: serde_json::Value = match client.get(&url).bearer_auth(&access_token).send().await {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => serde_json::Value::Null,
    };
    // Mark as read
    let _ = client.post(format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify", id))
        .bearer_auth(&access_token)
        .json(&serde_json::json!({"removeLabelIds":["UNREAD"]}))
        .send().await;
    // Extract body
    fn extract_body(part: &serde_json::Value) -> String {
        if let Some(data) = part["body"]["data"].as_str() {
            let bytes = data.replace('-', "+").replace('_', "/");
            if let Ok(decoded) = base64::decode(&bytes) {
                if let Ok(s) = String::from_utf8(decoded) { return s; }
            }
        }
        if let Some(parts) = part["parts"].as_array() {
            for p in parts {
                let mime = p["mimeType"].as_str().unwrap_or("");
                if mime == "text/html" || mime == "text/plain" {
                    let body = extract_body(p);
                    if !body.is_empty() { return body; }
                }
                let nested = extract_body(p);
                if !nested.is_empty() { return nested; }
            }
        }
        String::new()
    }
    let body = extract_body(&msg["payload"]);
    let hdrs: HashMap<String, String> = msg["payload"]["headers"].as_array()
        .map(|a| a.iter().filter_map(|h| Some((h["name"].as_str()?.to_lowercase(), h["value"].as_str()?.to_string()))).collect())
        .unwrap_or_default();
    (cors_headers(), Json(serde_json::json!({
        "ok": true,
        "id": id,
        "subject": hdrs.get("subject").cloned().unwrap_or_default(),
        "from": hdrs.get("from").cloned().unwrap_or_default(),
        "to": hdrs.get("to").cloned().unwrap_or_default(),
        "date": hdrs.get("date").cloned().unwrap_or_default(),
        "body": body,
    }))).into_response()
}
// ── End Gmail ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let posts = blog::load_posts(std::path::Path::new("content/blog"));
    let tags = blog::collect_tags(&posts);
    let stripe_key = std::env::var("STRIPE_SECRET_KEY").ok().filter(|s| !s.is_empty());
    let resend_key = std::env::var("RESEND_API_KEY").ok().filter(|s| !s.is_empty());
    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").ok().filter(|s| !s.is_empty());
    let charin_api_key = std::env::var("CHARIN_API_KEY").ok().filter(|s| !s.is_empty());
    let newsletter_admin_token = std::env::var("NEWSLETTER_ADMIN_TOKEN").ok().filter(|s| !s.is_empty());
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty());
    let mcp_key = std::env::var("MCP_API_KEY").ok().filter(|s| !s.is_empty());
    let m5_register_token = std::env::var("M5_REGISTER_TOKEN").ok().filter(|s| !s.is_empty());
    let m5_hitl_token = std::env::var("M5_HITL_TOKEN").ok().filter(|s| !s.is_empty());
    let m5_initial_url = std::env::var("M5_HITL_URL").ok().filter(|s| !s.is_empty());
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty());
    let groq_api_key = std::env::var("GROQ_API_KEY").ok().filter(|s| !s.is_empty());
    let gmail_client_id = std::env::var("GMAIL_CLIENT_ID").ok().filter(|s| !s.is_empty());
    let gmail_client_secret = std::env::var("GMAIL_CLIENT_SECRET").ok().filter(|s| !s.is_empty());
    let gmail_refresh_token = std::env::var("GMAIL_REFRESH_TOKEN").ok().filter(|s| !s.is_empty());
    let gmail_email = std::env::var("GMAIL_EMAIL").ok().filter(|s| !s.is_empty());
    if gmail_client_id.is_some() { println!("Gmail API configured"); }
    if telegram_token.is_some() { println!("Telegram notifications configured"); }
    if groq_api_key.is_some() { println!("Groq transcription configured"); }
    let (chat_notify_tx, _) = broadcast::channel::<String>(128);
    if charin_api_key.is_some() { println!("Charin API key configured"); }
    if stripe_key.is_some() { println!("Stripe configured"); }
    if resend_key.is_some() { println!("Resend configured"); }
    if newsletter_admin_token.is_some() { println!("Newsletter admin token configured"); }
    if anthropic_key.is_some() { println!("Anthropic AI chat configured"); }
    if mcp_key.is_some() { println!("MCP API key configured"); }
    // Load persisted sessions (survive server restarts)
    let now_ts = now_secs();
    let saved = load_sessions();
    let fanclub_sessions_init: HashMap<String, (String, u64)> = saved.fanclub
        .into_iter().filter(|(_, (_, exp))| *exp > now_ts).collect();
    let dash_sessions_init: HashMap<String, u64> = saved.dash
        .into_iter().filter(|(_, exp)| *exp > now_ts).collect();
    println!("Loaded {} fanclub + {} dash sessions from disk", fanclub_sessions_init.len(), dash_sessions_init.len());
    let state = Arc::new(AppState {
        posts, tags, stripe_key, resend_key, stripe_webhook_secret, charin_api_key,
        newsletter_admin_token, anthropic_key, mcp_key,
        otp_store: Mutex::new(HashMap::new()),
        fanclub_sessions: Mutex::new(fanclub_sessions_init),
        otp_rate_limit: Mutex::new(HashMap::new()),
        dash_sessions: Mutex::new(dash_sessions_init),
        newsletter_auth_attempts: Mutex::new(HashMap::new()),
        chat_rate_limit: Mutex::new(HashMap::new()),
        mcp_rate_limit: Mutex::new(HashMap::new()),
        m5_url: Mutex::new(m5_initial_url),
        m5_register_token,
        m5_hitl_token,
        user_memory: Mutex::new(load_user_memory()),
        admin_sessions: Mutex::new(HashMap::new()),
        user_sessions: Mutex::new(HashMap::new()),
        shared_pty: Arc::new(SharedPty::new()),
        telegram_token,
        groq_api_key,
        chat_notify_tx,
        pending_admin_replies: Mutex::new(HashMap::new()),
        owner_last_seen: AtomicU64::new(0),
        pending_live_chats: Mutex::new(HashMap::new()),
        videos: Mutex::new(load_video_meta()),
        gmail_client_id,
        gmail_client_secret,
        gmail_refresh_token,
        gmail_email,
        gmail_access_token: Mutex::new(None),
    });
    std::fs::create_dir_all(VIDEO_DIR).ok();

    let app = Router::new()
        .route("/", get(home))
        .route("/ja", get(redirect_root))
        .route("/en", get(redirect_root))
        .route("/about", get(about))
        .route("/terminal", get(redirect_terminal))
        .route("/projects", get(redirect_projects))
        .route("/career",   get(redirect_career))
        .route("/music",    get(redirect_music))
        .route("/browser",  get(redirect_browser))
        .route("/koe",      get(redirect_koe))
        .route("/uta",      get(redirect_uta))
        .route("/news",     get(redirect_news))
        .route("/settings", get(redirect_settings))
        .route("/camera",   get(redirect_camera))
        .route("/game",     get(redirect_game))
        .route("/finder",   get(redirect_finder))
        .route("/contact",  get(redirect_contact))
        .route("/now",      get(redirect_now))
        .route("/podcast",  get(redirect_podcast))
        .route("/soluna", get(soluna_page))
        .route("/blog", get(blog_list_tag))
        .route("/blog/soluna/{slug}", get(blog_soluna_proxy))
        .route("/blog/{slug}", get(blog_post))
        .route("/sitemap.xml", get(sitemap))
        .route("/feed.xml", get(rss_feed))
        .route("/robots.txt", get(robots))
        .route("/health", get(health))
        .nest_service("/anime", ServeDir::new("public/anime"))
        .nest_service("/mv", ServeDir::new("public/mv"))
        .route("/api/fanclub/verify", post(fanclub_verify))
        .route("/api/fanclub/verify", axum::routing::options(options_cors))
        .route("/api/fanclub/otp/send", post(fanclub_send_otp))
        .route("/api/fanclub/otp/send", axum::routing::options(options_cors))
        .route("/api/fanclub/otp/verify", post(fanclub_verify_otp))
        .route("/api/fanclub/otp/verify", axum::routing::options(options_cors))
        .route("/fanclub", get(fanclub_login_page))
        .route("/fanclub/members", get(fanclub_members))
        .route("/fanclub/logout", get(fanclub_logout))
        .route("/api/newsletter", post(newsletter_post_with_notify))
        .route("/api/newsletter", get(newsletter_get))
        .route("/api/newsletter", axum::routing::options(options_cors))
        .route("/api/stripe/webhook", post(stripe_webhook))
        .nest_service("/blog/images", ServeDir::new("public/blog/images"))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/audio", ServeDir::new("public/audio"))
        .route("/favicon.svg", get(|| async {
            let body = std::fs::read_to_string("public/favicon.svg").unwrap_or_default();
            ([("content-type", "image/svg+xml"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/favicon.ico", get(|| async {
            let body = std::fs::read("public/favicon.ico").unwrap_or_default();
            ([("content-type", "image/x-icon"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/apple-touch-icon.png", get(|| async {
            let body = std::fs::read("public/apple-touch-icon.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/favicon-16x16.png", get(|| async {
            let body = std::fs::read("public/favicon-16x16.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=604800")], body)
        }))
        .route("/favicon-32x32.png", get(|| async {
            let body = std::fs::read("public/favicon-32x32.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=604800")], body)
        }))
        .route("/favicon-192.png", get(|| async {
            let body = std::fs::read("public/favicon-192.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=604800")], body)
        }))
        .route("/favicon-512.png", get(|| async {
            let body = std::fs::read("public/favicon-512.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=604800")], body)
        }))
        .route("/og-image.jpg", get(|| async {
            let body = std::fs::read("public/og-image.jpg").unwrap_or_default();
            ([("content-type", "image/jpeg"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/og-image.svg", get(|| async {
            let body = std::fs::read_to_string("public/og-image.svg").unwrap_or_default();
            ([("content-type", "image/svg+xml"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/og-image.png", get(|| async {
            let body = std::fs::read("public/og-image.png").unwrap_or_default();
            ([("content-type", "image/png"), ("cache-control", "public, max-age=86400")], body)
        }))
        .route("/sw.js", get(|| async {
            let body = std::fs::read_to_string("public/sw.js").unwrap_or_default();
            ([("content-type", "application/javascript"), ("cache-control", "no-cache")], body)
        }))
        .route("/api/video/upload", post(video_upload))
        .route("/api/video/upload", axum::routing::options(options_cors))
        .route("/api/videos", get(video_list))
        .route("/api/video/{id}", get(video_stream))
        .route("/api/video/{id}", axum::routing::delete(video_delete))
        .route("/api/mail/list", get(mail_list))
        .route("/api/mail/message/{id}", get(mail_message))
        .route("/manifest.webmanifest", get(|| async {
            let body = r##"{"name":"濱田優貴 / Yuki Hamada","short_name":"YukiHamada","start_url":"/","display":"standalone","background_color":"#080810","theme_color":"#080810","description":"Enabler CEO - ex-Mercari CPO - Builder","icons":[{"src":"/favicon-192.png","sizes":"192x192","type":"image/png"},{"src":"/favicon-512.png","sizes":"512x512","type":"image/png","purpose":"any maskable"}],"categories":["business","personal"],"lang":"ja"}"##;
            ([("content-type", "application/manifest+json"), ("cache-control", "public, max-age=3600")], body)
        }))
        .route("/api/login/otp", post(admin_send_otp))
        .route("/api/login/otp", axum::routing::options(options_cors))
        .route("/api/login/verify", post(admin_verify_otp))
        .route("/api/login/verify", axum::routing::options(options_cors))
        .route("/api/login/me", get(admin_me))
        .route("/api/user/otp", post(user_send_otp))
        .route("/api/user/otp", axum::routing::options(options_cors))
        .route("/api/user/verify", post(user_verify_otp))
        .route("/api/user/verify", axum::routing::options(options_cors))
        .route("/api/user/me", get(user_me))
        .route("/ws/terminal", get(ws_terminal))
        .route("/yukiterm", get(yukiterm_script))
        .route("/chat", get(chat_page))
        .route("/api/chat", post(chat_handler))
        .route("/api/chat", axum::routing::options(options_cors))
        .route("/api/m5/register", post(m5_register_handler))
        .route("/api/m5/status", get(m5_status_handler))
        .route("/api/chat/memory", get(user_memory_get).delete(user_memory_delete))
        .route("/api/chat/admin-stream", get(admin_chat_stream))
        .route("/api/chat/admin-reply", post(admin_chat_reply))
        .route("/api/chat/admin-reply", axum::routing::options(options_cors))
        .route("/api/chat/poll-reply", get(poll_admin_reply))
        .route("/api/chat/owner-heartbeat", post(owner_heartbeat_handler))
        .route("/api/chat/owner-heartbeat", axum::routing::options(options_cors))
        .route("/api/chat/owner-online", get(owner_online_handler))
        .route("/api/transcribe", post(transcribe_audio))
        .route("/api/transcribe", axum::routing::options(options_cors))
        .route("/api/chat/ai-suggest", post(admin_ai_suggest))
        .route("/api/chat/ai-suggest", axum::routing::options(options_cors))
        .route("/mcp", get(mcp_page).post(mcp_handler))
        .route("/mcp", axum::routing::options(options_cors))
        .route("/.well-known/mcp.json", get(mcp_discovery))
        .route("/.well-known/agent.json", get(a2a_agent_card))
        .route("/a2a", post(a2a_tasks_handler))
        .route("/a2a", axum::routing::options(options_cors))
        .route("/api/analytics/log", post(analytics_log))
        .route("/api/analytics/log", axum::routing::options(options_cors))
        .route("/api/analytics", get(analytics_dashboard))
        .route("/analytics", get(admin_analytics))
        .route("/dashboard", get(analytics_dashboard))
        .route("/dashboard/login", post(dashboard_login_post))
        .route("/dashboard/x", get(x_dashboard))
        .route("/api/x/auth", get(x_auth_start))
        .route("/api/x/callback", get(x_auth_callback))
        .route("/api/x/drafts", get(x_drafts_list))
        .route("/api/x/drafts", post(x_drafts_action))
        .route("/api/x/connection", post(x_connection))
        .fallback(get(not_found))
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(security_headers))
        .layer(axum::middleware::from_fn(redirect_hamada_tokyo));

    let addr = "0.0.0.0:8080";
    println!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
