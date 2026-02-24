use askama::Template;
use axum::extract::State;
use std::sync::Arc;

use crate::AppState;

// --- Data structures ---

#[allow(dead_code)]
pub struct Project {
    pub title: &'static str,
    pub description: &'static str,
    pub href: &'static str,
    pub badge: Option<&'static str>,
    pub features: Vec<&'static str>,
    pub icon_svg: &'static str,
    pub color: &'static str,
}

pub struct TimelineItem {
    pub year: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub description: &'static str,
    pub link: Option<&'static str>,
    pub highlight: bool,
}

pub struct SocialLink {
    pub name: &'static str,
    pub href: &'static str,
    pub svg: &'static str,
}

// --- Template ---

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    lang: &'static str,
    title: String,
    description: String,
    base_url: String,
    canonical_url: String,
    og_image: String,
    // Hero
    name_display: String,
    name_sub: String,
    roles: Vec<String>,
    subtitle: String,
    // Projects
    projects_title: String,
    projects_subtitle: String,
    projects: Vec<Project>,
    // Timeline
    timeline_title: String,
    timeline_subtitle: String,
    timeline_items: Vec<TimelineItem>,
    // Social
    social_links: Vec<SocialLink>,
    // Footer
    footer_description: String,
    copyright_year: i32,
    // i18n
    alt_lang_url: String,
    alt_lang_label: String,
    cta_label: String,
    view_service_label: String,
}

fn social_links() -> Vec<SocialLink> {
    vec![
        SocialLink {
            name: "X (Twitter)",
            href: "https://x.com/yukihamada",
            svg: r#"<path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>"#,
        },
        SocialLink {
            name: "GitHub",
            href: "https://github.com/yukihamada",
            svg: r#"<path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>"#,
        },
        SocialLink {
            name: "LinkedIn",
            href: "https://linkedin.com/in/yukihamada",
            svg: r#"<path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"/>"#,
        },
        SocialLink {
            name: "Facebook",
            href: "https://facebook.com/yukihamada",
            svg: r#"<path d="M24 12.073c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.99 4.388 10.954 10.125 11.854v-8.385H7.078v-3.47h3.047V9.43c0-3.007 1.792-4.669 4.533-4.669 1.312 0 2.686.235 2.686.235v2.953H15.83c-1.491 0-1.956.925-1.956 1.874v2.25h3.328l-.532 3.47h-2.796v8.385C19.612 23.027 24 18.062 24 12.073z"/>"#,
        },
        SocialLink {
            name: "Instagram",
            href: "https://instagram.com/yukihamada",
            svg: r#"<path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z"/>"#,
        },
        SocialLink {
            name: "Email",
            href: "mailto:mail@yukihamada.jp",
            svg: r#"<path d="M20 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z"/>"#,
        },
    ]
}

fn projects_ja() -> Vec<Project> {
    vec![
        Project {
            title: "chatweb.ai",
            description: "AIとのチャットインターフェース。複数の最新モデルを一度に利用可能。",
            href: "https://chatweb.ai",
            badge: Some("Popular"),
            features: vec!["マルチモデル", "高速レスポンス", "使いやすいUI"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"/>"#,
            color: "from-blue-500 to-cyan-600",
        },
        Project {
            title: "elio.love",
            description: "世界初のMCP対応iOSアプリ。完全オフライン、プライバシー重視のAIアシスタント。",
            href: "https://elio.love",
            badge: Some("New"),
            features: vec!["MCP対応", "完全オフライン", "プライバシー保護"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>"#,
            color: "from-purple-500 to-indigo-600",
        },
        Project {
            title: "jiuflow.art",
            description: "ブラジリアン柔術のインストラクショナルプラットフォーム。",
            href: "https://jiuflow.art",
            badge: None,
            features: vec!["動画学習", "進捗管理", "フローチャート"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>"#,
            color: "from-orange-500 to-red-600",
        },
        Project {
            title: "stayflow",
            description: "ホテル・民泊管理システム。予約管理から顧客管理まで。",
            href: "https://stayflowapp.com",
            badge: None,
            features: vec!["予約管理", "顧客管理", "自動化"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>"#,
            color: "from-emerald-500 to-teal-600",
        },
    ]
}

fn projects_en() -> Vec<Project> {
    vec![
        Project {
            title: "chatweb.ai",
            description: "AI Chat Interface. Access multiple latest models in one place.",
            href: "https://chatweb.ai",
            badge: Some("Popular"),
            features: vec!["Multi-model", "Fast Response", "Intuitive UI"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"/>"#,
            color: "from-blue-500 to-cyan-600",
        },
        Project {
            title: "elio.love",
            description: "World's first MCP-enabled iOS app. Fully offline, privacy-first AI assistant.",
            href: "https://elio.love",
            badge: Some("New"),
            features: vec!["MCP Support", "Fully Offline", "Privacy First"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>"#,
            color: "from-purple-500 to-indigo-600",
        },
        Project {
            title: "jiuflow.art",
            description: "Brazilian Jiu-Jitsu instructional platform.",
            href: "https://jiuflow.art",
            badge: None,
            features: vec!["Video Learning", "Progress Tracking", "Flowcharts"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>"#,
            color: "from-orange-500 to-red-600",
        },
        Project {
            title: "stayflow",
            description: "Hotel & vacation rental management system.",
            href: "https://stayflowapp.com",
            badge: None,
            features: vec!["Booking Mgmt", "CRM", "Automation"],
            icon_svg: r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>"#,
            color: "from-emerald-500 to-teal-600",
        },
    ]
}

fn timeline_ja() -> Vec<TimelineItem> {
    vec![
        TimelineItem {
            year: "2024~",
            title: "株式会社イネブラ",
            role: "代表取締役CEO",
            description: "人生を「本質」だけで満たすEnablerとして、ライフスタイル・フィンテック・エデュテックの3つの事業を展開。",
            link: Some("https://enablerhq.com"),
            highlight: true,
        },
        TimelineItem {
            year: "2024~",
            title: "令和トラベル",
            role: "社外取締役・株主",
            description: "AIを活用したデジタルトラベルエージェンシー「NEWT」を運営。",
            link: Some("https://newt.net"),
            highlight: false,
        },
        TimelineItem {
            year: "2018~2024",
            title: "NOT A HOTEL",
            role: "共同創業者・元取締役・現株主",
            description: "「自宅を持たない暮らし」を実現する会員制ホテル兼不動産サービス。",
            link: Some("https://notahotel.com"),
            highlight: false,
        },
        TimelineItem {
            year: "2014~2021",
            title: "メルカリ",
            role: "取締役・CPO・CINO",
            description: "日本最大のフリマアプリ。取締役としてプロダクト責任者を務め成長を牽引。",
            link: None,
            highlight: false,
        },
        TimelineItem {
            year: "2003~2013",
            title: "サイブリッジ",
            role: "共同創業者",
            description: "塾講師ナビ、オールクーポンなどのウェブサービスを開発・運営。",
            link: None,
            highlight: false,
        },
    ]
}

fn timeline_en() -> Vec<TimelineItem> {
    vec![
        TimelineItem {
            year: "2024~",
            title: "Enabler, Inc.",
            role: "Founder & CEO",
            description: "Building lifestyle, fintech, and edtech businesses as an Enabler filling life with only the essentials.",
            link: Some("https://enablerhq.com"),
            highlight: true,
        },
        TimelineItem {
            year: "2024~",
            title: "Reiwa Travel",
            role: "Outside Director & Shareholder",
            description: "Operating NEWT, an AI-powered digital travel agency.",
            link: Some("https://newt.net"),
            highlight: false,
        },
        TimelineItem {
            year: "2018~2024",
            title: "NOT A HOTEL",
            role: "Co-founder, Former Director, Shareholder",
            description: "A membership hotel & real estate service realizing a life without owning a home.",
            link: Some("https://notahotel.com"),
            highlight: false,
        },
        TimelineItem {
            year: "2014~2021",
            title: "Mercari",
            role: "Director, CPO, CINO",
            description: "Japan's largest marketplace app. Led product growth as a board member.",
            link: None,
            highlight: false,
        },
        TimelineItem {
            year: "2003~2013",
            title: "Cybridge",
            role: "Co-founder",
            description: "Developed and operated web services including tutoring platforms and coupon services.",
            link: None,
            highlight: false,
        },
    ]
}

pub async fn home(State(state): State<Arc<AppState>>) -> HomeTemplate {
    let year = chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026);
    HomeTemplate {
        lang: "ja",
        title: "濱田優貴 - Founder & Entrepreneur".to_string(),
        description: "イネブラ創業者、エンジェル投資家。元メルカリCPO・取締役。AI、テクノロジー、柔術など多様な分野で活動中。".to_string(),
        base_url: state.base_url.clone(),
        canonical_url: state.base_url.clone(),
        og_image: format!("{}/static/og-image.svg", state.base_url),
        name_display: "濱田 優貴".to_string(),
        name_sub: "Yuki Hamada".to_string(),
        roles: vec![
            "起業家".to_string(),
            "エンジェル投資家".to_string(),
            "柔術家".to_string(),
            "ポーカープレイヤー".to_string(),
            "ギタリスト".to_string(),
        ],
        subtitle: "株式会社イネブラ 代表取締役CEO / 元メルカリCPO・CINO・取締役".to_string(),
        projects_title: "Projects".to_string(),
        projects_subtitle: "開発・運営しているプロダクト一覧".to_string(),
        projects: projects_ja(),
        timeline_title: "キャリア".to_string(),
        timeline_subtitle: "千葉県立大高校 → 東京理科大学（中退）→ 起業家・経営者として活動".to_string(),
        timeline_items: timeline_ja(),
        social_links: social_links(),
        footer_description: "イネブラ創業者、エンジェル投資家。世界中のクリエイターを支援し、新しい価値を生み出すコミュニティを構築。".to_string(),
        copyright_year: year,
        alt_lang_url: format!("{}/en", state.base_url),
        alt_lang_label: "English".to_string(),
        cta_label: "イネブラを見る".to_string(),
        view_service_label: "詳しく見る".to_string(),
    }
}

pub async fn home_en(State(state): State<Arc<AppState>>) -> HomeTemplate {
    let year = chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026);
    HomeTemplate {
        lang: "en",
        title: "Yuki Hamada - Founder & Entrepreneur".to_string(),
        description: "Founder of Enabler, angel investor. Former CPO & Director at Mercari. Active in AI, technology, BJJ, and more.".to_string(),
        base_url: state.base_url.clone(),
        canonical_url: format!("{}/en", state.base_url),
        og_image: format!("{}/static/og-image.svg", state.base_url),
        name_display: "Yuki Hamada".to_string(),
        name_sub: "濱田優貴".to_string(),
        roles: vec![
            "Entrepreneur".to_string(),
            "Angel Investor".to_string(),
            "BJJ Practitioner".to_string(),
            "Poker Player".to_string(),
            "Guitarist".to_string(),
        ],
        subtitle: "Founder & CEO of Enabler, Inc. / Former CPO & Director at Mercari".to_string(),
        projects_title: "Projects".to_string(),
        projects_subtitle: "Products I build and maintain".to_string(),
        projects: projects_en(),
        timeline_title: "Career".to_string(),
        timeline_subtitle: "From student to serial entrepreneur and tech executive".to_string(),
        timeline_items: timeline_en(),
        social_links: social_links(),
        footer_description: "Founder of Enabler, angel investor. Supporting creators worldwide and building communities that create new value.".to_string(),
        copyright_year: year,
        alt_lang_url: state.base_url.clone(),
        alt_lang_label: "日本語".to_string(),
        cta_label: "View Enabler".to_string(),
        view_service_label: "Learn more".to_string(),
    }
}
