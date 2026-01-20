use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone)]
pub struct Window(pub(crate) web_sys::Window);

impl Window {
    pub fn get() -> Self {
        Self(web_sys::window().expect("Window object not found"))
    }

    pub fn inner_width(&self) -> Option<f64> {
        self.0.inner_width().ok().and_then(|v| v.as_f64())
    }

    pub fn inner_height(&self) -> Option<f64> {
        self.0.inner_height().ok().and_then(|v| v.as_f64())
    }

    pub fn size(&self) -> Option<(f64, f64)> {
        Some((self.inner_width()?, self.inner_height()?))
    }

    pub fn attach_fn<F>(&self, name: &str, f: F)
    where
        F: FnMut() + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
        let _ = js_sys::Reflect::set(
            &self.0,
            &JsValue::from_str(name),
            closure.as_ref().unchecked_ref(),
        );

        closure.forget();
    }

    pub fn add_event_listener<F>(&self, event: &str, f: F) -> Closure<dyn FnMut(web_sys::Event)>
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::Event)>);
        let _ = self
            .0
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
        closure
    }

    pub fn add_wheel_event_listener<F>(&self, f: F) -> Closure<dyn FnMut(web_sys::WheelEvent)>
    where
        F: FnMut(web_sys::WheelEvent) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::WheelEvent)>);
        let _ = self
            .0
            .add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
        closure
    }

    pub fn add_resize_event_listener<F>(&self, f: F) -> Closure<dyn FnMut(web_sys::UiEvent)>
    where
        F: FnMut(web_sys::UiEvent) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::UiEvent)>);
        let _ = self
            .0
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
        closure
    }

    pub fn remove_listener(&self, event: &str, closure: &JsValue) {
        let _ = self
            .0
            .remove_event_listener_with_callback(event, closure.unchecked_ref());
    }

    /// Starts an animation loop. The closure should return `true` to continue, `false` to stop.
    pub fn start_animation_loop<F>(&self, mut f: F)
    where
        F: FnMut(f64) -> bool + 'static,
    {
        let closure_rc = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
        let closure_clone = closure_rc.clone();
        let window = self.0.clone();

        *closure_clone.borrow_mut() = Some(Closure::wrap(Box::new(move |time| {
            if f(time) {
                let _ = window.request_animation_frame(
                    closure_rc
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                );
            } else {
                let _ = closure_rc.borrow_mut().take();
            }
        }) as Box<dyn FnMut(f64)>));

        let _ = self.0.request_animation_frame(
            closure_clone
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .unchecked_ref(),
        );
    }

    pub fn request_animation_frame<F>(&self, f: F)
    where
        F: FnMut(f64) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(f64)>);
        let _ = self
            .0
            .request_animation_frame(closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

impl std::ops::Deref for Window {
    type Target = web_sys::Window;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}