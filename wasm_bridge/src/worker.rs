use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Url, Worker, WorkerOptions};

pub struct BridgeWorker {
    inner: Worker,
    _on_message: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(web_sys::ErrorEvent)>>,
}

impl BridgeWorker {
    pub fn new_from_script(script: &str) -> Result<Self, JsValue> {
        let options = WorkerOptions::new();
        options.set_name("bridge-worker");

        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(script));

        let blob_props = BlobPropertyBag::new();
        blob_props.set_type("application/javascript");

        let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)?;
        let url = Url::create_object_url_with_blob(&blob)?;

        let worker = Worker::new_with_options(&url, &options)?;

        // Revoke the URL after creating the worker to free memory
        Url::revoke_object_url(&url)?;

        Ok(Self {
            inner: worker,
            _on_message: None,
            _on_error: None,
        })
    }

    pub fn set_onmessage<F>(&mut self, f: F)
    where
        F: FnMut(web_sys::MessageEvent) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::MessageEvent)>);
        self.inner.set_onmessage(Some(closure.as_ref().unchecked_ref()));
        self._on_message = Some(closure);
    }

    pub fn set_onerror<F>(&mut self, f: F)
    where
        F: FnMut(web_sys::ErrorEvent) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        self.inner.set_onerror(Some(closure.as_ref().unchecked_ref()));
        self._on_error = Some(closure);
    }

    pub fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
        self.inner.post_message(message)
    }

    pub fn terminate(&self) {
        self.inner.terminate();
    }
}

impl Drop for BridgeWorker {
    fn drop(&mut self) {
        // We don't automatically terminate on drop because the worker might be intended to outlive the wrapper struct in some cases,
        // but typically in Rust RAII we might want to.
        // For now, let's leave it manual or rely on the user to call terminate if they want to stop it.
        // However, we MUST drop the closure to prevent leaks if it's not done automatically.
        // (Closure is in the Option, so it will be dropped).
    }
}
