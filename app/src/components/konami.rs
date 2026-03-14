use leptos::prelude::*;

#[component]
pub fn KonamiListener() -> impl IntoView {
    let active = RwSignal::new(false);

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use crate::wasm_utils::animation_loop;

        // Konami: Up Up Down Down Left Right Left Right B A
        const KONAMI: &[&str] = &[
            "ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown",
            "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight",
            "b", "a",
        ];

        let seq: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        Effect::new(move |_| {
            let seq = seq.clone();
            let key_cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                let mut s = seq.borrow_mut();
                s.push(e.key().to_lowercase());
                if s.len() > KONAMI.len() {
                    let drain_end = s.len() - KONAMI.len();
                    s.drain(0..drain_end);
                }
                if s.len() == KONAMI.len() && s.iter().zip(KONAMI.iter()).all(|(a, b)| a == *b) {
                    active.set(true);
                    if let Some(storage) = web_sys::window()
                        .and_then(|w| w.local_storage().ok().flatten())
                    {
                        let _ = storage.set_item("achievement_konami", "true");
                    }
                    s.clear();
                }
            });
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let target: &web_sys::EventTarget = doc.unchecked_ref();
                let _ = target.add_event_listener_with_callback("keydown", key_cb.as_ref().unchecked_ref());
                key_cb.forget();
            }
        });

        // Confetti animation — canvas injected via DOM, not in hydration tree
        struct ConfettiParticle {
            x: f64, y: f64, vx: f64, vy: f64,
            rotation: f64, color: &'static str, size: f64,
        }

        Effect::new(move |_| {
            if !active.get() { return; }

            let win = match web_sys::window() { Some(w) => w, None => return };
            let document = match win.document() { Some(d) => d, None => return };
            let body = match document.body() { Some(b) => b, None => return };

            // Create canvas via DOM injection
            let canvas: web_sys::HtmlCanvasElement = match document.create_element("canvas") {
                Ok(el) => el.unchecked_into(),
                Err(_) => return,
            };
            canvas.set_class_name("confetti-canvas");
            let _ = body.append_child(&canvas);

            let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1920.0);
            let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1080.0);
            canvas.set_width(w as u32);
            canvas.set_height(h as u32);

            let colors = &["#8b5cf6", "#a78bfa", "#c4b5fd", "#f472b6", "#34d399", "#fbbf24", "#60a5fa"];
            let particles: Rc<RefCell<Vec<ConfettiParticle>>> = Rc::new(RefCell::new(
                (0..150).map(|_| {
                    let ci = (js_sys::Math::random() * colors.len() as f64) as usize;
                    ConfettiParticle {
                        x: js_sys::Math::random() * w,
                        y: -20.0 - js_sys::Math::random() * 200.0,
                        vx: (js_sys::Math::random() - 0.5) * 8.0,
                        vy: js_sys::Math::random() * 4.0 + 2.0,
                        rotation: js_sys::Math::random() * 360.0,
                        color: colors[ci.min(colors.len() - 1)],
                        size: js_sys::Math::random() * 8.0 + 4.0,
                    }
                }).collect()
            ));

            let start = Rc::new(std::cell::Cell::new(0.0f64));
            let particles_anim = particles.clone();
            let start_clone = start.clone();
            let canvas_anim = canvas.clone();

            let _handle = animation_loop(move |ts| {
                if start_clone.get() == 0.0 { start_clone.set(ts); }
                let elapsed = ts - start_clone.get();

                let ctx: web_sys::CanvasRenderingContext2d = match canvas_anim.get_context("2d").ok().flatten() {
                    Some(c) => c.dyn_into().unwrap(),
                    None => return,
                };
                ctx.clear_rect(0.0, 0.0, w, h);

                let mut ps = particles_anim.borrow_mut();
                for p in ps.iter_mut() {
                    p.x += p.vx;
                    p.y += p.vy;
                    p.vy += 0.15;
                    p.rotation += p.vx * 2.0;
                    p.vx *= 0.99;

                    let alpha = if elapsed > 3000.0 { ((5000.0 - elapsed) / 2000.0).max(0.0) } else { 1.0 };
                    ctx.save();
                    ctx.translate(p.x, p.y).ok();
                    ctx.rotate(p.rotation * std::f64::consts::PI / 180.0).ok();
                    ctx.set_global_alpha(alpha);
                    ctx.set_fill_style_str(p.color);
                    ctx.fill_rect(-p.size / 2.0, -p.size / 4.0, p.size, p.size / 2.0);
                    ctx.restore();
                }

                if elapsed > 5000.0 {
                    // Remove canvas from DOM and reset state
                    let _ = canvas_anim.remove();
                    active.set(false);
                }
            });
            std::mem::forget(_handle);
        });
    }

    view! {
        <Show when=move || active.get()>
            <div class="konami-toast">"Secret Found! 🎮"</div>
        </Show>
    }
}
