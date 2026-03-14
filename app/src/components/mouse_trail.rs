use leptos::prelude::*;

#[component]
pub fn MouseTrail() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::collections::VecDeque;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use crate::wasm_utils::{animation_loop, prefers_reduced_motion};

        struct Trail {
            x: f64,
            y: f64,
            age: f64,
        }

        Effect::new(move |_| {
            if prefers_reduced_motion() {
                return;
            }

            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let document = match window.document() {
                Some(d) => d,
                None => return,
            };
            let body = match document.body() {
                Some(b) => b,
                None => return,
            };

            // Create canvas directly via DOM — not part of Leptos hydration tree
            let canvas: web_sys::HtmlCanvasElement = document
                .create_element("canvas")
                .unwrap()
                .unchecked_into();
            canvas.set_class_name("mouse-trail-canvas");
            let _ = body.append_child(&canvas);

            let w = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1920.0);
            let h = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1080.0);
            let dpr = window.device_pixel_ratio();
            canvas.set_width((w * dpr) as u32);
            canvas.set_height((h * dpr) as u32);

            let trails: Rc<RefCell<VecDeque<Trail>>> = Rc::new(RefCell::new(VecDeque::new()));

            let trails_mouse = trails.clone();
            let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let mut t = trails_mouse.borrow_mut();
                t.push_back(Trail {
                    x: e.client_x() as f64,
                    y: e.client_y() as f64,
                    age: 0.0,
                });
                if t.len() > 30 {
                    t.pop_front();
                }
            });

            let doc_target: &web_sys::EventTarget = document.unchecked_ref();
            let _ = doc_target.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());
            move_cb.forget();

            let canvas_resize = canvas.clone();
            let resize_cb = Closure::<dyn FnMut()>::new(move || {
                let win = web_sys::window().unwrap();
                let dpr = win.device_pixel_ratio();
                let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1920.0);
                let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1080.0);
                canvas_resize.set_width((w * dpr) as u32);
                canvas_resize.set_height((h * dpr) as u32);
            });
            let _ = window.add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref());
            resize_cb.forget();

            let trails_anim = trails.clone();
            let canvas_anim = canvas.clone();
            let _handle = animation_loop(move |_ts| {
                let ctx: web_sys::CanvasRenderingContext2d = match canvas_anim.get_context("2d").ok().flatten() {
                    Some(c) => c.dyn_into().unwrap(),
                    None => return,
                };
                let win = web_sys::window().unwrap();
                let dpr = win.device_pixel_ratio();
                let w = canvas_anim.width() as f64 / dpr;
                let h = canvas_anim.height() as f64 / dpr;
                ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).ok();
                ctx.clear_rect(0.0, 0.0, w, h);

                let mut t = trails_anim.borrow_mut();
                let len = t.len();
                for (i, trail) in t.iter_mut().enumerate() {
                    trail.age += 0.04;
                    if trail.age < 1.0 {
                        let alpha = (1.0 - trail.age) * 0.4 * (i as f64 / len.max(1) as f64);
                        let radius = (1.0 - trail.age) * 12.0 + 2.0;
                        ctx.set_fill_style_str(&format!("rgba(190, 90, 20, {alpha:.3})"));
                        ctx.begin_path();
                        let _ = ctx.arc(trail.x, trail.y, radius, 0.0, std::f64::consts::TAU);
                        ctx.fill();
                    }
                }
                t.retain(|trail| trail.age < 1.0);
            });
            std::mem::forget(_handle);
        });
    }

    // Render nothing — canvas is injected into document.body via Effect above
    view! {}
}
