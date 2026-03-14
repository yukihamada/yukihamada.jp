use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = web_sys::window().unwrap();
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
    });
    let _ = JsFuture::from(promise).await;
}

pub struct AnimationHandle {
    id: Rc<RefCell<i32>>,
    running: Rc<RefCell<bool>>,
}

impl AnimationHandle {
    pub fn cancel(&self) {
        *self.running.borrow_mut() = false;
        if let Some(window) = web_sys::window() {
            window.cancel_animation_frame(*self.id.borrow()).ok();
        }
    }
}

impl Drop for AnimationHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn animation_loop<F>(mut tick: F) -> AnimationHandle
where
    F: FnMut(f64) + 'static,
{
    let id = Rc::new(RefCell::new(0i32));
    let running = Rc::new(RefCell::new(true));

    let id_clone = id.clone();
    let running_clone = running.clone();

    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let id_inner = id.clone();
    let running_inner = running.clone();

    *g.borrow_mut() = Some(Closure::new(move |timestamp: f64| {
        if !*running_inner.borrow() {
            return;
        }
        tick(timestamp);
        if *running_inner.borrow() {
            if let Some(window) = web_sys::window() {
                if let Some(ref cb) = *f.borrow() {
                    if let Ok(new_id) = window.request_animation_frame(cb.as_ref().unchecked_ref())
                    {
                        *id_inner.borrow_mut() = new_id;
                    }
                }
            }
        }
    }));

    if let Some(window) = web_sys::window() {
        if let Some(ref cb) = *g.borrow() {
            if let Ok(new_id) = window.request_animation_frame(cb.as_ref().unchecked_ref()) {
                *id_clone.borrow_mut() = new_id;
            }
        }
    }

    AnimationHandle {
        id: id_clone,
        running: running_clone,
    }
}

/// Simple 2D value noise for flow fields
pub fn noise2d(x: f64, y: f64) -> f64 {
    fn hash(x: i32, y: i32) -> f64 {
        let n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
        let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        let n = n ^ (n >> 16);
        (n & 0x7FFF) as f64 / 32767.0
    }

    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);

    let a = hash(xi, yi);
    let b = hash(xi + 1, yi);
    let c = hash(xi, yi + 1);
    let d = hash(xi + 1, yi + 1);

    a + u * (b - a) + v * (c - a) + u * v * (a - b - c + d)
}

pub fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|mql| mql.matches())
        .unwrap_or(false)
}
