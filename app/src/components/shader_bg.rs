use leptos::prelude::*;

#[component]
pub fn ShaderBg() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use crate::wasm_utils::{animation_loop, prefers_reduced_motion};

        const VERT: &str = r#"
            attribute vec2 position;
            void main() { gl_Position = vec4(position, 0.0, 1.0); }
        "#;

        const FRAG: &str = r#"
            precision mediump float;
            uniform float u_time;
            uniform vec2 u_resolution;
            uniform vec2 u_mouse;
            void main() {
                vec2 uv = gl_FragCoord.xy / u_resolution;
                float t = u_time * 0.15;
                float v = 0.0;
                v += sin(uv.x * 8.0 + t * 1.2);
                v += sin(uv.y * 6.0 + t * 0.8);
                v += sin((uv.x + uv.y) * 4.0 + t);
                v += sin(sqrt(uv.x * uv.x * 16.0 + uv.y * uv.y * 16.0) - t * 1.5);
                vec2 mouse = u_mouse / u_resolution;
                float d = distance(uv, mouse);
                v += sin(d * 25.0 - t * 4.0) * 0.3 * exp(-d * 3.0);
                v *= 0.25;
                vec3 col = vec3(
                    0.22 + 0.12 * sin(v * 3.14159),
                    0.04 + 0.06 * sin(v * 3.14159 + 2.094),
                    0.32 + 0.2 * sin(v * 3.14159 + 4.189)
                );
                gl_FragColor = vec4(col, 1.0);
            }
        "#;

        fn compile_shader(
            gl: &web_sys::WebGlRenderingContext,
            src: &str,
            shader_type: u32,
        ) -> Option<web_sys::WebGlShader> {
            let shader = gl.create_shader(shader_type)?;
            gl.shader_source(&shader, src);
            gl.compile_shader(&shader);
            if gl.get_shader_parameter(&shader, web_sys::WebGlRenderingContext::COMPILE_STATUS)
                .as_bool().unwrap_or(false)
            {
                Some(shader)
            } else {
                gl.delete_shader(Some(&shader));
                None
            }
        }

        Effect::new(move |_| {
            if prefers_reduced_motion() { return; }

            let window = match web_sys::window() { Some(w) => w, None => return };
            let document = match window.document() { Some(d) => d, None => return };

            // Create canvas via DOM — not part of Leptos hydration tree
            let canvas: web_sys::HtmlCanvasElement = match document.create_element("canvas") {
                Ok(el) => el.unchecked_into(),
                Err(_) => return,
            };
            canvas.set_class_name("shader-canvas");

            // Insert as first child of .hero section
            if let Ok(Some(hero)) = document.query_selector(".hero") {
                if let Some(first) = hero.first_child() {
                    let _ = hero.insert_before(&canvas, Some(&first));
                } else {
                    let _ = hero.append_child(&canvas);
                }
            } else {
                return;
            }

            let w = canvas.client_width() as f64;
            let h = canvas.client_height() as f64;
            canvas.set_width((w * 0.5) as u32);
            canvas.set_height((h * 0.5) as u32);

            let gl: web_sys::WebGlRenderingContext = match canvas.get_context("webgl").ok().flatten() {
                Some(c) => c.dyn_into().unwrap(),
                None => return,
            };

            let vert_shader = match compile_shader(&gl, VERT, web_sys::WebGlRenderingContext::VERTEX_SHADER) {
                Some(s) => s, None => return,
            };
            let frag_shader = match compile_shader(&gl, FRAG, web_sys::WebGlRenderingContext::FRAGMENT_SHADER) {
                Some(s) => s, None => return,
            };

            let program = match gl.create_program() {
                Some(p) => p, None => return,
            };
            gl.attach_shader(&program, &vert_shader);
            gl.attach_shader(&program, &frag_shader);
            gl.link_program(&program);

            if !gl.get_program_parameter(&program, web_sys::WebGlRenderingContext::LINK_STATUS)
                .as_bool().unwrap_or(false)
            {
                return;
            }

            gl.use_program(Some(&program));

            // Full-screen quad
            let vertices: [f32; 12] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];
            let buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(web_sys::WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));

            unsafe {
                let vert_array = js_sys::Float32Array::view(&vertices);
                gl.buffer_data_with_array_buffer_view(
                    web_sys::WebGlRenderingContext::ARRAY_BUFFER,
                    &vert_array,
                    web_sys::WebGlRenderingContext::STATIC_DRAW,
                );
            }

            let pos_attr = gl.get_attrib_location(&program, "position") as u32;
            gl.enable_vertex_attrib_array(pos_attr);
            gl.vertex_attrib_pointer_with_i32(pos_attr, 2, web_sys::WebGlRenderingContext::FLOAT, false, 0, 0);

            let u_time = gl.get_uniform_location(&program, "u_time");
            let u_resolution = gl.get_uniform_location(&program, "u_resolution");
            let u_mouse = gl.get_uniform_location(&program, "u_mouse");

            let mouse = Rc::new(RefCell::new((w * 0.5, h * 0.5)));
            let mouse_clone = mouse.clone();

            let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let mut m = mouse_clone.borrow_mut();
                m.0 = e.client_x() as f64 * 0.5;
                m.1 = e.client_y() as f64 * 0.5;
            });
            let doc: &web_sys::EventTarget = document.unchecked_ref();
            let _ = doc.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());
            move_cb.forget();

            let canvas_resize = canvas.clone();
            let resize_cb = Closure::<dyn FnMut()>::new(move || {
                let c: &web_sys::HtmlCanvasElement = &canvas_resize;
                c.set_width((c.client_width() as f64 * 0.5) as u32);
                c.set_height((c.client_height() as f64 * 0.5) as u32);
            });
            let _ = window.add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref());
            resize_cb.forget();

            let start = Rc::new(std::cell::Cell::new(0.0f64));
            let start_c = start.clone();
            let mouse_anim = mouse.clone();

            let _handle = animation_loop(move |ts| {
                if start_c.get() == 0.0 { start_c.set(ts); }
                let t = (ts - start_c.get()) / 1000.0;

                let cw = canvas.width() as f32;
                let ch = canvas.height() as f32;
                gl.viewport(0, 0, cw as i32, ch as i32);

                gl.uniform1f(u_time.as_ref(), t as f32);
                gl.uniform2f(u_resolution.as_ref(), cw, ch);
                let m = mouse_anim.borrow();
                gl.uniform2f(u_mouse.as_ref(), m.0 as f32, ch - m.1 as f32);

                gl.draw_arrays(web_sys::WebGlRenderingContext::TRIANGLES, 0, 6);
            });
            std::mem::forget(_handle);
        });
    }

    // Render nothing — canvas is injected into .hero via Effect above
    view! {}
}
