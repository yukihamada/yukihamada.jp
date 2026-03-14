use leptos::prelude::*;
use crate::i18n::use_locale;

#[component]
pub fn ChatWidget() -> impl IntoView {
    let open = RwSignal::new(false);
    let input = RwSignal::new(String::new());
    let messages: RwSignal<Vec<(bool, String)>> = RwSignal::new(vec![
        (false, "こんにちは！濱田優貴について何でも聞いてください。".to_string()),
    ]);
    let loading = RwSignal::new(false);
    let locale = use_locale();
    let do_send = RwSignal::new(false);

    // Trigger send when do_send flips to true
    Effect::new(move |_| {
        if !do_send.get() { return; }
        do_send.set(false);

        let msg = input.get_untracked();
        let msg = msg.trim().to_string();
        if msg.is_empty() || loading.get_untracked() { return; }

        input.set(String::new());
        messages.update(|v| v.push((true, msg.clone())));
        loading.set(true);

        let lang = locale.get_untracked().code().to_string();

        #[cfg(target_arch = "wasm32")]
        {
            use serde_json::json;
            wasm_bindgen_futures::spawn_local(async move {
                let body = json!({ "message": msg, "lang": lang }).to_string();
                let result = gloo_net::http::Request::post("/api/chat")
                    .header("Content-Type", "application/json")
                    .body(body)
                    .unwrap()
                    .send()
                    .await;

                let reply = match result {
                    Ok(res) => match res.json::<serde_json::Value>().await {
                        Ok(json) => json["reply"]
                            .as_str()
                            .unwrap_or("エラーが発生しました。")
                            .to_string(),
                        Err(_) => "レスポンスの解析に失敗しました。".to_string(),
                    },
                    Err(_) => "通信エラーが発生しました。".to_string(),
                };
                messages.update(|v| v.push((false, reply)));
                loading.set(false);

                // Scroll to bottom
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        if let Some(el) = doc.get_element_by_id("chat-messages") {
                            el.set_scroll_top(el.scroll_height());
                        }
                    }
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = lang;
            loading.set(false);
        }
    });

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send.set(true);
        }
    };

    let placeholder = move || match locale.get().code() {
        "ja" => "メッセージを入力...",
        _ => "Ask me anything...",
    };

    view! {
        <div class="chat-widget">
            <button
                class="chat-fab"
                class:open=move || open.get()
                on:click=move |_| open.update(|v| *v = !*v)
                aria-label="Chat with Yuki"
            >
                <svg class="chat-fab-icon chat-fab-open" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
                </svg>
                <svg class="chat-fab-icon chat-fab-close" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M18 6L6 18M6 6l12 12"/>
                </svg>
            </button>

            <div class="chat-panel" class:open=move || open.get()>
                <div class="chat-header">
                    <div class="chat-avatar">
                        <span class="gradient-text">"YH"</span>
                    </div>
                    <div>
                        <div class="chat-header-name">"Yuki AI"</div>
                        <div class="chat-header-sub">"Ask me anything"</div>
                    </div>
                </div>
                <div class="chat-messages" id="chat-messages">
                    {move || messages.get().into_iter().map(|(is_user, text)| {
                        let cls = if is_user { "chat-msg user" } else { "chat-msg bot" };
                        view! { <div class=cls>{text}</div> }
                    }).collect::<Vec<_>>()}
                    <Show when=move || loading.get()>
                        <div class="chat-msg bot chat-typing">
                            <span/><span/><span/>
                        </div>
                    </Show>
                </div>
                <div class="chat-input-row">
                    <textarea
                        class="chat-input"
                        placeholder=placeholder
                        prop:value=move || input.get()
                        on:input=move |ev| input.set(event_target_value(&ev))
                        on:keydown=on_keydown
                        rows="1"
                    />
                    <button
                        class="chat-send"
                        on:click=move |_| do_send.set(true)
                        disabled=move || loading.get()
                    >
                        <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20">
                            <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"/>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
