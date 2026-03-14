use leptos::prelude::*;
use crate::i18n::{use_locale, Locale};
use crate::data::projects::PROJECTS;
use crate::data::timeline::TIMELINE;
use crate::i18n::translate;

struct VfsEntry {
    name: &'static str,
    is_dir: bool,
    content: Option<String>,
    children: Vec<VfsEntry>,
}

fn build_fs(locale: Locale) -> VfsEntry {
    let about = format!(
        "Name: {}\nRole: {}\nBased in: Tokyo, Japan\nSite: yukihamada.jp\n\n{}\n",
        translate(locale, "hero.name_first"),
        translate(locale, "hero.subtitle"),
        translate(locale, "footer.desc"),
    );

    let skills = "Languages: Rust, Swift, TypeScript, Python, Kotlin, SQL\n\
        Frameworks: Axum, Leptos, SwiftUI, React, Next.js\n\
        Infra: AWS (Lambda, DynamoDB, API GW), Fly.io, Docker, Cloudflare\n\
        Data: SQLite, DynamoDB, PostgreSQL, libSQL\n\
        Other: WASM, WebGL, Canvas API, P2P, Solana\n";

    let project_entries: Vec<VfsEntry> = PROJECTS.iter().map(|p| {
        let desc = translate(locale, &format!("work.{}.desc", p.key));
        let content = format!("Project: {}\nURL: {}\nStatus: {}\n\n{}\n",
            translate(locale, &format!("work.{}.desc", p.key).replace(".desc", ".title")),
            p.href,
            p.badge.unwrap_or("—"),
            desc,
        );
        VfsEntry { name: leak_str(p.key), is_dir: false, content: Some(content), children: vec![] }
    }).collect();

    let career_entries: Vec<VfsEntry> = TIMELINE.iter().map(|t| {
        let title = translate(locale, &format!("career.{}.title", t.key));
        let role = translate(locale, &format!("career.{}.role", t.key));
        let desc = translate(locale, &format!("career.{}.desc", t.key));
        let content = format!("{}\n{} ({})\n\n{}\n",
            title, role, t.year, desc,
        );
        VfsEntry { name: leak_str(t.key), is_dir: false, content: Some(content), children: vec![] }
    }).collect();

    VfsEntry {
        name: "",
        is_dir: true,
        content: None,
        children: vec![
            VfsEntry {
                name: "home",
                is_dir: true,
                content: None,
                children: vec![
                    VfsEntry {
                        name: "yuki",
                        is_dir: true,
                        content: None,
                        children: vec![
                            VfsEntry { name: "about.txt", is_dir: false, content: Some(about), children: vec![] },
                            VfsEntry { name: "skills.txt", is_dir: false, content: Some(skills.to_string()), children: vec![] },
                            VfsEntry {
                                name: "projects",
                                is_dir: true,
                                content: None,
                                children: project_entries,
                            },
                            VfsEntry {
                                name: "career",
                                is_dir: true,
                                content: None,
                                children: career_entries,
                            },
                        ],
                    },
                ],
            },
        ],
    }
}

fn leak_str(s: &str) -> &'static str {
    // For static-like usage in VFS built once
    Box::leak(s.to_string().into_boxed_str())
}

fn resolve_path<'a>(root: &'a VfsEntry, cwd: &str, path: &str) -> Option<&'a VfsEntry> {
    let abs = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", cwd.trim_end_matches('/'), path)
    };

    let parts: Vec<&str> = abs.split('/').filter(|s| !s.is_empty()).collect();
    let mut resolved: Vec<&str> = vec![];
    for p in &parts {
        match *p {
            "." => {}
            ".." => { resolved.pop(); }
            other => resolved.push(other),
        }
    }

    let mut node = root;
    for part in &resolved {
        let found = node.children.iter().find(|c| c.name == *part);
        match found {
            Some(child) => node = child,
            None => return None,
        }
    }
    Some(node)
}

fn normalize_cwd(cwd: &str, path: &str) -> String {
    let abs = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", cwd.trim_end_matches('/'), path)
    };
    let parts: Vec<&str> = abs.split('/').filter(|s| !s.is_empty()).collect();
    let mut resolved: Vec<&str> = vec![];
    for p in &parts {
        match *p {
            "." => {}
            ".." => { resolved.pop(); }
            other => resolved.push(other),
        }
    }
    if resolved.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", resolved.join("/"))
    }
}

fn exec_command(root: &VfsEntry, cwd: &mut String, input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd {
        "help" => {
            "Available commands:\n\
             \x1b[1m  ls\x1b[0m       List directory contents\n\
             \x1b[1m  cd\x1b[0m       Change directory\n\
             \x1b[1m  cat\x1b[0m      Display file contents\n\
             \x1b[1m  pwd\x1b[0m      Print working directory\n\
             \x1b[1m  whoami\x1b[0m   Display user info\n\
             \x1b[1m  open\x1b[0m     Open URL in browser\n\
             \x1b[1m  tree\x1b[0m     Show directory tree\n\
             \x1b[1m  clear\x1b[0m    Clear terminal\n\
             \x1b[1m  exit\x1b[0m     Close terminal\n\
             \nTip: Press / to toggle terminal".to_string()
        }
        "pwd" => cwd.clone(),
        "whoami" => "yuki — Founder & CEO of Enabler, Inc.".to_string(),
        "clear" => "\x1b[CLEAR]".to_string(),
        "exit" => "\x1b[EXIT]".to_string(),
        "ls" => {
            let target = if arg.is_empty() { cwd.as_str() } else { arg };
            match resolve_path(root, cwd, target) {
                Some(node) if node.is_dir => {
                    let mut out = String::new();
                    for child in &node.children {
                        if child.is_dir {
                            out.push_str(&format!("\x1b[1;34m{}/\x1b[0m  ", child.name));
                        } else {
                            out.push_str(&format!("{}  ", child.name));
                        }
                    }
                    out
                }
                Some(_) => format!("{}: Not a directory", target),
                None => format!("ls: {}: No such file or directory", target),
            }
        }
        "cd" => {
            if arg.is_empty() || arg == "~" {
                *cwd = "/home/yuki".to_string();
                return String::new();
            }
            let new_path = normalize_cwd(cwd, arg);
            match resolve_path(root, cwd, arg) {
                Some(node) if node.is_dir => {
                    *cwd = new_path;
                    String::new()
                }
                Some(_) => format!("cd: {}: Not a directory", arg),
                None => format!("cd: {}: No such file or directory", arg),
            }
        }
        "cat" => {
            if arg.is_empty() {
                return "usage: cat <file>".to_string();
            }
            match resolve_path(root, cwd, arg) {
                Some(node) if !node.is_dir => {
                    node.content.clone().unwrap_or_else(|| "(empty)".to_string())
                }
                Some(_) => format!("cat: {}: Is a directory", arg),
                None => format!("cat: {}: No such file or directory", arg),
            }
        }
        "tree" => {
            let target = if arg.is_empty() { cwd.as_str() } else { arg };
            match resolve_path(root, cwd, target) {
                Some(node) if node.is_dir => {
                    let mut out = String::new();
                    fn print_tree(node: &VfsEntry, prefix: &str, out: &mut String) {
                        for (i, child) in node.children.iter().enumerate() {
                            let is_last = i == node.children.len() - 1;
                            let connector = if is_last { "└── " } else { "├── " };
                            let next_prefix = if is_last {
                                format!("{}    ", prefix)
                            } else {
                                format!("{}│   ", prefix)
                            };
                            if child.is_dir {
                                out.push_str(&format!("{}{}\x1b[1;34m{}/\x1b[0m\n", prefix, connector, child.name));
                                print_tree(child, &next_prefix, out);
                            } else {
                                out.push_str(&format!("{}{}{}\n", prefix, connector, child.name));
                            }
                        }
                    }
                    out.push_str(&format!("\x1b[1;34m{}\x1b[0m\n", target));
                    print_tree(node, "", &mut out);
                    out
                }
                Some(_) => format!("tree: {}: Not a directory", target),
                None => format!("tree: {}: No such file or directory", target),
            }
        }
        "open" => {
            if arg.is_empty() {
                return "usage: open <url|project>".to_string();
            }
            // Try to find project by name
            for p in PROJECTS {
                if p.key.eq_ignore_ascii_case(arg) || p.href.contains(arg) {
                    return format!("\x1b[OPEN]{}", p.href);
                }
            }
            if arg.starts_with("http") {
                return format!("\x1b[OPEN]{}", arg);
            }
            format!("open: Unknown project '{}'", arg)
        }
        _ => format!("command not found: {}. Type 'help' for available commands.", cmd),
    }
}

#[component]
pub fn Terminal(
    #[prop(into)] show: RwSignal<bool>,
) -> impl IntoView {
    let locale = use_locale();
    let lines = RwSignal::new(Vec::<(String, String)>::new()); // (prompt, output)
    let input_val = RwSignal::new(String::new());
    let cwd = RwSignal::new("/home/yuki".to_string());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let history = RwSignal::new(Vec::<String>::new());
    let history_idx = RwSignal::new(0i32);

    // Build filesystem once when locale changes
    let fs = RwSignal::new(None::<Box<VfsEntry>>);

    Effect::new(move |_| {
        if show.get() {
            fs.set(Some(Box::new(build_fs(locale.get()))));
            // Show welcome message
            lines.set(vec![
                (String::new(), "Welcome to yukihamada.jp terminal".to_string()),
                (String::new(), "Type 'help' for available commands.".to_string()),
                (String::new(), String::new()),
            ]);
            cwd.set("/home/yuki".to_string());
            input_val.set(String::new());

            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    crate::wasm_utils::sleep_ms(100).await;
                    if let Some(el) = input_ref.get() {
                        let _ = el.focus();
                    }
                });
            }
        }
    });

    let on_submit = move || {
        let cmd = input_val.get_untracked();
        if cmd.is_empty() { return; }

        let prompt = format!("yuki@yukihamada:{}$ {}", cwd.get_untracked(), cmd);

        let output = if let Some(ref root) = *fs.read_untracked() {
            let mut cwd_str = cwd.get_untracked();
            let result = exec_command(root, &mut cwd_str, &cmd);
            cwd.set(cwd_str);
            result
        } else {
            "Error: filesystem not initialized".to_string()
        };

        // Handle special escape codes
        if output == "\x1b[CLEAR]" {
            lines.set(vec![]);
        } else if output == "\x1b[EXIT]" {
            show.set(false);
            return;
        } else if let Some(url) = output.strip_prefix("\x1b[OPEN]") {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(win) = web_sys::window() {
                    let _ = win.open_with_url_and_target(url, "_blank");
                }
            }
            lines.update(|l| {
                l.push((prompt.clone(), format!("Opening {}...", url)));
            });
        } else {
            lines.update(|l| {
                l.push((prompt.clone(), output));
            });
        }

        // Save to history
        history.update(|h| h.push(cmd));
        history_idx.set(-1);
        input_val.set(String::new());

        // Scroll to bottom
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                crate::wasm_utils::sleep_ms(10).await;
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(el) = doc.query_selector(".terminal-output").ok().flatten() {
                        el.set_scroll_top(el.scroll_height());
                    }
                }
            });
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                on_submit();
            }
            "ArrowUp" => {
                ev.prevent_default();
                let h = history.get_untracked();
                if h.is_empty() { return; }
                let idx = history_idx.get_untracked();
                let new_idx = if idx < 0 {
                    h.len() as i32 - 1
                } else {
                    (idx - 1).max(0)
                };
                if let Some(cmd) = h.get(new_idx as usize) {
                    input_val.set(cmd.clone());
                    history_idx.set(new_idx);
                }
            }
            "ArrowDown" => {
                ev.prevent_default();
                let h = history.get_untracked();
                let idx = history_idx.get_untracked();
                if idx < 0 { return; }
                let new_idx = idx + 1;
                if (new_idx as usize) < h.len() {
                    input_val.set(h[new_idx as usize].clone());
                    history_idx.set(new_idx);
                } else {
                    input_val.set(String::new());
                    history_idx.set(-1);
                }
            }
            "Escape" => {
                show.set(false);
            }
            "c" if ev.ctrl_key() => {
                let prompt = format!("yuki@yukihamada:{}$ {}^C", cwd.get_untracked(), input_val.get_untracked());
                lines.update(|l| l.push((prompt, String::new())));
                input_val.set(String::new());
            }
            _ => {}
        }
    };

    let focus_input = move |_: web_sys::MouseEvent| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    view! {
        <Show when=move || show.get()>
            <div class="terminal-overlay" on:click=move |_| show.set(false)>
                <div class="terminal-window" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                    <div class="terminal-titlebar">
                        <div class="terminal-dots">
                            <span class="terminal-dot dot-red" on:click=move |_| show.set(false)></span>
                            <span class="terminal-dot dot-yellow"></span>
                            <span class="terminal-dot dot-green"></span>
                        </div>
                        <span class="terminal-title">"yuki@yukihamada: ~"</span>
                    </div>
                    <div class="terminal-output" on:click=focus_input>
                        {move || {
                            lines.get().iter().map(|(prompt, output)| {
                                let p = prompt.clone();
                                let o = output.clone();
                                // Parse ANSI color codes for display
                                let display_output = o
                                    .replace("\x1b[1;34m", "<span class=\"term-blue\">")
                                    .replace("\x1b[1m", "<span class=\"term-bold\">")
                                    .replace("\x1b[0m", "</span>");
                                view! {
                                    <div class="terminal-line">
                                        {if !p.is_empty() {
                                            Some(view! { <div class="terminal-prompt" inner_html=p.replace("$", "<span class=\"term-green\">$</span>") /> })
                                        } else {
                                            None
                                        }}
                                        {if !display_output.is_empty() {
                                            Some(view! { <pre class="terminal-result" inner_html=display_output /> })
                                        } else {
                                            None
                                        }}
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                        <div class="terminal-input-line">
                            <span class="terminal-prompt-text">
                                {"yuki@yukihamada:"} {move || cwd.get()} {"$ "}
                            </span>
                            <input
                                node_ref=input_ref
                                type="text"
                                class="terminal-input"
                                autocomplete="off"
                                autocapitalize="off"
                                spellcheck="false"
                                on:keydown=on_keydown
                                on:input=move |ev| input_val.set(event_target_value(&ev))
                                prop:value=move || input_val.get()
                            />
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }
}
