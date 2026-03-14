use leptos::prelude::*;
use crate::i18n::{translate, use_locale};

struct SkillNode {
    label: &'static str,
    x: f64,
    y: f64,
    size: f64,
    color: &'static str,
    #[allow(dead_code)]
    group: u8,
}

const SKILLS: &[SkillNode] = &[
    // Core (center)
    SkillNode { label: "Rust", x: 0.5, y: 0.4, size: 36.0, color: "#f97316", group: 0 },
    SkillNode { label: "Swift", x: 0.3, y: 0.35, size: 28.0, color: "#60a5fa", group: 0 },
    SkillNode { label: "TypeScript", x: 0.67, y: 0.35, size: 26.0, color: "#3b82f6", group: 0 },
    // Frameworks
    SkillNode { label: "Axum", x: 0.38, y: 0.55, size: 22.0, color: "#e07b30", group: 1 },
    SkillNode { label: "Leptos", x: 0.58, y: 0.55, size: 22.0, color: "#e07b30", group: 1 },
    SkillNode { label: "SwiftUI", x: 0.23, y: 0.5, size: 20.0, color: "#60a5fa", group: 1 },
    SkillNode { label: "React", x: 0.77, y: 0.5, size: 20.0, color: "#61dafb", group: 1 },
    // Infra
    SkillNode { label: "AWS", x: 0.25, y: 0.7, size: 24.0, color: "#f59e0b", group: 2 },
    SkillNode { label: "Fly.io", x: 0.5, y: 0.72, size: 22.0, color: "#c8601a", group: 2 },
    SkillNode { label: "Docker", x: 0.72, y: 0.7, size: 20.0, color: "#0ea5e9", group: 2 },
    // Data
    SkillNode { label: "SQLite", x: 0.4, y: 0.85, size: 20.0, color: "#10b981", group: 3 },
    SkillNode { label: "DynamoDB", x: 0.6, y: 0.85, size: 18.0, color: "#f59e0b", group: 3 },
    SkillNode { label: "WASM", x: 0.5, y: 0.2, size: 24.0, color: "#c8601a", group: 0 },
];

// Connections between skills (indices into SKILLS)
const EDGES: &[(usize, usize)] = &[
    (0, 3), (0, 4), (0, 12), // Rust -> Axum, Leptos, WASM
    (1, 5),                   // Swift -> SwiftUI
    (2, 6),                   // TypeScript -> React
    (3, 8), (4, 8),           // Axum/Leptos -> Fly.io
    (0, 7), (7, 11),          // Rust -> AWS -> DynamoDB
    (8, 10),                  // Fly.io -> SQLite
    (0, 10),                  // Rust -> SQLite
    (12, 4),                  // WASM -> Leptos
];

#[component]
pub fn SkillTree() -> impl IntoView {
    let locale = use_locale();
    let wrap_ref = NodeRef::<leptos::html::Div>::new();
    let hovered = RwSignal::new(Option::<usize>::None);

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use crate::wasm_utils::animation_loop;

        Effect::new(move |_| {
            let wrap_el = match wrap_ref.get() {
                Some(el) => el,
                None => return,
            };

            let document = web_sys::window().unwrap().document().unwrap();

            // Create canvas via DOM — not part of Leptos hydration tree
            let canvas: web_sys::HtmlCanvasElement = document
                .create_element("canvas").unwrap().unchecked_into();
            canvas.set_class_name("skill-tree-canvas");

            // Insert before the Show tooltip (as first child of wrap)
            let wrap: &web_sys::Element = &wrap_el;
            if let Some(first) = wrap.first_child() {
                let _ = wrap.insert_before(&canvas, Some(&first));
            } else {
                let _ = wrap.append_child(&canvas);
            }

            let dpr = web_sys::window().unwrap().device_pixel_ratio();
            let w = canvas.client_width() as f64;
            let h = canvas.client_height() as f64;
            if w < 1.0 || h < 1.0 {
                let canvas_retry = canvas.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    crate::wasm_utils::sleep_ms(200).await;
                    let dpr = web_sys::window().unwrap().device_pixel_ratio();
                    canvas_retry.set_width((canvas_retry.client_width() as f64 * dpr) as u32);
                    canvas_retry.set_height((canvas_retry.client_height() as f64 * dpr) as u32);
                });
                return;
            }
            canvas.set_width((w * dpr) as u32);
            canvas.set_height((h * dpr) as u32);

            let hover_state: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

            // Mouse move detection
            let hover_mouse = hover_state.clone();
            let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let target_val = e.target().unwrap();
                let canvas_el_ref: &web_sys::HtmlCanvasElement = target_val.unchecked_ref();
                let rect = canvas_el_ref.get_bounding_client_rect();
                let mx = (e.client_x() as f64 - rect.left()) / rect.width();
                let my = (e.client_y() as f64 - rect.top()) / rect.height();

                let mut found = None;
                for (i, s) in SKILLS.iter().enumerate() {
                    let dx = mx - s.x;
                    let dy = my - s.y;
                    let threshold = s.size / 300.0;
                    if dx * dx + dy * dy < threshold * threshold {
                        found = Some(i);
                        break;
                    }
                }
                *hover_mouse.borrow_mut() = found;
                hovered.set(found);
            });
            let _ = canvas.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());
            move_cb.forget();

            let leave_h = hover_state.clone();
            let leave_cb = Closure::<dyn FnMut()>::new(move || {
                *leave_h.borrow_mut() = None;
                hovered.set(None);
            });
            let _ = canvas.add_event_listener_with_callback("mouseleave", leave_cb.as_ref().unchecked_ref());
            leave_cb.forget();

            // Resize handler
            let canvas_resize = canvas.clone();
            let resize_cb = Closure::<dyn FnMut()>::new(move || {
                let dpr = web_sys::window().unwrap().device_pixel_ratio();
                let w = canvas_resize.client_width() as f64;
                let h = canvas_resize.client_height() as f64;
                if w > 0.0 && h > 0.0 {
                    canvas_resize.set_width((w * dpr) as u32);
                    canvas_resize.set_height((h * dpr) as u32);
                }
            });
            let _ = web_sys::window().unwrap().add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref());
            resize_cb.forget();

            let hover_anim = hover_state.clone();
            let _handle = animation_loop(move |ts| {
                let ctx: web_sys::CanvasRenderingContext2d = match canvas.get_context("2d").ok().flatten() {
                    Some(c) => c.dyn_into().unwrap(),
                    None => return,
                };
                let dpr = web_sys::window().unwrap().device_pixel_ratio();
                let cw = canvas.client_width() as f64;
                let ch = canvas.client_height() as f64;
                if cw > 0.0 && ch > 0.0 && (canvas.width() as f64) < 1.0 {
                    canvas.set_width((cw * dpr) as u32);
                    canvas.set_height((ch * dpr) as u32);
                }
                let w = canvas.width() as f64 / dpr;
                let h = canvas.height() as f64 / dpr;
                if w < 1.0 || h < 1.0 { return; }
                ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).ok();
                ctx.clear_rect(0.0, 0.0, w, h);

                let hovered_idx = *hover_anim.borrow();
                let pulse = (ts / 1000.0).sin() * 0.15 + 1.0;

                // Draw edges
                for &(a, b) in EDGES {
                    let sa = &SKILLS[a];
                    let sb = &SKILLS[b];
                    let is_active = hovered_idx == Some(a) || hovered_idx == Some(b);
                    let alpha = if is_active { 0.6 } else { 0.15 };
                    let width = if is_active { 2.0 } else { 1.0 };

                    ctx.set_stroke_style_str(&format!("rgba(190, 90, 20, {alpha})"));
                    ctx.set_line_width(width);
                    ctx.begin_path();
                    ctx.move_to(sa.x * w, sa.y * h);
                    ctx.line_to(sb.x * w, sb.y * h);
                    ctx.stroke();
                }

                // Draw nodes
                for (i, s) in SKILLS.iter().enumerate() {
                    let is_hov = hovered_idx == Some(i);
                    let connected = hovered_idx.map(|hi| {
                        EDGES.iter().any(|&(a, b)| (a == hi && b == i) || (b == hi && a == i)) || hi == i
                    }).unwrap_or(true);

                    let size = if is_hov { s.size * pulse } else { s.size };
                    let alpha = if connected { 1.0 } else { 0.3 };

                    ctx.set_global_alpha(alpha);

                    if is_hov {
                        ctx.set_shadow_blur(20.0);
                        ctx.set_shadow_color(s.color);
                    }

                    ctx.set_fill_style_str(s.color);
                    ctx.begin_path();
                    let _ = ctx.arc(s.x * w, s.y * h, size * 0.5, 0.0, std::f64::consts::TAU);
                    ctx.fill();

                    ctx.set_shadow_blur(0.0);

                    // Label — drawn below the circle in a readable dark color
                    let font_size = if is_hov { 13.0f64 } else { 11.0f64 };
                    ctx.set_fill_style_str("rgba(80,40,10,0.85)");
                    ctx.set_font(&format!("{}px 'Plus Jakarta Sans', sans-serif", font_size));
                    ctx.set_text_align("center");
                    ctx.set_text_baseline("top");
                    let label_y = s.y * h + size * 0.5 + 4.0;
                    let _ = ctx.fill_text(s.label, s.x * w, label_y);

                    ctx.set_global_alpha(1.0);
                }
            });
            std::mem::forget(_handle);
        });
    }

    view! {
        <section class="section skill-tree-section" id="skills">
            <div class="container">
                <div class="section-header fade-in">
                    <p class="section-label">{move || translate(locale.get(), "skills.label")}</p>
                    <h2>{move || translate(locale.get(), "skills.title")}</h2>
                    <p class="section-hint">{move || translate(locale.get(), "skills.hint")}</p>
                </div>
                <div class="skill-tree-wrap fade-in" node_ref=wrap_ref>
                    <Show when=move || hovered.get().is_some()>
                        <div class="skill-tooltip">
                            {move || hovered.get().map(|i| SKILLS[i].label).unwrap_or("")}
                        </div>
                    </Show>
                </div>
            </div>
        </section>
    }
}
