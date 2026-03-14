use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::use_params_map;

use crate::data::music::TRACKS;
use crate::i18n::{translate, use_locale, Locale};
#[cfg(target_arch = "wasm32")]
use crate::i18n;

#[component]
pub fn MusicPage() -> impl IntoView {
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

    let lang = params.with_untracked(|p| p.get("lang").unwrap_or_else(|| "ja".to_string()));
    let canonical_music = format!("https://yukihamada.jp/{lang}/music");

    // Inject music player JS
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            let mut tracks_json = String::from("[");
            for (i, t) in TRACKS.iter().enumerate() {
                if i > 0 {
                    tracks_json.push(',');
                }
                tracks_json.push_str(&format!(
                    r#"{{"title":"{}","artist":"{}","src":"{}","artwork":"{}","color":"{}"}}"#,
                    t.title, t.artist, t.src, t.artwork, t.color
                ));
            }
            tracks_json.push(']');

            let js = format!(
                r#"
                (function(){{
                    if(window._music_init) return;
                    window._music_init = true;
                    var tracks = {tracks_json};
                    var audio = new Audio();
                    var ci = 0;
                    var playing = false;
                    var player = document.getElementById('music-player');
                    var seekBar = document.getElementById('player-seek');
                    if(!player || !seekBar) return;

                    window.playTrack = function(i) {{
                        ci = i;
                        var t = tracks[ci];
                        audio.src = t.src;
                        document.getElementById('player-artwork').src = t.artwork;
                        document.getElementById('player-title').textContent = t.title;
                        document.getElementById('player-artist').textContent = t.artist;
                        player.style.display = '';
                        player.style.borderColor = t.color;
                        audio.play();
                        playing = true;
                        updatePlayBtn();
                        highlightTrack();
                    }};

                    window.togglePlay = function() {{
                        if (playing) {{ audio.pause(); }} else {{ audio.play(); }}
                        playing = !playing;
                        updatePlayBtn();
                    }};

                    window.prevTrack = function() {{
                        ci = (ci - 1 + tracks.length) % tracks.length;
                        playTrack(ci);
                    }};

                    window.nextTrack = function() {{
                        ci = (ci + 1) % tracks.length;
                        playTrack(ci);
                    }};

                    function updatePlayBtn() {{
                        document.getElementById('play-icon').style.display = playing ? 'none' : '';
                        document.getElementById('pause-icon').style.display = playing ? '' : 'none';
                    }}

                    function highlightTrack() {{
                        document.querySelectorAll('.music-track').forEach(function(el, idx) {{
                            el.style.borderColor = idx === ci ? tracks[ci].color : '';
                        }});
                    }}

                    function fmtTime(s) {{
                        var m = Math.floor(s / 60);
                        var sec = Math.floor(s % 60);
                        return m + ':' + (sec < 10 ? '0' : '') + sec;
                    }}

                    audio.addEventListener('timeupdate', function() {{
                        if (audio.duration) {{
                            seekBar.value = (audio.currentTime / audio.duration * 100).toFixed(1);
                            document.getElementById('player-current').textContent = fmtTime(audio.currentTime);
                            document.getElementById('player-duration').textContent = fmtTime(audio.duration);
                        }}
                    }});

                    seekBar.addEventListener('input', function() {{
                        if (audio.duration) {{
                            audio.currentTime = audio.duration * seekBar.value / 100;
                        }}
                    }});

                    audio.addEventListener('ended', function() {{
                        nextTrack();
                    }});
                }})();
                "#
            );
            let _ = web_sys::js_sys::eval(&js);
        });
    }

    view! {
        <Title text=move || format!("{} - Yuki Hamada", translate(locale.get(), "music.title")) />
        <Meta name="description" content=move || translate(locale.get(), "music.subtitle") />
        <Meta name="author" content="Yuki Hamada" />
        <Link rel="canonical" href=canonical_music />
        <Meta property="og:title" content=move || format!("{} - Yuki Hamada", translate(locale.get(), "music.title")) />
        <Meta property="og:description" content=move || translate(locale.get(), "music.subtitle") />
        <Meta property="og:type" content="website" />
        <Meta property="og:image" content="https://yukihamada.jp/og-image.svg" />
        <Meta property="og:image:width" content="1200" />
        <Meta property="og:image:height" content="630" />
        <Meta property="og:site_name" content="Yuki Hamada" />
        <Meta property="og:url" content=move || format!("https://yukihamada.jp/{}/music", locale.get().code()) />
        <Meta name="twitter:card" content="summary_large_image" />
        <Meta name="twitter:site" content="@yukihamada" />
        <Meta name="twitter:creator" content="@yukihamada" />
        <Meta name="twitter:title" content=move || format!("{} - Yuki Hamada", translate(locale.get(), "music.title")) />
        <Meta name="twitter:description" content=move || translate(locale.get(), "music.subtitle") />
        <Meta name="twitter:image" content="https://yukihamada.jp/og-image.svg" />
        <Link rel="alternate" hreflang="ja" href="https://yukihamada.jp/ja/music" />
        <Link rel="alternate" hreflang="en" href="https://yukihamada.jp/en/music" />
        <Link rel="alternate" hreflang="x-default" href="https://yukihamada.jp/ja/music" />

        <div class="section" style="padding-top:100px;">
            <div class="container">
                <div class="section-header fade-in">
                    <h2>"Music"</h2>
                    <p>{move || translate(locale.get(), "music.subtitle")}</p>
                </div>

                // Player (hidden until a track is clicked)
                <div class="music-player glass-card fade-in" id="music-player" style="display:none;">
                    <div class="player-inner">
                        <img id="player-artwork" src="" alt="" class="player-artwork"/>
                        <div class="player-details">
                            <div id="player-title" class="player-title"></div>
                            <div id="player-artist" class="player-artist"></div>
                            <div class="player-progress">
                                <input type="range" id="player-seek" min="0" max="100" value="0" step="0.1" class="player-seek"/>
                                <div class="player-times">
                                    <span id="player-current">"0:00"</span>
                                    <span id="player-duration">"0:00"</span>
                                </div>
                            </div>
                            <div class="player-controls">
                                <button onclick="prevTrack()" aria-label="Previous">
                                    <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
                                </button>
                                <button id="play-pause-btn" onclick="togglePlay()" class="player-play-btn" aria-label="Play/Pause">
                                    <svg id="play-icon" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
                                    <svg id="pause-icon" viewBox="0 0 24 24" fill="currentColor" style="display:none;"><path d="M6 4h4v16H6zM14 4h4v16h-4z"/></svg>
                                </button>
                                <button onclick="nextTrack()" aria-label="Next">
                                    <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

                // Track List
                <div class="music-list">
                    {TRACKS.iter().enumerate().map(|(i, t)| {
                        let artwork = t.artwork;
                        let title = t.title;
                        let artist = t.artist;
                        let onclick = format!("playTrack({i})");
                        view! {
                            <div class="music-track glass-card fade-in" onclick=onclick style="cursor:pointer;">
                                <img src=artwork alt=title class="music-artwork" loading="lazy"/>
                                <div class="music-info">
                                    <div class="music-title">{title}</div>
                                    <div class="music-artist">{artist}</div>
                                </div>
                                <button class="music-play-btn" aria-label=format!("Play {title}")>
                                    <svg viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
                                </button>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}
