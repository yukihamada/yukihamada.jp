use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }
}

/// Global theme signal — call this once in layout
pub fn use_theme() -> RwSignal<Theme> {
    let theme = RwSignal::new(Theme::Dark);

    #[cfg(target_arch = "wasm32")]
    {
        // Read from localStorage or system preference
        Effect::new(move |_| {
            let saved = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item("theme").ok().flatten());

            let initial = match saved.as_deref() {
                Some("light") => Theme::Light,
                Some("dark") => Theme::Dark,
                _ => {
                    // Check system preference
                    let prefers_dark = web_sys::window()
                        .and_then(|w| w.match_media("(prefers-color-scheme: light)").ok().flatten())
                        .map(|mq| mq.matches())
                        .unwrap_or(false);
                    if prefers_dark { Theme::Light } else { Theme::Dark }
                }
            };
            theme.set(initial);
        });
    }

    theme
}

#[allow(unused_variables)]
pub fn apply_theme(theme: Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(root) = doc.document_element() {
                let _ = root.set_attribute("data-theme", theme.as_str());
            }
        }
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = storage.set_item("theme", theme.as_str());
        }
    }
}

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme = use_theme();

    // Apply theme whenever it changes
    Effect::new(move |_| {
        apply_theme(theme.get());
    });

    let toggle = move |_: web_sys::MouseEvent| {
        let new_theme = match theme.get_untracked() {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        theme.set(new_theme);
    };

    view! {
        <button class="theme-toggle" on:click=toggle title="Toggle theme" aria-label="Toggle dark/light mode">
            <Show when=move || theme.get() == Theme::Dark
                fallback=|| view! {
                    // Moon icon (currently light, click to go dark)
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width:16px;height:16px;">
                        <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/>
                    </svg>
                }
            >
                // Sun icon (currently dark, click to go light)
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width:16px;height:16px;">
                    <circle cx="12" cy="12" r="5"/>
                    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
                </svg>
            </Show>
        </button>
    }
}
