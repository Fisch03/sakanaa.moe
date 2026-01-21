use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Blob, BlobPropertyBag, Url, WorkerOptions};

// Keep track of workers to clean them up on hot reload
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = _register_worker)]
    fn register_worker(w: &web_sys::Worker);
    #[wasm_bindgen(js_namespace = window, js_name = _cleanup_workers)]
    fn cleanup_workers();
}

pub struct Worker {
    inner: web_sys::Worker,
    _on_message: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl Worker {
    pub fn new_from_script(script: &str) -> Result<Self, JsValue> {
        let options = WorkerOptions::new();
        options.set_name("bridge-worker");
        options.set_type(web_sys::WorkerType::Module);

        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(script));

        let blob_props = BlobPropertyBag::new();
        blob_props.set_type("application/javascript");

        let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)?;
        let url = Url::create_object_url_with_blob(&blob)?;

        let worker = web_sys::Worker::new_with_options(&url, &options)?;

        Url::revoke_object_url(&url)?;

        #[cfg(feature = "hot-reload")]
        if let Some(w) = web_sys::window()
            && js_sys::Reflect::has(&w, &"_register_worker".into()).unwrap_or(false)
        {
            register_worker(&worker);
        }

        let mut worker = Self {
            inner: worker,
            _on_message: None,
            _on_error: None,
        };
        worker.set_onerror(move |e| {
            log::error!("Album worker error: {:?}", e);
        });

        Ok(worker)
    }

    pub fn new_rust_worker(entry_point: &str) -> Result<Self, JsValue> {
        let origin = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default();

        let timestamp = js_sys::Date::now();

        let script = format!(
            r#"
            import init, {{ {} }} from '{}/pkg/site.js?t={}';
            
            (async () => {{
                try {{
                    await init('{}/pkg/site_bg.wasm?t={}');
                    {}(self);
                }} catch (e) {{
                    console.error("Worker initialization failed:", e);
                    setTimeout(() => {{ throw e; }}); 
                }}
            }})();
            "#,
            entry_point, origin, timestamp, origin, timestamp, entry_point
        );

        Self::new_from_script(&script)
    }

    pub fn set_onmessage<T, F>(&mut self, mut f: F)
    where
        T: for<'de> serde::Deserialize<'de>,
        F: FnMut(T) + 'static,
    {
        let closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            if let Ok(data) = serde_wasm_bindgen::from_value(e.data()) {
                f(data);
            } else {
                web_sys::console::warn_1(&"Failed to deserialize worker message".into());
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.inner
            .set_onmessage(Some(closure.as_ref().unchecked_ref()));
        self._on_message = Some(closure);
    }

    pub fn set_onerror<F>(&mut self, f: F)
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(web_sys::Event)>);
        self.inner
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        self._on_error = Some(closure);
    }

    pub fn post_message<T: serde::Serialize>(&self, message: &T) -> Result<(), JsValue> {
        let val = serde_wasm_bindgen::to_value(message)?;
        self.inner.post_message(&val)
    }

    pub fn terminate(&self) {
        self.inner.terminate();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.terminate();
    }
}

// Extension trait for the Worker side logic (DedicatedWorkerGlobalScope)
pub trait WorkerContextExt {
    fn post_message_typed<T: serde::Serialize>(&self, message: &T) -> Result<(), JsValue>;
    fn set_onmessage_typed<T, F>(&self, f: F)
    where
        T: for<'de> serde::Deserialize<'de>,
        F: FnMut(T) + 'static;
}

impl WorkerContextExt for web_sys::DedicatedWorkerGlobalScope {
    fn post_message_typed<T: serde::Serialize>(&self, message: &T) -> Result<(), JsValue> {
        let val = serde_wasm_bindgen::to_value(message)?;
        self.post_message(&val)
    }

    fn set_onmessage_typed<T, F>(&self, mut f: F)
    where
        T: for<'de> serde::Deserialize<'de>,
        F: FnMut(T) + 'static,
    {
        let closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            if let Ok(data) = serde_wasm_bindgen::from_value(e.data()) {
                f(data);
            } else {
                web_sys::console::warn_1(&"Failed to deserialize worker message".into());
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Keep the closure alive for the lifetime of the worker
    }
}
