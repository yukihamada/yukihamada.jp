//! ともしび — yukihamada.jp に間借りするコミュニティ。
//!
//! コンセプト: 権限がある人（メール認証で api_token を持つ人）が MCP で
//! 「火をともす / 薪をくべる」と、公開ページのリアルな焚き火に投稿が積もる。
//!
//! - ストレージ: /data/community/{members,posts}.json （既存サイト流儀のJSONファイル）
//! - 認証: メール magic-link → 確認で api_token 発行（ATSMで欠けていた「動くトークン」）
//! - MCP: POST /community/mcp （読み取り無認証 / 書き込みは `Authorization: Bearer <api_token>`）
//! - ページ: GET /community （Canvasのリアルな焚き火＋薪フィード）

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::AppState;

const FROM_EMAIL: &str = "ともしび <info@enablerdao.com>";
static STORE_LOCK: Mutex<()> = Mutex::new(());
/// auth_request のレート制限: email -> 直近のリクエスト時刻(秒)。メール爆撃の踏み台防止。
static AUTH_RL: Mutex<BTreeMap<String, Vec<i64>>> = Mutex::new(BTreeMap::new());

fn base_url() -> String {
    std::env::var("COMMUNITY_BASE_URL")
        .or_else(|_| std::env::var("BASE_URL"))
        .unwrap_or_else(|_| "https://yukihamada.jp".to_string())
}

fn data_dir() -> String {
    std::env::var("COMMUNITY_DATA_DIR").unwrap_or_else(|_| "/data/community".to_string())
}

// ── storage ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct Member {
    id: String,
    email: String,
    name: String,
    api_token: String,
    role: String, // "keeper" | "member"
    created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Post {
    id: String,
    author_name: String,
    body: String,
    kind: String, // "log"(薪) | "ignite"(着火) | "content"
    url: Option<String>,
    created_at: String,
}

/// 焚き火は10分薪が来ないと消え、その薪は「建物」に移って永遠に残る（append-only）。
fn fire_ttl_secs() -> i64 {
    std::env::var("COMMUNITY_FIRE_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600)
}

#[derive(Serialize, Deserialize, Clone)]
struct Building {
    id: String,
    started_at: String,
    ended_at: String,
    log_count: usize,
    members: Vec<String>,
    posts: Vec<Post>,
}

#[derive(Serialize, Deserialize, Clone)]
struct MagicToken {
    token: String,
    email: String,
    name: String,
    expires_at: i64,
    used: bool,
}

fn load<T: DeserializeOwned>(name: &str) -> Vec<T> {
    let path = format!("{}/{}", data_dir(), name);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(name: &str, v: &[T]) {
    let _ = std::fs::create_dir_all(data_dir());
    let path = format!("{}/{}", data_dir(), name);
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(&path, s);
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn rand_hex(n: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..n).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
}

fn member_by_token(token: &str) -> Option<Member> {
    if token.is_empty() {
        return None;
    }
    load::<Member>("members.json")
        .into_iter()
        .find(|m| m.api_token == token)
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())?;
    raw.split(';')
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == "tomoshibi")
        .map(|(_, v)| v.to_string())
}

/// 表示言語を決める（global-first: ?lang / Accept-Language、既定は英語）。
#[derive(Deserialize, Default)]
pub struct LangQ {
    #[serde(default)]
    lang: Option<String>,
}

fn pick_lang(headers: &HeaderMap, q: &LangQ) -> &'static str {
    if let Some(l) = q.lang.as_deref() {
        if l == "en" {
            return "en";
        }
        if l == "ja" {
            return "ja";
        }
    }
    let al = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    if al.starts_with("ja") || al.split(',').any(|p| p.trim().starts_with("ja")) {
        "ja"
    } else {
        "en"
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── seed ───────────────────────────────────────────────────────────────────

/// 起動時に一度だけ呼ぶ。投稿が空なら紙芝居「悪魔を1匹」を最初の薪として置く。
pub fn seed_if_empty() {
    let _g = STORE_LOCK.lock().unwrap();
    let mut posts: Vec<Post> = load("posts.json");
    let buildings: Vec<Building> = load("buildings.json");
    if posts.is_empty() && buildings.is_empty() {
        posts.push(Post {
            id: rand_hex(8),
            author_name: "濱田 優貴".to_string(),
            body: "紙芝居『あなたの中に、悪魔を1匹。』第1話 を公開。本人クローン声＋12シーンの音声同期紙芝居。最初の薪をくべます🔥".to_string(),
            kind: "content".to_string(),
            url: Some("https://devil-podcast.fly.dev/".to_string()),
            created_at: now_rfc3339(),
        });
        save("posts.json", &posts);
    }
}

// ── auth ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuthReq {
    email: String,
    #[serde(default)]
    name: String,
}

pub async fn auth_request(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthReq>,
) -> Response {
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 200 {
        return (StatusCode::BAD_REQUEST, "invalid email").into_response();
    }
    let name = if body.name.trim().is_empty() {
        email.split('@').next().unwrap_or("名無し").to_string()
    } else {
        body.name.trim().chars().take(40).collect()
    };

    // rate limit: 同一メール 60秒に1通・5通/時まで（メール爆撃の踏み台防止）
    {
        let now = chrono::Utc::now().timestamp();
        let mut rl = AUTH_RL.lock().unwrap();
        if rl.len() > 10_000 {
            rl.retain(|_, v| v.iter().any(|t| now - *t < 3600));
        }
        let v = rl.entry(email.clone()).or_default();
        v.retain(|t| now - *t < 3600);
        if v.iter().any(|t| now - *t < 60) || v.len() >= 5 {
            return (StatusCode::TOO_MANY_REQUESTS, "少し時間をおいて再度お試しください").into_response();
        }
        v.push(now);
    }

    let token = rand_hex(32);
    {
        let _g = STORE_LOCK.lock().unwrap();
        let mut toks: Vec<MagicToken> = load("magic_tokens.json");
        toks.retain(|t| !t.used && t.expires_at > chrono::Utc::now().timestamp());
        toks.push(MagicToken {
            token: token.clone(),
            email: email.clone(),
            name,
            expires_at: chrono::Utc::now().timestamp() + 1800,
            used: false,
        });
        save("magic_tokens.json", &toks);
    }

    let link = format!("{}/api/community/auth/verify?token={}", base_url(), token);
    if let Some(key) = state.resend_key.clone() {
        let client = reqwest::Client::new();
        let _ = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "from": FROM_EMAIL,
                "to": [email],
                "subject": "ともしび — ログインリンク🔥",
                "html": format!(
                    "<div style='font-family:sans-serif;line-height:1.7'>\
                     <p>下のリンクで焚き火に火をともします（30分有効）。</p>\
                     <p><a href='{link}' style='background:#e8651f;color:#fff;padding:12px 22px;border-radius:6px;text-decoration:none'>🔥 火をともす</a></p>\
                     <p style='color:#888;font-size:12px'>{link}</p></div>"
                ),
            }))
            .send()
            .await;
    } else {
        println!("DEV community magic link for {email}: {link}");
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
pub struct VerifyQ {
    token: String,
}

pub async fn auth_verify(Query(q): Query<VerifyQ>) -> Response {
    let (email, name) = {
        let _g = STORE_LOCK.lock().unwrap();
        let mut toks: Vec<MagicToken> = load("magic_tokens.json");
        let now = chrono::Utc::now().timestamp();
        let Some(idx) = toks
            .iter()
            .position(|t| t.token == q.token && !t.used && t.expires_at > now)
        else {
            return (StatusCode::BAD_REQUEST, Html(
                "<body style='background:#111;color:#eee;font-family:sans-serif;text-align:center;padding:80px'>リンクが無効か期限切れです。<br><a style='color:#e8a' href='/community/join'>もう一度</a></body>".to_string()
            )).into_response();
        };
        toks[idx].used = true;
        let email = toks[idx].email.clone();
        let name = toks[idx].name.clone();
        save("magic_tokens.json", &toks);

        // upsert member
        let mut members: Vec<Member> = load("members.json");
        if let Some(m) = members.iter_mut().find(|m| m.email == email) {
            if m.name != name && !name.is_empty() {
                m.name = name.clone();
            }
        } else {
            members.push(Member {
                id: rand_hex(8),
                email: email.clone(),
                name: name.clone(),
                api_token: rand_hex(32),
                role: "member".to_string(),
                created_at: now_rfc3339(),
            });
        }
        save("members.json", &members);
        (email, name)
    };

    let api_token = load::<Member>("members.json")
        .into_iter()
        .find(|m| m.email == email)
        .map(|m| m.api_token)
        .unwrap_or_default();

    let cookie = format!(
        "tomoshibi={api_token}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Lax; Secure"
    );
    let mut resp = Redirect::to("/community/welcome").into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        header::HeaderValue::from_str(&cookie).unwrap(),
    );
    let _ = name;
    resp
}

pub async fn welcome_page(headers: HeaderMap, Query(q): Query<LangQ>) -> Response {
    let Some(tok) = cookie_token(&headers) else {
        return Redirect::to("/community/join").into_response();
    };
    let Some(m) = member_by_token(&tok) else {
        return Redirect::to("/community/join").into_response();
    };
    let base = base_url();
    let ja = pick_lang(&headers, &q) == "ja";
    let (lang_attr, title, h1, sub, l_key, l_add, l_talk, say1, say2, watch) = if ja {
        (
            "ja",
            "ともしび — 火がともりました",
            format!("{} さん、火がともりました", esc(&m.name)),
            "あなたの権限キー（api_token）です。これで Claude から焚き火を操作できます。",
            "あなたの api_token",
            "Claude に追加（1回だけ）",
            "Claude で話しかける",
            "ともしびに火をともして",
            "今やってる作業を薪にして",
            "🔥 焚き火を見る",
        )
    } else {
        (
            "en",
            "Tomoshibi — Fire lit",
            format!("{}, your fire is lit", esc(&m.name)),
            "This is your api key. It lets Claude tend the campfire.",
            "Your api_token",
            "Add to Claude (once)",
            "Talk to Claude",
            "Light the fire on Tomoshibi",
            "Turn what I'm working on into a log",
            "🔥 See the fire",
        )
    };
    // セキュリティ: 権限キーは画面に平文で晒さない。表示はマスク、コピーは全文。
    let masked = {
        let t = &m.api_token;
        if t.len() > 12 {
            format!("{}…{}", &t[..6], &t[t.len() - 4..])
        } else {
            "•".repeat(t.len())
        }
    };
    let cmd_full = format!(
        "claude mcp add --transport http tomoshibi {base}/community/mcp --header \"Authorization: Bearer {}\"",
        m.api_token
    );
    let cmd_masked = cmd_full.replace(m.api_token.as_str(), &masked);
    let html = format!(
        r#"<!DOCTYPE html><html lang={lang_attr}><head><meta charset=utf-8>
<meta name=viewport content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>{css}</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body class=center>
<div class=card>
<div class=fire-emoji>🔥</div>
<h1>{h1}</h1>
<p class=sub>{sub}</p>
<div class=label>{l_key}</div>
<div class=cmd onclick="cp(this,this.dataset.c)" data-c='{token}'>{masked}<span class=hint>コピー</span></div>
<div class=label>{l_add}</div>
<div class=cmd onclick="cp(this,this.dataset.c)" data-c='{cmd_full}'>{cmd_masked}<span class=hint>コピー</span></div>
<div class=label>{l_talk}</div>
<div class=say onclick="cp(this,'{say1}')">"{say1}"<span class=hint>コピー</span></div>
<div class=say onclick="cp(this,'{say2}')">"{say2}"<span class=hint>コピー</span></div>
<a class=watch href="/community">{watch}</a>
</div>
<script>
try{{localStorage.setItem('rtcName',{name_js});}}catch(_){{}}
function cp(el,t){{navigator.clipboard.writeText(t).catch(()=>{{}});var h=el.querySelector('.hint');if(h){{var o=h.textContent;h.textContent='コピー済 ✓';setTimeout(()=>h.textContent=o,1500)}}}}
</script>
</body></html>"#,
        css = PAGE_CSS,
        name_js = serde_json::to_string(&m.name).unwrap_or_else(|_| "\"\"".into()),
        token = esc(&m.api_token),
    );
    Html(html).into_response()
}

// ── public APIs ────────────────────────────────────────────────────────────

/// 認証済みか（api_token を cookie か Bearer で持っているか）。
/// 未認証には実名・本文・メール・トークンを一切出さない（fail-closed）。
fn is_authed(headers: &HeaderMap) -> bool {
    let tok = cookie_token(headers).or_else(|| bearer(headers)).unwrap_or_default();
    member_by_token(&tok).is_some()
}

/// 焚き火の勢いが増えたか減ったかを正直に出す。
/// 薪 = いま生きてる薪(posts.json) ＋ 建物に積もった薪(buildings.json) の作成時刻を母集団に、
/// 直近24時間 と その前の24時間 の本数を比べる。人=入会・建物=形成(ended_at) も同様。
/// delta>0 なら火は育っている、<0 なら冷めてきている。
fn activity_trend() -> serde_json::Value {
    let now = chrono::Utc::now().timestamp();
    let win = 86_400i64; // 24h
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|t| t.timestamp())
    };
    // 直近winと、その前のwinの件数
    let bucket = |ts: &[i64]| -> (i64, i64) {
        let (mut last, mut prev) = (0i64, 0i64);
        for &t in ts {
            let age = now - t;
            if age < win {
                last += 1;
            } else if age < 2 * win {
                prev += 1;
            }
        }
        (last, prev)
    };
    let buildings = load::<Building>("buildings.json");
    // 薪: 生きてる薪 ＋ 建物に積もった薪、すべての created_at
    let mut wood: Vec<i64> = load::<Post>("posts.json")
        .iter()
        .filter(|p| is_log(p))
        .filter_map(|p| parse(&p.created_at))
        .collect();
    wood.extend(
        buildings
            .iter()
            .flat_map(|b| b.posts.iter())
            .filter_map(|p| parse(&p.created_at)),
    );
    let people: Vec<i64> = load::<Member>("members.json")
        .iter()
        .filter_map(|m| parse(&m.created_at))
        .collect();
    let blds: Vec<i64> = buildings.iter().filter_map(|b| parse(&b.ended_at)).collect();
    let chip = |ts: &[i64]| {
        let (last, prev) = bucket(ts);
        serde_json::json!({ "last": last, "prev": prev, "delta": last - prev })
    };
    serde_json::json!({
        "window_secs": win,
        "wood": chip(&wood),
        "people": chip(&people),
        "buildings": chip(&blds),
    })
}

pub async fn api_posts(headers: HeaderMap) -> Json<serde_json::Value> {
    {
        let _g = STORE_LOCK.lock().unwrap();
        let _ = archive_locked(); // 消えた火は遅延的に建物へ
    }
    let (alive, remain) = fire_state();
    let mut posts: Vec<Post> = load("posts.json");
    let count = posts.iter().filter(|p| is_log(p)).count(); // 薪の数（炎の大きさ）
    posts.reverse(); // newest first
    posts.truncate(100);
    let buildings = load::<Building>("buildings.json").len();
    let authed = is_authed(&headers);
    // 未ログインには本文・実名・冒頭を出さない（実名/機密が冒頭に来る投稿で漏れるため）。
    // 炎の大きさが分かる最小限（件数・種別・時刻）だけをティーザーとして返す。
    let posts_out: Vec<serde_json::Value> = if authed {
        posts.iter().map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null)).collect()
    } else {
        posts
            .iter()
            .map(|p| serde_json::json!({
                "id": p.id,
                "kind": p.kind,
                "created_at": p.created_at,
                "teaser": true,
            }))
            .collect()
    };
    Json(serde_json::json!({
        "posts": posts_out,
        "count": count,
        "fire_alive": alive,
        "remain_secs": remain,
        "ttl_secs": fire_ttl_secs(),
        "buildings": buildings,
        "trend": activity_trend(), // 薪・人・建物が直近24hで増えたか減ったか
        "authed": authed,
    }))
}

pub async fn api_buildings(headers: HeaderMap) -> Json<serde_json::Value> {
    {
        let _g = STORE_LOCK.lock().unwrap();
        let _ = archive_locked();
    }
    let mut bs: Vec<Building> = load("buildings.json");
    bs.reverse();
    if !is_authed(&headers) {
        // 未ログインには実名・本文を出さない。建物の存在と規模（薪数・期間）だけ。
        let teaser: Vec<serde_json::Value> = bs
            .iter()
            .map(|b| serde_json::json!({
                "id": b.id,
                "started_at": b.started_at,
                "ended_at": b.ended_at,
                "log_count": b.log_count,
                "members_count": b.members.len(),
                "teaser": true,
            }))
            .collect();
        return Json(serde_json::json!({ "buildings": teaser, "authed": false }));
    }
    Json(serde_json::json!({ "buildings": bs, "authed": true }))
}

/// 保存データの可視化用。投稿/メンバー/建物の件数・保存先・最終更新を返す。
pub async fn api_stats(headers: HeaderMap) -> axum::response::Response {
    use axum::response::IntoResponse;
    // 会員(api_token cookie/bearer)のみ。未ログインは 401 → トップでは非表示。
    let tok = cookie_token(&headers).or_else(|| bearer(&headers)).unwrap_or_default();
    if member_by_token(&tok).is_none() {
        return (axum::http::StatusCode::UNAUTHORIZED, Json(serde_json::json!({"authed": false}))).into_response();
    }
    {
        let _g = STORE_LOCK.lock().unwrap();
        let _ = archive_locked();
    }
    let posts: Vec<Post> = load("posts.json");
    let logs = posts.iter().filter(|p| is_log(p)).count();
    let buildings = load::<Building>("buildings.json").len();
    let members = load::<Member>("members.json").len();
    // 最終更新 = データファイルの最新 mtime を日付に
    let dir = data_dir();
    let newest = ["posts.json", "buildings.json", "members.json"]
        .iter()
        .filter_map(|f| std::fs::metadata(format!("{}/{}", dir, f)).ok())
        .filter_map(|m| m.modified().ok())
        .max();
    let updated = newest
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.format("%Y-%m-%d").to_string()
        })
        .unwrap_or_else(|| "—".to_string());
    Json(serde_json::json!({
        "posts": posts.len(),
        "logs": logs,
        "members": members,
        "buildings": buildings,
        "storage": "/data/community (Fly volume・追記型)",
        "updated": updated,
    })).into_response()
}

pub async fn api_members(headers: HeaderMap) -> Json<serde_json::Value> {
    // 未ログインには実名を出さない。人数のみ（炎を囲む人数の可視化はティーザーとして残す）。
    let count = load::<Member>("members.json").len();
    if !is_authed(&headers) {
        return Json(serde_json::json!({ "count": count, "authed": false }));
    }
    let members: Vec<serde_json::Value> = load::<Member>("members.json")
        .into_iter()
        .map(|m| serde_json::json!({"name": m.name, "role": m.role, "since": m.created_at}))
        .collect();
    Json(serde_json::json!({ "count": count, "members": members, "authed": true }))
}

/// 声の焚き火 (koe.live) の在室人数プロキシ。
/// ブラウザ→koe.live は CORS が無いため同一オリジンで中継する。
/// room=atsmwtf は ATSUME の焚き火と同じ声部屋 = 2つのページが1つの火を囲む。
pub async fn api_koe_presence() -> Json<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut count = 0u64;
    let mut total = 0u64;
    if let Ok(r) = client.get("https://koe.live/api/rooms").send().await {
        if let Ok(v) = r.json::<serde_json::Value>().await {
            if let Some(rooms) = v.get("rooms").and_then(|x| x.as_array()) {
                for room in rooms {
                    let peers = room.get("peers").and_then(|p| p.as_u64()).unwrap_or(0);
                    total += peers;
                    if room.get("name").and_then(|n| n.as_str()) == Some("atsmwtf") {
                        count = peers;
                    }
                }
            }
        }
    }
    Json(serde_json::json!({ "room": "atsmwtf", "count": count, "total": total }))
}

// ── shared write helpers ─────────────────────────────────────────────────

/// 薪か？（kind=content のピン留め作品は焚き火ライフサイクルの対象外）。
fn is_log(p: &Post) -> bool {
    p.kind != "content"
}

/// 最後の薪から FIRE_TTL_SECS 過ぎていたら、薪だけを建物に移す。
/// ピン留め(content)は posts.json に残す。STORE_LOCK 保持下で呼ぶこと。
fn archive_locked() -> Option<Building> {
    let all: Vec<Post> = load("posts.json");
    let logs: Vec<Post> = all.iter().filter(|p| is_log(p)).cloned().collect();
    if logs.is_empty() {
        return None;
    }
    let last = logs.iter().map(|p| &p.created_at).max()?;
    let last_ts = chrono::DateTime::parse_from_rfc3339(last).ok()?.timestamp();
    if chrono::Utc::now().timestamp() - last_ts < fire_ttl_secs() {
        return None;
    }
    let started = logs.iter().map(|p| p.created_at.clone()).min().unwrap_or_default();
    let mut members: Vec<String> = logs.iter().map(|p| p.author_name.clone()).collect();
    members.sort();
    members.dedup();
    let b = Building {
        id: rand_hex(8),
        started_at: started,
        ended_at: last.clone(),
        log_count: logs.len(),
        members,
        posts: logs,
    };
    let mut buildings: Vec<Building> = load("buildings.json");
    buildings.push(b.clone());
    save("buildings.json", &buildings);
    // ピン留め(content)だけ残す
    let pinned: Vec<Post> = all.into_iter().filter(|p| !is_log(p)).collect();
    save("posts.json", &pinned);
    Some(b)
}

/// 火が生きているか（最後の薪から10分以内）と、残り秒。薪が無ければ未着火。
fn fire_state() -> (bool, i64) {
    let posts: Vec<Post> = load("posts.json");
    let Some(last) = posts.iter().filter(|p| is_log(p))
        .filter_map(|p| chrono::DateTime::parse_from_rfc3339(&p.created_at).ok())
        .map(|d| d.timestamp()).max() else {
        return (false, 0);
    };
    let remain = fire_ttl_secs() - (chrono::Utc::now().timestamp() - last);
    (remain > 0, remain.max(0))
}

fn add_post(author: &str, body: &str, kind: &str, url: Option<String>) -> Post {
    let _g = STORE_LOCK.lock().unwrap();
    let _ = archive_locked(); // 消えた火を建物に移してから、新しい薪をくべる
    let mut posts: Vec<Post> = load("posts.json");
    let p = Post {
        id: rand_hex(8),
        author_name: author.to_string(),
        body: body.chars().take(2000).collect(),
        kind: kind.to_string(),
        url,
        created_at: now_rfc3339(),
    };
    posts.push(p.clone());
    if posts.len() > 1000 {
        let drop = posts.len() - 1000;
        posts.drain(0..drop);
    }
    save("posts.json", &posts);
    p
}

// ── 火を実体にリンクする（A:webhook薪 / C:記念日 / D:federation） ───────────
//
// 旧「番人ポエム」は撤去。火の燃料を Bot の定型文ではなく、すでに鼓動している
// 実体（コミット/デプロイ/売上・記念日・他コミュニティの火）に繋ぐ。
// → 無人でも火が生き、しかも薪はすべて本物のイベント＝コンセプトを汚さない。

/// 起動時に呼ぶ。記念日点火(C)と federation(D) を、設定があるときだけ動かす。
/// 設定（anniversaries.json / COMMUNITY_FEDERATION_PEERS）が無ければ何もしない＝安全。
pub fn spawn_background() {
    spawn_anniversary();
    spawn_federation();
}

#[derive(Serialize, Deserialize, Clone)]
struct Anniversary {
    date: String, // "MM-DD" または "YYYY-MM-DD"
    title: String,
    #[serde(default)]
    url: Option<String>,
}

/// C. 記念日に灯す火。anniversaries.json があり今日が一致したら 1日1回 memorial を灯す。
fn spawn_anniversary() {
    tokio::spawn(async move {
        loop {
            let list: Vec<Anniversary> = load("anniversaries.json");
            if !list.is_empty() {
                let now = chrono::Utc::now();
                let md = now.format("%m-%d").to_string();
                let ymd = now.format("%Y-%m-%d").to_string();
                for a in list.iter().filter(|a| a.date == md || a.date == ymd) {
                    let posts: Vec<Post> = load("posts.json");
                    let already = posts.iter().any(|p| {
                        p.kind == "memorial"
                            && p.body.contains(&a.title)
                            && p.created_at.starts_with(ymd.as_str())
                    });
                    if !already {
                        add_post("記念", &a.title, "memorial", a.url.clone());
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
}

/// D. federation。`COMMUNITY_FEDERATION_PEERS="名前|URL,名前|URL"`（URLは相手の
/// /api/community/posts）。相手の新しい薪を relay として取り込む。relay は再 relay
/// しない（ループ防止）。設定が無ければ起動しない。
fn spawn_federation() {
    let raw = std::env::var("COMMUNITY_FEDERATION_PEERS").unwrap_or_default();
    let peers: Vec<(String, String)> = raw
        .split(',')
        .filter_map(|p| p.split_once('|'))
        .map(|(n, u)| (n.trim().to_string(), u.trim().to_string()))
        .filter(|(n, u)| !n.is_empty() && u.starts_with("http"))
        .collect();
    if peers.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            for (name, url) in &peers {
                let Ok(r) = client
                    .get(url)
                    .timeout(std::time::Duration::from_secs(8))
                    .send()
                    .await
                else {
                    continue;
                };
                let Ok(j) = r.json::<serde_json::Value>().await else {
                    continue;
                };
                let Some(arr) = j.get("posts").and_then(|v| v.as_array()) else {
                    continue;
                };
                let mut seen: Vec<String> = load("federation_seen.json");
                for p in arr.iter().take(20) {
                    let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                    if kind == "content" || kind == "relay" {
                        continue; // ピン留め・再relay は取り込まない
                    }
                    let oid = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let key = format!("{name}:{oid}");
                    if oid.is_empty() || seen.contains(&key) {
                        continue;
                    }
                    let author = p.get("author_name").and_then(|v| v.as_str()).unwrap_or("誰か");
                    let body = p.get("body").and_then(|v| v.as_str()).unwrap_or("");
                    let url2 = p
                        .get("url")
                        .and_then(|v| v.as_str())
                        .filter(|s| s.starts_with("http"))
                        .map(|s| s.to_string());
                    add_post(&format!("{name}» {author}"), body, "relay", url2);
                    seen.push(key);
                }
                if seen.len() > 2000 {
                    let drop = seen.len() - 2000;
                    seen.drain(0..drop);
                }
                save("federation_seen.json", &seen);
            }
            tokio::time::sleep(std::time::Duration::from_secs(180)).await;
        }
    });
}

// ── webhook（A. 仕事の鼓動を薪に） ──────────────────────────────────────────
// 外部の実イベント（commit / deploy / sale / ...）を薪としてくべる。
// 共有シークレット COMMUNITY_WEBHOOK_SECRET 一致が必要。未設定なら無効(503)。

#[derive(Deserialize)]
pub struct WebhookBody {
    #[serde(default)]
    text: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    author: Option<String>,
}

fn webhook_kind(k: &str) -> &'static str {
    match k {
        "commit" => "commit",
        "deploy" => "deploy",
        "sale" => "sale",
        "memorial" => "memorial",
        _ => "event",
    }
}

pub async fn webhook_post(headers: HeaderMap, Json(body): Json<WebhookBody>) -> Response {
    let secret = std::env::var("COMMUNITY_WEBHOOK_SECRET").unwrap_or_default();
    if secret.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, "webhook disabled").into_response();
    }
    let provided = headers
        .get("x-tomoshibi-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != secret {
        return (StatusCode::UNAUTHORIZED, "bad secret").into_response();
    }
    let text = body.text.trim();
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, "text required").into_response();
    }
    let kind = webhook_kind(body.kind.as_deref().unwrap_or("event"));
    let author = body
        .author
        .as_deref()
        .map(|s| s.chars().take(40).collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "🛰 自動".to_string());
    let url = body
        .url
        .as_deref()
        .filter(|s| s.starts_with("http"))
        .filter(|s| s.len() <= 500 && !s.contains(['"', '<', '>', ' ']))
        .map(|s| s.to_string());
    add_post(&author, text, kind, url);
    Json(serde_json::json!({"ok": true})).into_response()
}

// ── MCP ────────────────────────────────────────────────────────────────────

fn tool_defs() -> serde_json::Value {
    serde_json::json!([
        {"name":"list_logs","description":"焚き火の薪（最近の投稿）を読む。認証不要。","inputSchema":{"type":"object","properties":{}}},
        {"name":"list_members","description":"火を囲むメンバー一覧。認証不要。","inputSchema":{"type":"object","properties":{}}},
        {"name":"fire_status","description":"焚き火の状態（薪の数＝炎の大きさ）。認証不要。","inputSchema":{"type":"object","properties":{}}},
        {"name":"ignite","description":"焚き火に火をともす（参加表明）。要 Bearer api_token。","inputSchema":{"type":"object","properties":{"message":{"type":"string","description":"一言（省略可）"}}}},
        {"name":"add_log","description":"薪をくべる（今の作業・進捗を一行投稿）。要 Bearer api_token。","inputSchema":{"type":"object","required":["text"],"properties":{"text":{"type":"string"},"url":{"type":"string","description":"関連URL(省略可)"}}}}
    ])
}

pub async fn mcp_get() -> Html<String> {
    let base = base_url();
    Html(format!(
        r#"<!DOCTYPE html><meta charset=utf-8><title>ともしび MCP</title>
<body style="background:#111;color:#eee;font-family:system-ui;max-width:680px;margin:40px auto;padding:0 20px;line-height:1.7">
<h1>🔥 ともしび MCP</h1>
<p>権限のある人が Claude から焚き火を操作する MCP サーバー。読み取りは認証不要、書き込みは <code>Authorization: Bearer &lt;api_token&gt;</code>（<a style="color:#e8a" href="/community/join">/community/join</a> でメール認証して取得）。</p>
<pre style="background:#1a1a1a;padding:14px;border-radius:6px;overflow-x:auto">claude mcp add --transport http tomoshibi {base}/community/mcp --header "Authorization: Bearer &lt;api_token&gt;"</pre>
<p>tools: list_logs / list_members / fire_status（読取） ・ ignite / add_log（書込）</p>
</body>"#
    ))
}

pub async fn mcp_post(headers: HeaderMap, Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let ok = |result: serde_json::Value| {
        Json(serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}))
    };
    let err = |code: i64, msg: &str| {
        Json(serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":msg}}))
    };

    match method {
        "initialize" => {
            let protocol_version = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05")
                .to_string();
            Json(serde_json::json!({"jsonrpc":"2.0","id":id,"result":{
                "protocolVersion": protocol_version,
                "serverInfo":{"name":"tomoshibi","version":"1.0.0"},
                "capabilities":{"tools":{}},
                "instructions":"🪵 ともしび — 消える火が、いちばん残る。\nくべた言葉は10分で消え、でも\"建物\"になって積もる。\n→ add_log に、いま頭にある一行を放り込む。それだけで火は育つ。"
            }}))
        }
        "ping" => ok(serde_json::json!({})),
        "tools/list" => ok(serde_json::json!({"tools": tool_defs()})),
        "tools/call" => {
            let name = req
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let args = req
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let text = |s: String| {
                serde_json::json!({"content":[{"type":"text","text": s}]})
            };

            // 読み取りでも実名・本文は認証必須。未認証はティーザー（人数・件数のみ）。
            let authed = is_authed(&headers);
            match name {
                "list_logs" => {
                    let posts: Vec<Post> = load("posts.json");
                    let n = posts.iter().filter(|p| is_log(p)).count();
                    if !authed {
                        return ok(text(if n == 0 {
                            "まだ薪がありません。".into()
                        } else {
                            format!("🔥 薪が {n} 本くべられています。本文を読むには /community/join でログインしてください。")
                        }));
                    }
                    let mut posts = posts;
                    posts.reverse();
                    posts.truncate(30);
                    let lines: Vec<String> = posts
                        .iter()
                        .map(|p| format!("🔥 {} — {}", p.author_name, p.body))
                        .collect();
                    ok(text(if lines.is_empty() {
                        "まだ薪がありません。".into()
                    } else {
                        lines.join("\n")
                    }))
                }
                "list_members" => {
                    let n = load::<Member>("members.json").len();
                    if !authed {
                        return ok(text(if n == 0 {
                            "（まだ誰もいません）".into()
                        } else {
                            format!("🔥 {n} 人が火を囲んでいます。名前を見るには /community/join でログインしてください。")
                        }));
                    }
                    let ms: Vec<String> = load::<Member>("members.json")
                        .iter()
                        .map(|m| format!("{} ({})", m.name, m.role))
                        .collect();
                    ok(text(if ms.is_empty() { "（まだ誰もいません）".into() } else { ms.join("\n") }))
                }
                "fire_status" => {
                    let n = load::<Post>("posts.json").len();
                    let mem = load::<Member>("members.json").len();
                    let blds = load::<Building>("buildings.json").len();
                    let (alive, remain) = fire_state();
                    let s = if alive {
                        format!("🔥 焚き火 燃焼中。薪 {n} 本・人 {mem}・残り {}分{}秒で消えます（薪をくべると延びる）。建物 {blds} 棟。", remain / 60, remain % 60)
                    } else {
                        format!("🔥 火は消えています。薪をくべると新しい焚き火が始まります。人 {mem}・建物 {blds} 棟。")
                    };
                    ok(text(s))
                }
                "ignite" | "add_log" => {
                    // write — require valid Bearer api_token (no cookie fallback → no CSRF)
                    let tok = bearer(&headers).unwrap_or_default();
                    let Some(m) = member_by_token(&tok) else {
                        return err(-32001, "unauthorized: 有効な Bearer api_token が必要です。/community/join でメール認証して取得してください。");
                    };
                    if name == "ignite" {
                        let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        let body = if msg.is_empty() {
                            format!("{} が火をともした🔥", m.name)
                        } else {
                            format!("{} が火をともした🔥 — {}", m.name, msg)
                        };
                        add_post(&m.name, &body, "ignite", None);
                        ok(text("火がともりました🔥 焚き火に刻まれました。".into()))
                    } else {
                        let t = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        if t.trim().is_empty() {
                            return err(-32602, "text が必要です");
                        }
                        let url = args
                            .get("url")
                            .and_then(|v| v.as_str())
                            .filter(|s| s.starts_with("https://") || s.starts_with("http://"))
                            .filter(|s| s.len() <= 500 && !s.contains(['"', '<', '>', ' ']))
                            .map(|s| s.to_string());
                        add_post(&m.name, t, "log", url);
                        ok(text("薪をくべました🔥 炎が大きくなりました。".into()))
                    }
                }
                _ => err(-32601, "unknown tool"),
            }
        }
        _ => err(-32601, "method not found"),
    }
}

// ── pages ────────────────────────────────────────────────────────────────

const PAGE_CSS: &str = r#"
*{box-sizing:border-box;margin:0;padding:0}
body{color:#f3ede2;font-family:'Helvetica Neue',Arial,sans-serif;min-height:100vh;
 background:
  radial-gradient(58% 46% at 50% 104%, rgba(255,170,70,.36), transparent 70%),
  radial-gradient(90% 70% at 50% 100%, rgba(232,101,31,.26), transparent 72%),
  radial-gradient(120% 90% at 50% 30%, transparent 40%, rgba(0,0,0,.5) 100%),
  #08080a;
 background-attachment:fixed}
.center{display:flex;align-items:center;justify-content:center;padding:40px 20px}
.card{max-width:560px;width:100%;background:rgba(255,255,255,.03);border:1px solid rgba(255,255,255,.08);border-radius:14px;padding:32px 28px}
.fire-emoji{font-size:44px;margin-bottom:10px}
h1{font-size:22px;color:#f0c987;margin-bottom:8px;font-weight:800}
.sub{color:rgba(243,237,226,.55);font-size:14px;line-height:1.7;margin-bottom:22px}
.label{font-size:11px;letter-spacing:.1em;text-transform:uppercase;color:#e8651f;margin:18px 0 6px}
.cmd,.say{background:#161616;border:1px solid #2a2a2a;border-radius:6px;padding:13px 16px;font-size:13px;color:#f0c987;cursor:pointer;position:relative;overflow-x:auto;white-space:nowrap;margin-bottom:6px}
.say{color:rgba(243,237,226,.8);font-size:14px}
.cmd{font-family:'JetBrains Mono',monospace}
.hint{position:absolute;right:10px;top:50%;transform:translateY(-50%);font-size:10px;color:rgba(243,237,226,.3)}
.watch{display:inline-block;margin-top:22px;color:#f0c987;border:1px solid rgba(240,201,135,.25);padding:11px 20px;border-radius:6px;text-decoration:none}
input{width:100%;background:#161616;border:1px solid #2a2a2a;border-radius:6px;padding:13px 16px;color:#f3ede2;font-size:15px;margin-bottom:8px}
button{width:100%;background:#e8651f;border:none;border-radius:6px;padding:14px;color:#fff;font-size:15px;font-weight:700;cursor:pointer}
"#;

pub async fn join_page(headers: HeaderMap, Query(q): Query<LangQ>) -> Html<String> {
    let base = base_url();
    let ja = pick_lang(&headers, &q) == "ja";
    let (title, h1, sub, name_ph, email_ph, btn, have_key, watch, sending, sent, failed) = if ja {
        (
            "ともしび — 火をともす",
            "焚き火に火をともす",
            "あなたが作業すると、その熱が炎になる。<br>まずメールで認証して、Claude から焚き火を操作する権限キーを受け取ります。誰でも参加できます。",
            "名前（例: 濱田 優貴）",
            "メールアドレス",
            "🔥 ログインリンクを送る",
            "すでに権限キーを持っている人",
            "🔥 焚き火を見る",
            "送信中…",
            "✓ メールを送りました。リンクを開くと火がともります。",
            "送信に失敗しました。",
        )
    } else {
        (
            "Tomoshibi — Light a fire",
            "Light the campfire",
            "When you work, that warmth becomes the flame.<br>Verify by email to get an api key that lets Claude tend the fire. Anyone can join.",
            "Name (e.g. Yuki Hamada)",
            "Email address",
            "🔥 Send login link",
            "Already have an api key?",
            "🔥 See the fire",
            "Sending…",
            "✓ Email sent. Open the link to light the fire.",
            "Failed to send.",
        )
    };
    let lang_attr = if ja { "ja" } else { "en" };
    Html(format!(
        r#"<!DOCTYPE html><html lang={lang_attr}><head><meta charset=utf-8>
<meta name=viewport content="width=device-width,initial-scale=1">
<title>{title}</title><style>{css}</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body class=center>
<div class=card>
<div class=fire-emoji>🔥</div>
<h1>{h1}</h1>
<p class=sub>{sub}</p>
<form onsubmit="return go(event)">
<input id=name placeholder="{name_ph}" maxlength=40>
<input id=email type=email placeholder="{email_ph}" required>
<button type=submit>{btn}</button>
</form>
<p id=msg class=sub style="margin-top:14px"></p>
<div class=label>{have_key}</div>
<div class=cmd onclick="cp(this,this.dataset.c)" data-c='claude mcp add --transport http tomoshibi {base}/community/mcp --header "Authorization: Bearer <api_token>"'>claude mcp add --transport http tomoshibi {base}/community/mcp --header "Authorization: Bearer &lt;api_token&gt;"<span class=hint>コピー</span></div>
<a class=watch href="/community">{watch}</a>
</div>
<script>
function cp(el,t){{navigator.clipboard.writeText(t).catch(()=>{{}});var h=el.querySelector('.hint');if(h){{var o=h.textContent;h.textContent='✓';setTimeout(()=>h.textContent=o,1500)}}}}
async function go(e){{e.preventDefault();var msg=document.getElementById('msg');msg.textContent='{sending}';
try{{var r=await fetch('/api/community/auth/request',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{email:document.getElementById('email').value,name:document.getElementById('name').value}})}});
if(r.ok){{msg.innerHTML='{sent}';}}else{{msg.textContent='{failed}';}}}}catch(_){{msg.textContent='{failed}';}}return false;}}
</script>
</body></html>"#,
        css = PAGE_CSS,
        base = base,
    ))
}

pub async fn page() -> Html<String> {
    Html(BONFIRE_HTML.to_string())
}

/// 消えた焚き火が積もる建物の一覧。append-only に残り続ける。
pub async fn buildings_page(headers: HeaderMap, Query(q): Query<LangQ>) -> Html<String> {
    {
        let _g = STORE_LOCK.lock().unwrap();
        let _ = archive_locked();
    }
    let ja = pick_lang(&headers, &q) == "ja";
    let (lang_attr, title, page_h1, page_sub, nav_fire, nav_join, empty) = if ja {
        (
            "ja", "ともしび — 建物", "🏛 建物",
            "消えた焚き火の薪は、ここに建物として永遠に残ります。",
            "🔥 焚き火へ", "火をともす",
            "まだ建物はありません。焚き火が10分消えると、その薪がここに建物として残ります。",
        )
    } else {
        (
            "en", "Tomoshibi — Buildings", "🏛 Buildings",
            "Logs from fires that went out remain here forever as buildings.",
            "🔥 To the fire", "Light a fire",
            "No buildings yet. When a fire is quiet for 10 minutes, its logs remain here as a building.",
        )
    };
    let authed = is_authed(&headers);
    let mut bs: Vec<Building> = load("buildings.json");
    bs.reverse();
    let cards: String = if bs.is_empty() {
        format!("<div class=empty>{empty}</div>")
    } else {
        let (lbl_fire, lbl_logs) = if ja { ("焚き火", "薪") } else { ("Fire", "logs") };
        bs.iter().map(|b| {
            // 未ログインには実名・本文を出さない。規模（薪数・人数・期間）だけのティーザー。
            let (members, logs) = if authed {
                let m = b.members.iter().map(|m| esc(m)).collect::<Vec<_>>().join(if ja {"・"} else {", "});
                let l = b.posts.iter().rev().take(6).map(|p| {
                    format!("<div class=bl>🪵 <b>{}</b> {}</div>", esc(&p.author_name), esc(&p.body))
                }).collect::<Vec<_>>().join("");
                (m, l)
            } else {
                let m = if ja { format!("{} 人", b.members.len()) } else { format!("{} people", b.members.len()) };
                let teaser = if ja { "ログインすると薪が読めます" } else { "Log in to read the logs" };
                let l = format!("<div class=bl><a style=\"color:#f0c987\" href=\"/community/join\">🔒 {teaser}</a></div>");
                (m, l)
            };
            format!(
                "<div class=bld><div class=bhead><span class=bicon>🏛</span><div><div class=bt>{lbl_fire} #{id}</div><div class=bm>{lbl_logs} {n} ・ {members}</div></div></div><div class=blogs>{logs}</div><div class=bdate>{start} 〜 {end}</div></div>",
                id = &b.id[..6.min(b.id.len())], n = b.log_count, members = members, logs = logs,
                start = esc(&b.started_at[..10.min(b.started_at.len())]),
                end = esc(&b.ended_at[..10.min(b.ended_at.len())]),
            )
        }).collect()
    };
    Html(format!(
        r#"<!DOCTYPE html><html lang={lang_attr}><head><meta charset=utf-8>
<meta name=viewport content="width=device-width,initial-scale=1"><meta name=robots content=noindex>
<title>{title}</title>
<style>{css}
.bwrap{{max-width:680px;margin:0 auto;padding:40px 18px 80px}}
.bld{{background:rgba(255,255,255,.03);border:1px solid rgba(244,205,139,.14);border-radius:12px;padding:18px;margin-bottom:14px}}
.bhead{{display:flex;gap:12px;align-items:center;margin-bottom:10px}}
.bicon{{font-size:30px}} .bt{{color:#f4cd8b;font-weight:800}} .bm{{font-size:12px;color:rgba(243,237,226,.5)}}
.bl{{font-size:13px;color:rgba(243,237,226,.85);padding:4px 0;border-top:1px solid rgba(255,255,255,.05)}}
.bdate{{font-size:11px;color:rgba(243,237,226,.3);margin-top:8px}}
.empty{{color:rgba(243,237,226,.4);text-align:center;font-size:13px;margin-top:40px}}
.bnav{{text-align:center;margin:10px 0 24px}} .bnav a{{color:#f0c987;text-decoration:none;margin:0 10px;font-size:14px}}
</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body>
<div class=bwrap>
<h1 style="text-align:center;color:#f4cd8b;font-size:24px;font-weight:900;margin-bottom:6px">{page_h1}</h1>
<p style="text-align:center;color:rgba(243,237,226,.5);font-size:13px;line-height:1.7;margin-bottom:8px">{page_sub}</p>
<div class=bnav><a href="/community">{nav_fire}</a><a href="/community/join">{nav_join}</a></div>
{cards}
</div></body></html>"#,
        css = PAGE_CSS, cards = cards
    ))
}

/// リアルな焚き火（Canvasパーティクル）＋薪フィード。投稿数で炎が大きくなる。
const BONFIRE_HTML: &str = r##"<!DOCTYPE html><html lang=ja><head><meta charset=utf-8>
<meta name=viewport content="width=device-width,initial-scale=1">
<title>ともしび — 焚き火</title>
<meta name=robots content="noindex">
<style>
*{box-sizing:border-box;margin:0;padding:0}
html,body{height:100%}
body{background:#08080a;color:#f3ede2;font-family:'Helvetica Neue',Arial,sans-serif;overflow-x:hidden}
#fire{position:fixed;inset:0;width:100%;height:100%;z-index:0;display:block}
.vignette{position:fixed;inset:0;z-index:1;pointer-events:none;background:radial-gradient(120% 90% at 50% 78%,transparent 30%,rgba(0,0,0,.55) 100%)}
.wrap{position:relative;z-index:2;max-width:680px;margin:0 auto;padding:42px 18px 120px;min-height:100%}
.top{text-align:center;margin-bottom:30px}
.top h1{font-size:26px;font-weight:900;color:#f4cd8b;letter-spacing:.04em;text-shadow:0 2px 30px rgba(232,101,31,.5)}
.top .sub{color:rgba(243,237,226,.5);font-size:13px;margin-top:6px;line-height:1.7}
.stat{display:inline-flex;gap:16px;margin-top:14px;font-size:12px;color:rgba(243,237,226,.6)}
.stat b{color:#f4cd8b}
.trend{margin-left:5px;font-size:10.5px;font-weight:700;letter-spacing:.02em;vertical-align:1px}
.trend.up{color:#ffae5a}
.trend.down{color:#7ec8ff}
.trend.flat{color:rgba(243,237,226,.32)}
.feed{margin-top:min(340px,46vh)}
.log{background:rgba(20,16,14,.66);backdrop-filter:blur(6px);border:1px solid rgba(244,205,139,.12);border-radius:10px;padding:13px 16px;margin-bottom:10px;animation:rise .6s ease}
@keyframes rise{from{opacity:0;transform:translateY(12px)}to{opacity:1;transform:none}}
.log .who{font-size:12px;color:#f4cd8b;font-weight:700}
.log .who .k{font-size:10px;color:#e8651f;margin-left:6px;border:1px solid rgba(232,101,31,.4);border-radius:4px;padding:1px 5px}
.log .who .k[data-kind=deploy]{color:#7ee0a0;border-color:rgba(126,224,160,.45)}
.log .who .k[data-kind=commit]{color:#7ec8ff;border-color:rgba(126,200,255,.45)}
.log .who .k[data-kind=sale]{color:#ffd479;border-color:rgba(255,212,121,.5)}
.log .who .k[data-kind=relay]{color:#c89bff;border-color:rgba(200,155,255,.45)}
.log .who .k[data-kind=memorial]{color:#ff9bbf;border-color:rgba(255,155,191,.5)}
.log .who .k[data-kind=event]{color:#f0c987;border-color:rgba(240,201,135,.4)}
.log .body{font-size:14px;color:rgba(243,237,226,.92);margin-top:4px;line-height:1.65;white-space:pre-wrap;word-break:break-word}
.log a{color:#f0c987}
.log .t{font-size:11px;color:rgba(243,237,226,.32);margin-top:5px}
.countdown{margin-top:10px;font-size:12px;color:rgba(243,237,226,.45);min-height:16px}
.countdown.low{color:#ffae5a}
.join{position:fixed;left:0;right:0;bottom:0;z-index:3;text-align:center;padding:16px;background:linear-gradient(transparent,rgba(8,8,10,.9) 40%)}
.join a{display:inline-block;background:#e8651f;color:#fff;text-decoration:none;font-weight:700;padding:13px 26px;border-radius:8px;box-shadow:0 6px 30px rgba(232,101,31,.5)}
.join a.alt{background:transparent;color:#f0c987;border:1px solid rgba(240,201,135,.3);box-shadow:none;margin-left:8px}
.empty{color:rgba(243,237,226,.55);text-align:center;font-size:13px;line-height:1.8;margin-top:min(340px,46vh)}
.empty a{color:#f0c987}
.presence{display:inline-block;margin-top:12px;font-size:12.5px;color:#ffd9a8;background:rgba(232,101,31,.14);border:1px solid rgba(232,101,31,.42);border-radius:999px;padding:5px 14px;text-decoration:none}
.presence:hover{background:rgba(232,101,31,.24)}
.ember{position:fixed;inset:0;z-index:5;display:none;flex-direction:column;align-items:center;justify-content:center;text-align:center;background:rgba(6,6,8,.82);backdrop-filter:blur(3px);padding:30px;animation:fade 1.2s ease}
.ember.show{display:flex}
@keyframes fade{from{opacity:0}to{opacity:1}}
.ember .big{font-size:50px;filter:grayscale(.4) brightness(.7)}
.ember h2{color:#f4cd8b;font-size:22px;font-weight:900;margin:14px 0 6px}
.ember p{color:rgba(243,237,226,.6);font-size:14px;line-height:1.7;max-width:340px}
.ember a{display:inline-block;margin-top:22px;background:#6a4a2a;color:#fff;text-decoration:none;font-weight:700;padding:13px 26px;border-radius:8px}
</style><script defer src="https://enabler-analytics.fly.dev/t.js"></script></head>
<body>
<canvas id=fire></canvas>
<div class=vignette></div>
<div class=wrap>
  <div class=top>
    <h1>🔥 ともしび</h1>
    <div class=sub id=sub></div>
    <a class=presence id=presence href="/room/tomoshibi" hidden></a>
    <a class=presence id=vpresence href="https://koe.live/app?room=atsmwtf" target=_blank rel=noopener hidden></a>
    <div class=stat><span><span id=l-wood>薪</span> <b id=cn>0</b><span class=trend id=tr-wood></span></span><span><span id=l-people>人</span> <b id=mn>0</b><span class=trend id=tr-people></span></span><span><span id=l-bld>建物</span> <b id=bn>0</b><span class=trend id=tr-bld></span></span></div>
    <div id=countdown class=countdown></div>
  </div>
  <div id=feed class=feed></div>
</div>
<div id=ember class=ember>
  <div class=big>🏛</div>
  <h2 id=em-title>火が消えました</h2>
  <p id=em-text></p>
  <a id=em-link href="/community/buildings">🏛 建物を見る</a>
</div>
<div class=join>
  <a id=j-room href="/room/tomoshibi">🔴 焚き火ルーム</a>
  <a class=alt id=j-voice href="https://koe.live/app?room=atsmwtf" target=_blank rel=noopener>🎙 声</a>
  <a class=alt id=j-join href="/community/join">🔥 火をともす</a>
  <a class=alt id=j-bld href="/community/buildings">🏛 建物</a>
</div>
<script>
// ── realistic bonfire (canvas particle system) ──
const cv=document.getElementById('fire'),ctx=cv.getContext('2d');
let W,H,DPR,intensity=1,alive=true;
function resize(){DPR=Math.min(devicePixelRatio||1,2);W=cv.width=innerWidth*DPR;H=cv.height=innerHeight*DPR;cv.style.width=innerWidth+'px';cv.style.height=innerHeight+'px';}
addEventListener('resize',resize);resize();
function baseXY(){return {x:W*0.5, y:H*0.74};}
const flames=[],embers=[],smoke=[];
function rnd(a,b){return a+Math.random()*(b-a)}
function spawnFlame(){const b=baseXY();const spread=rnd(-1,1);flames.push({x:b.x+spread*28*DPR,y:b.y+rnd(-4,8)*DPR,vx:spread*rnd(.1,.5)*DPR,vy:-rnd(2.6,4.6)*DPR*(0.8+intensity*0.12),life:1,decay:rnd(.012,.024),size:rnd(26,46)*DPR*(0.85+intensity*0.05),hue:rnd(18,42)});}
function spawnEmber(){const b=baseXY();embers.push({x:b.x+rnd(-22,22)*DPR,y:b.y+rnd(-10,6)*DPR,vx:rnd(-.6,.6)*DPR,vy:-rnd(1.4,3.4)*DPR,life:1,decay:rnd(.006,.014),size:rnd(1,2.6)*DPR,sway:rnd(0,6.28),sp:rnd(.02,.06)});}
function spawnSmoke(){const b=baseXY();smoke.push({x:b.x+rnd(-16,16)*DPR,y:b.y-60*DPR,vx:rnd(-.3,.3)*DPR,vy:-rnd(.5,1.1)*DPR,life:1,decay:rnd(.004,.008),size:rnd(40,80)*DPR});}
function logsGfx(){const b=baseXY();ctx.save();ctx.translate(b.x,b.y+10*DPR);ctx.globalCompositeOperation='source-over';
  // glowing wood logs
  for(let i=-1;i<=1;i++){ctx.save();ctx.rotate(i*0.5);
   const grd=ctx.createLinearGradient(0,-8*DPR,0,8*DPR);grd.addColorStop(0,'#3a2418');grd.addColorStop(.5,'#5a341f');grd.addColorStop(1,'#241208');
   ctx.fillStyle=grd;ctx.beginPath();ctx.roundRect(-62*DPR,-7*DPR,124*DPR,14*DPR,7*DPR);ctx.fill();
   // ember glow on log
   ctx.fillStyle='rgba(232,101,31,'+(0.16+0.08*Math.sin(Date.now()/200+i))+')';ctx.beginPath();ctx.roundRect(-50*DPR,-5*DPR,100*DPR,10*DPR,5*DPR);ctx.fill();
   ctx.restore();}
  ctx.restore();}
let t=0;
function frame(){t++;
  // flicker intensity
  const flick=0.9+Math.sin(t*0.3)*0.06+Math.random()*0.08;
  ctx.globalCompositeOperation='source-over';ctx.fillStyle='rgba(8,8,10,0.28)';ctx.fillRect(0,0,W,H);
  const b=baseXY();
  // ground glow
  const gg=ctx.createRadialGradient(b.x,b.y,4*DPR,b.x,b.y,260*DPR*(0.8+intensity*0.06));
  gg.addColorStop(0,'rgba(255,170,70,'+(0.5*flick)+')');gg.addColorStop(.4,'rgba(232,101,31,'+(0.22*flick)+')');gg.addColorStop(1,'rgba(232,101,31,0)');
  ctx.globalCompositeOperation='lighter';ctx.fillStyle=gg;ctx.beginPath();ctx.arc(b.x,b.y,260*DPR,0,6.29);ctx.fill();
  logsGfx();
  // spawn (only while the fire is alive; dying fire shrinks toward 0)
  if(alive&&intensity>0.05){
    const fcount=Math.max(1,Math.round((6+intensity*2.2)*flick*Math.min(1,intensity)));
    for(let i=0;i<fcount;i++)spawnFlame();
    if(t%2==0)for(let i=0;i<1+intensity*0.5;i++)spawnEmber();
    if(t%3==0)spawnSmoke();
  } else if(t%4==0){spawnEmber();spawnSmoke();}
  // smoke (under flames, soft dark)
  ctx.globalCompositeOperation='source-over';
  for(let i=smoke.length-1;i>=0;i--){const p=smoke[i];p.x+=p.vx;p.y+=p.vy;p.vy*=0.99;p.life-=p.decay;p.size*=1.012;if(p.life<=0){smoke.splice(i,1);continue;}
    ctx.fillStyle='rgba(30,26,28,'+(p.life*0.10)+')';ctx.beginPath();ctx.arc(p.x,p.y,p.size,0,6.29);ctx.fill();}
  // flames (additive)
  ctx.globalCompositeOperation='lighter';
  for(let i=flames.length-1;i>=0;i--){const p=flames[i];p.x+=p.vx;p.y+=p.vy;p.vy*=0.985;p.vx*=0.98;p.life-=p.decay;p.size*=0.965;if(p.life<=0||p.size<2){flames.splice(i,1);continue;}
    const a=p.life;const sat=70;const light=40+30*a;
    const g=ctx.createRadialGradient(p.x,p.y,0,p.x,p.y,p.size);
    g.addColorStop(0,'hsla('+(p.hue+18)+','+sat+'%,'+(light+22)+'%,'+(a*0.9)+')');
    g.addColorStop(.45,'hsla('+p.hue+','+sat+'%,'+light+'%,'+(a*0.55)+')');
    g.addColorStop(1,'hsla('+(p.hue-10)+',90%,30%,0)');
    ctx.fillStyle=g;ctx.beginPath();ctx.arc(p.x,p.y,p.size,0,6.29);ctx.fill();}
  // embers (bright, flicker, sway)
  for(let i=embers.length-1;i>=0;i--){const p=embers[i];p.sway+=p.sp;p.x+=p.vx+Math.sin(p.sway)*0.5*DPR;p.y+=p.vy;p.vy*=0.992;p.life-=p.decay;if(p.life<=0){embers.splice(i,1);continue;}
    const a=p.life*(0.6+0.4*Math.sin(p.sway*3));ctx.fillStyle='rgba(255,'+(160+Math.floor(60*a))+',80,'+a+')';ctx.beginPath();ctx.arc(p.x,p.y,p.size,0,6.29);ctx.fill();}
  requestAnimationFrame(frame);}
requestAnimationFrame(frame);

// ── i18n (global-first: English unless browser/?lang= says Japanese) ──
const STR={
  ja:{
    docTitle:'ともしび — 焚き火',
    sub:'作業が薪になる、火を囲む場所。<br>見る人・薪をくべる人・集まる人、どれでもいい。まず火を見て、よかったら火をともそう。',
    wood:'薪',people:'人',bld:'建物',
    roomEmpty:'🔴 焚き火ルームはいま静か — 開いて待つ',
    roomN:n=>'🔴 焚き火ルームにいま '+n+'人 — 加わる',
    voiceEmpty:'🎙 声の焚き火はいま静か — マイクひとつで火にあたる',
    voiceN:n=>'🎙 いま '+n+'人が声の火のそばに — 加わる',
    jVoice:'🎙 声で繋がる',
    burning:s=>'あと '+s+' で火が消えます（薪をくべると延びる）',
    uninit:'まだ火がついていません — あなたの火をともそう',
    outNow:'火は消えました',
    emTitle:'火が消えました',
    emText:'10分、薪がくべられず焚き火は消えました。<br>薪は建物となって永遠に残ります。',
    emLink:'🏛 建物を見る',
    jRoom:'🔴 焚き火ルーム',jJoin:'🔥 火をともす',jBld:'🏛 建物',
    emptyFeed:'まだ薪がありません。<br><a href="/community/join">火をともして</a>、最初の薪をくべよう。',
    kinds:{ignite:'着火',content:'作品',commit:'commit',deploy:'deploy',sale:'売上',event:'イベント',relay:'中継',memorial:'記念'},
    now:'たった今',min:'分前',hr:'時間前',day:'日前',
    trendTitle:'直近24時間 vs その前の24時間（▲増えた / ▼減った）',
  },
  en:{
    docTitle:'Tomoshibi — Campfire',
    sub:'A campfire where your work becomes firewood.<br>Watch, add a log, or gather — any of these. Look at the fire first, then light yours if you like.',
    wood:'Logs',people:'People',bld:'Buildings',
    roomEmpty:'🔴 The fire room is quiet — open it and wait',
    roomN:n=>'🔴 '+n+' by the fire now — join',
    voiceEmpty:'🎙 The voice fire is quiet — join with just a mic',
    voiceN:n=>'🎙 '+n+' talking by the fire — join',
    jVoice:'🎙 Voice',
    burning:s=>'fire goes out in '+s+' (add a log to extend)',
    uninit:'No fire yet — light yours',
    outNow:'The fire went out',
    emTitle:'The fire went out',
    emText:'No logs for 10 minutes, so the fire went out.<br>Its logs remain forever as a building.',
    emLink:'🏛 See buildings',
    jRoom:'🔴 Fire room',jJoin:'🔥 Light a fire',jBld:'🏛 Buildings',
    emptyFeed:'No logs yet.<br><a href="/community/join">Light a fire</a> and add the first log.',
    kinds:{ignite:'lit',content:'work',commit:'commit',deploy:'deploy',sale:'sale',event:'event',relay:'relay',memorial:'memorial'},
    now:'just now',min:'m ago',hr:'h ago',day:'d ago',
    trendTitle:'last 24h vs the 24h before (▲ up / ▼ down)',
  },
};
const qLang=new URLSearchParams(location.search).get('lang');
const lang=(qLang==='ja'||qLang==='en')?qLang:((navigator.language||'en').toLowerCase().startsWith('ja')?'ja':'en');
const T=STR[lang];
(function applyI18n(){
  document.documentElement.lang=lang;document.title=T.docTitle;
  document.getElementById('sub').innerHTML=T.sub;
  document.getElementById('l-wood').textContent=T.wood;
  document.getElementById('l-people').textContent=T.people;
  document.getElementById('l-bld').textContent=T.bld;
  document.getElementById('em-title').textContent=T.emTitle;
  document.getElementById('em-text').innerHTML=T.emText;
  document.getElementById('em-link').textContent=T.emLink;
  document.getElementById('j-room').textContent=T.jRoom;
  document.getElementById('j-voice').textContent=T.jVoice;
  document.getElementById('j-join').textContent=T.jJoin;
  document.getElementById('j-bld').textContent=T.jBld;
})();

// ── feed ──
function esc(s){return (s||'').replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]))}
function linkify(s){return esc(s).replace(/(https?:\/\/[^\s]+)/g,'<a href="$1" target=_blank rel=noopener>$1</a>')}
function ago(iso){const d=(Date.now()-new Date(iso).getTime())/1000;if(d<60)return T.now;if(d<3600)return Math.floor(d/60)+T.min;if(d<86400)return Math.floor(d/3600)+T.hr;return Math.floor(d/86400)+T.day;}
let remain=0,ttl=600,nLogs=0,sawFire=false;
function fmt(s){s=Math.max(0,s|0);return lang==='ja'?Math.floor(s/60)+'分'+('0'+(s%60)).slice(-2)+'秒':Math.floor(s/60)+':'+('0'+(s%60)).slice(-2)}
function tick(){const cd=document.getElementById('countdown');const em=document.getElementById('ember');
  if(remain>0)remain--;
  alive=remain>0;
  if(alive)sawFire=true;
  const ratio=ttl>0?Math.min(1,remain/ttl):0;
  // fire size = wood count, but fades as the 10-min window runs out
  intensity=alive?Math.max(0.3,intensity_base*(0.35+0.65*ratio)):0;
  if(alive){
    em.classList.remove('show');cd.classList.toggle('low',remain<120);
    cd.textContent=T.burning(fmt(remain));
  } else if(nLogs===0 && !sawFire){
    // 未着火 — 穏やかに誘う（消火オーバーレイは出さない）
    em.classList.remove('show');cd.classList.remove('low');
    cd.textContent=T.uninit;
  } else {
    // 見ている前で消えた／薪が尽きた
    em.classList.add('show');cd.classList.remove('low');cd.textContent=T.outNow;
  }
}
let intensity_base=1,roomPeople=0,voicePeople=0;
// 炎の大きさ = 薪の数 + 焚き火ルームの在室人数 + 声の火の在室人数（人が集まると物理的に火が育つ）
function recompIntensity(){intensity_base=Math.max(1,Math.min(12,1+nLogs*0.5+roomPeople*0.9+voicePeople*1.2));}
// ── voice presence: 声の焚き火 (koe.live / ATSUMEと同じ部屋) にいま何人いるか ──
async function loadVoice(){const el=document.getElementById('vpresence');try{
  const d=await (await fetch('/api/community/koe',{cache:'no-store'})).json();
  voicePeople=d.count||0;el.textContent=voicePeople>0?T.voiceN(voicePeople):T.voiceEmpty;el.hidden=false;
  recompIntensity();
}catch(_){el.hidden=true;}}
// ── presence: 焚き火ルームにいま何人いるか（空でも"閉店中"でなく"静か"に見せる） ──
async function loadPresence(){const el=document.getElementById('presence');try{
  const d=await (await fetch('/api/room/tomoshibi/presence',{cache:'no-store'})).json();
  roomPeople=d.count||0;el.textContent=roomPeople>0?T.roomN(roomPeople):T.roomEmpty;el.hidden=false;
  recompIntensity();
}catch(_){el.hidden=true;}}
const lockedTeaser=lang==='ja'?'🔒 ログインして読む':'🔒 Log in to read';
// 増えた(▲)か減った(▼)かを数字の横に出す。delta=直近24h − その前の24h。
function setTrend(id,delta){const el=document.getElementById(id);if(!el)return;
  const dn=delta|0;el.title=T.trendTitle;
  if(dn>0){el.className='trend up';el.textContent='▲+'+dn;}
  else if(dn<0){el.className='trend down';el.textContent='▼'+dn;}
  else{el.className='trend flat';el.textContent='→0';}}
async function load(){try{const r=await fetch('/api/community/posts');const d=await r.json();
  const n=d.count||0;document.getElementById('cn').textContent=n;
  document.getElementById('bn').textContent=d.buildings||0;
  if(d.trend){setTrend('tr-wood',d.trend.wood&&d.trend.wood.delta);setTrend('tr-people',d.trend.people&&d.trend.people.delta);setTrend('tr-bld',d.trend.buildings&&d.trend.buildings.delta);}
  nLogs=n;recompIntensity();
  ttl=d.ttl_secs||600;remain=d.remain_secs||0;if(d.fire_alive)sawFire=true;tick();
  // 人数: 認証済みは members 配列、未認証は count（実名は出ない）
  const m=await (await fetch('/api/community/members')).json();document.getElementById('mn').textContent=(m.count!=null?m.count:(m.members||[]).length);
  const feed=document.getElementById('feed');
  if(!d.posts||!d.posts.length){feed.innerHTML='<div class=empty>'+T.emptyFeed+'</div>';return;}
  feed.innerHTML=d.posts.map(p=>{const kl=T.kinds[p.kind];const k=kl?'<span class=k data-kind="'+esc(p.kind)+'">'+kl+'</span>':'';
    // 未ログイン(teaser)では実名・本文・URLを出さない。種別と時刻＋ログイン導線のみ。
    if(p.teaser||p.body===undefined){
      return '<div class=log><div class=who>🪵'+k+'</div><div class=body><a href="/community/join" style="color:#f0c987">'+lockedTeaser+'</a></div><div class=t>'+ago(p.created_at)+'</div></div>';
    }
    const u=p.url?'<div class=body><a href="'+esc(p.url)+'" target=_blank rel=noopener>'+esc(p.url)+'</a></div>':'';
    return '<div class=log><div class=who>'+esc(p.author_name)+k+'</div><div class=body>'+linkify(p.body)+'</div>'+u+'<div class=t>'+ago(p.created_at)+'</div></div>';}).join('');
}catch(e){}}
load();loadPresence();loadVoice();setInterval(load,8000);setInterval(loadPresence,15000);setInterval(loadVoice,20000);setInterval(tick,1000);
</script>
<!-- ── 隠し導線: 10分滞在で「MU?」が現れ、ふわふわ動く。一度押すと二度と出ない ── -->
<a id=mufes href="https://wearmu.com/fest">MU?</a>
<style>
#mufes{position:fixed;z-index:6;left:50%;top:42%;display:none;opacity:0;
  font-family:'Helvetica Neue',Arial,sans-serif;font-weight:900;font-size:30px;letter-spacing:.02em;
  color:#ffd9a8;text-decoration:none;cursor:pointer;
  text-shadow:0 0 18px rgba(232,101,31,.8),0 0 42px rgba(232,101,31,.45);
  background:rgba(232,101,31,.10);border:1px solid rgba(244,205,139,.35);border-radius:999px;padding:9px 20px;
  -webkit-backdrop-filter:blur(2px);backdrop-filter:blur(2px);
  transition:left 3s ease,top 3s ease,opacity 1.2s ease}
#mufes.show{display:block;opacity:1;animation:mupulse 2.4s ease-in-out infinite}
#mufes:hover{color:#fff;background:rgba(232,101,31,.22)}
@keyframes mupulse{0%,100%{transform:translate(-50%,-50%) scale(.88)}50%{transform:translate(-50%,-50%) scale(1.18)}}
</style>
<script>
(function(){
  try{if(localStorage.getItem('muFesHintDone'))return;}catch(_){}
  var el=document.getElementById('mufes');if(!el)return;
  function wander(){var mx=14,my=20;el.style.left=(mx+Math.random()*(100-2*mx))+'%';el.style.top=(my+Math.random()*(100-2*my))+'%';}
  el.addEventListener('click',function(){try{localStorage.setItem('muFesHintDone','1');}catch(_){}});
  setTimeout(function(){el.classList.add('show');wander();setInterval(wander,5200);},600000);
}());
</script>
</body></html>"##;
