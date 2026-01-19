use log::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn host_update_element(id: &str, content: &str) {
    if let Some(window) = web_sys::window()
        && let Some(document) = window.document()
    {
        if let Some(element) = document.get_element_by_id(id) {
            element.set_inner_html(content);
        } else {
            warn!("WASM tried to update missing element: {}", id);
        }
    }
}

pub fn attach_window_fn<F>(name: &str, f: F)
where
    F: FnMut() + 'static,
{
    let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);

    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str(name),
            closure.as_ref().unchecked_ref(),
        );
    }

    closure.forget();
}
