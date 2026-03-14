use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use crate::wasm_utils::{animation_loop, noise2d, prefers_reduced_motion, AnimationHandle};

#[cfg(target_arch = "wasm32")]
struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    home_x: f64,
    home_y: f64,
}

#[cfg(target_arch = "wasm32")]
struct ParticleSystem {
    particles: Vec<Particle>,
    width: f64,
    height: f64,
    mouse_x: f64,
    mouse_y: f64,
    time: f64,
    attracting: bool,
    attract_timer: f64,
}

#[cfg(target_arch = "wasm32")]
impl ParticleSystem {
    fn new(width: f64, height: f64, count: usize) -> Self {
        let mut particles = Vec::with_capacity(count);
        for _ in 0..count {
            let x = js_sys::Math::random() * width;
            let y = js_sys::Math::random() * height;
            particles.push(Particle {
                x,
                y,
                vx: (js_sys::Math::random() - 0.5) * 0.6,
                vy: (js_sys::Math::random() - 0.5) * 0.6,
                radius: js_sys::Math::random() * 1.5 + 0.5,
                home_x: x,
                home_y: y,
            });
        }
        Self {
            particles, width, height,
            mouse_x: -1000.0, mouse_y: -1000.0,
            time: 0.0, attracting: false, attract_timer: 0.0,
        }
    }

    fn resize(&mut self, w: f64, h: f64) {
        self.width = w;
        self.height = h;
    }

    fn start_attraction(&mut self) {
        self.attracting = true;
        self.attract_timer = 0.0;
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        let scale = self.height.min(self.width) * 0.15;

        let y_points: Vec<(f64, f64)> = vec![
            (-2.0, -2.0), (-1.5, -1.0), (-1.0, 0.0), (-1.0, 1.0), (-1.0, 2.0),
            (0.0, -2.0), (-0.5, -1.0),
        ];
        let h_points: Vec<(f64, f64)> = vec![
            (0.5, -2.0), (0.5, -1.0), (0.5, 0.0), (0.5, 1.0), (0.5, 2.0),
            (2.0, -2.0), (2.0, -1.0), (2.0, 0.0), (2.0, 1.0), (2.0, 2.0),
            (1.0, 0.0), (1.5, 0.0),
        ];

        let all_points: Vec<(f64, f64)> = y_points.iter().chain(h_points.iter())
            .map(|&(px, py)| (cx + px * scale, cy + py * scale))
            .collect();

        for (i, p) in self.particles.iter_mut().enumerate() {
            let target = &all_points[i % all_points.len()];
            p.home_x = target.0 + (js_sys::Math::random() - 0.5) * scale * 0.3;
            p.home_y = target.1 + (js_sys::Math::random() - 0.5) * scale * 0.3;
        }
    }

    fn update(&mut self) {
        self.time += 0.005;

        if self.attracting {
            self.attract_timer += 1.0;
            if self.attract_timer > 180.0 {
                self.attracting = false;
                for p in &mut self.particles {
                    p.home_x = js_sys::Math::random() * self.width;
                    p.home_y = js_sys::Math::random() * self.height;
                }
            }
        }

        let repulse_dist = 150.0;
        for p in &mut self.particles {
            if self.attracting {
                p.vx += (p.home_x - p.x) * 0.03;
                p.vy += (p.home_y - p.y) * 0.03;
                p.vx *= 0.92;
                p.vy *= 0.92;
            } else {
                let noise_val = noise2d(p.x * 0.003 + self.time, p.y * 0.003 + self.time);
                let angle = noise_val * std::f64::consts::TAU * 2.0;
                p.vx += angle.cos() * 0.05;
                p.vy += angle.sin() * 0.05;

                let dx = p.x - self.mouse_x;
                let dy = p.y - self.mouse_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < repulse_dist && dist > 0.0 {
                    let force = (repulse_dist - dist) / repulse_dist * 2.0;
                    p.vx += dx / dist * force;
                    p.vy += dy / dist * force;
                }

                p.vx *= 0.97;
                p.vy *= 0.97;

                let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
                if speed < 0.15 {
                    p.vx += (js_sys::Math::random() - 0.5) * 0.1;
                    p.vy += (js_sys::Math::random() - 0.5) * 0.1;
                }
            }

            p.x += p.vx;
            p.y += p.vy;

            if p.x < 0.0 { p.x = self.width; }
            if p.x > self.width { p.x = 0.0; }
            if p.y < 0.0 { p.y = self.height; }
            if p.y > self.height { p.y = 0.0; }
        }
    }

    fn draw(&self, ctx: &web_sys::CanvasRenderingContext2d) {
        ctx.clear_rect(0.0, 0.0, self.width, self.height);

        let connect_dist = 120.0;
        let len = self.particles.len();

        for i in 0..len {
            for j in (i + 1)..len {
                let dx = self.particles[i].x - self.particles[j].x;
                let dy = self.particles[i].y - self.particles[j].y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < connect_dist {
                    let opacity = (1.0 - dist / connect_dist) * 0.3;
                    ctx.set_stroke_style_str(&format!("rgba(139, 92, 246, {opacity})"));
                    ctx.set_line_width(0.5);
                    ctx.begin_path();
                    ctx.move_to(self.particles[i].x, self.particles[i].y);
                    ctx.line_to(self.particles[j].x, self.particles[j].y);
                    ctx.stroke();
                }
            }
        }

        for p in &self.particles {
            ctx.set_fill_style_str("rgba(139, 92, 246, 0.6)");
            ctx.begin_path();
            let _ = ctx.arc(p.x, p.y, p.radius, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }
    }
}

#[component]
pub fn ParticleCanvas() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |prev: Option<Option<AnimationHandle>>| {
            if let Some(Some(handle)) = prev {
                handle.cancel();
            }

            if prefers_reduced_motion() {
                return None;
            }

            let window = match web_sys::window() { Some(w) => w, None => return None };
            let document = match window.document() { Some(d) => d, None => return None };

            // Create canvas via DOM — not part of Leptos hydration tree
            let canvas: web_sys::HtmlCanvasElement = match document.create_element("canvas") {
                Ok(el) => el.unchecked_into(),
                Err(_) => return None,
            };
            canvas.set_class_name("particle-canvas");

            // Append to .hero section
            if let Ok(Some(hero)) = document.query_selector(".hero") {
                let _ = hero.append_child(&canvas);
            } else {
                return None;
            }

            let dpr = window.device_pixel_ratio();
            let rect = canvas.get_bounding_client_rect();
            let w = rect.width();
            let h = rect.height();
            canvas.set_width((w * dpr) as u32);
            canvas.set_height((h * dpr) as u32);

            let ctx: web_sys::CanvasRenderingContext2d = canvas
                .get_context("2d").ok().flatten().unwrap().dyn_into().unwrap();
            ctx.scale(dpr, dpr).ok();

            let is_mobile = w < 768.0;
            let count = if is_mobile { 40 } else { 80 };
            let system = Rc::new(RefCell::new(ParticleSystem::new(w, h, count)));

            // Mouse move
            let system_mouse = system.clone();
            let canvas_clone = canvas.clone();
            let mouse_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let rect = canvas_clone.get_bounding_client_rect();
                let mut sys = system_mouse.borrow_mut();
                sys.mouse_x = e.client_x() as f64 - rect.left();
                sys.mouse_y = e.client_y() as f64 - rect.top();
            });
            let _ = canvas.add_event_listener_with_callback("mousemove", mouse_cb.as_ref().unchecked_ref());
            mouse_cb.forget();

            // Mouse leave
            let system_leave = system.clone();
            let leave_cb = Closure::<dyn FnMut()>::new(move || {
                let mut sys = system_leave.borrow_mut();
                sys.mouse_x = -1000.0;
                sys.mouse_y = -1000.0;
            });
            let _ = canvas.add_event_listener_with_callback("mouseleave", leave_cb.as_ref().unchecked_ref());
            leave_cb.forget();

            // Click to attract
            let system_click = system.clone();
            let click_cb = Closure::<dyn FnMut()>::new(move || {
                system_click.borrow_mut().start_attraction();
            });
            let _ = canvas.add_event_listener_with_callback("click", click_cb.as_ref().unchecked_ref());
            click_cb.forget();

            // Resize
            let system_resize = system.clone();
            let canvas_resize = canvas.clone();
            let resize_cb = Closure::<dyn FnMut()>::new(move || {
                let win = web_sys::window().unwrap();
                let dpr = win.device_pixel_ratio();
                let rect = canvas_resize.get_bounding_client_rect();
                let w = rect.width();
                let h = rect.height();
                canvas_resize.set_width((w * dpr) as u32);
                canvas_resize.set_height((h * dpr) as u32);
                if let Ok(Some(ctx_val)) = canvas_resize.get_context("2d") {
                    let ctx: web_sys::CanvasRenderingContext2d = ctx_val.dyn_into().unwrap();
                    ctx.scale(dpr, dpr).ok();
                }
                system_resize.borrow_mut().resize(w, h);
            });
            let _ = window.add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref());
            resize_cb.forget();

            // Animation loop
            let system_anim = system.clone();
            let canvas_anim = canvas.clone();
            let handle = animation_loop(move |_timestamp| {
                let mut sys = system_anim.borrow_mut();
                sys.update();

                if let Ok(Some(ctx_val)) = canvas_anim.get_context("2d") {
                    let ctx: web_sys::CanvasRenderingContext2d = ctx_val.dyn_into().unwrap();
                    sys.draw(&ctx);
                }
            });

            Some(handle)
        });
    }

    // Render nothing — canvas is injected into .hero via Effect above
    view! {}
}
