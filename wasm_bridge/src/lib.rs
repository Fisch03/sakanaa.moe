use log::*;
use maud::{Markup, html};
use std::alloc::{Layout, alloc as std_alloc, dealloc as std_dealloc};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn host_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn host_error(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    pub fn host_warn(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = info)]
    pub fn host_info(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    pub fn host_debug(s: &str);
}

struct WasmBridgeLogger;

impl Log for WasmBridgeLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());

            match record.level() {
                Level::Error => host_error(&msg),
                Level::Warn => host_warn(&msg),
                Level::Info => host_info(&msg),
                Level::Debug => host_debug(&msg),
                Level::Trace => host_debug(&msg),
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: WasmBridgeLogger = WasmBridgeLogger;

pub fn init() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Trace)
    }

    #[cfg(feature = "hot-reload")]
    if let Some(window) = web_sys::window() {
        setup_hot_reload(&window);
    }
}

#[cfg(feature = "hot-reload")]
static mut HAS_CONNECTED: bool = false;

#[cfg(feature = "hot-reload")]
fn setup_hot_reload(window: &web_sys::Window) {
    if let Ok(location) = window.location().href() {
        let ws_protocol = if location.starts_with("https") {
            "wss"
        } else {
            "ws"
        };
        let host = window
            .location()
            .host()
            .unwrap_or_else(|_| "localhost:3000".into());
        let ws_url = format!("{}://{}/_ws", ws_protocol, host);

        connect_hmr(ws_url);
    }
}

#[cfg(feature = "hot-reload")]
fn connect_hmr(ws_url: String) {
    let ws = match web_sys::WebSocket::new(&ws_url) {
        Ok(ws) => ws,
        Err(_) => {
            schedule_reconnect(ws_url);
            return;
        }
    };

    let onopen = Closure::wrap(Box::new(move || unsafe {
        if HAS_CONNECTED {
            info!("Reconnected to server. Reloading page...");
            let _ = web_sys::window().unwrap().location().reload();
        } else {
            info!("Connected to Hot Reload Server");
            HAS_CONNECTED = true;
        }
    }) as Box<dyn FnMut()>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmsg = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            let txt: String = txt.into();
            if txt == "reload" {
                let _ = web_sys::window().unwrap().location().reload();
            }
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    ws.set_onmessage(Some(onmsg.as_ref().unchecked_ref()));
    onmsg.forget();

    let ws_url_clone = ws_url.clone();
    let onclose = Closure::wrap(Box::new(move |_e: web_sys::CloseEvent| {
        unsafe {
            if HAS_CONNECTED {
                warn!("lost connection to the dev server. attempting to reconnect...");
            }
        }
        schedule_reconnect(ws_url_clone.clone());
    }) as Box<dyn FnMut(web_sys::CloseEvent)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
}

#[cfg(feature = "hot-reload")]
fn schedule_reconnect(ws_url: String) {
    let closure = Closure::wrap(Box::new(move || {
        connect_hmr(ws_url.clone());
    }) as Box<dyn FnMut()>);

    // Retry every 1000ms
    let _ = web_sys::window()
        .unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            1000,
        );
    closure.forget();
}

pub fn load_wasm_module() -> Markup {
    html! {
        script type="module" {
            "import init from '/pkg/site.js'; init();"
        }
    }
}

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

pub unsafe fn from_host_string(ptr: *const u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

pub fn to_host_string(s: String) -> *mut u8 {
    let mut bytes = s.into_bytes();
    let total_len = bytes.len() as u32;

    let mut buffer = total_len.to_le_bytes().to_vec();
    buffer.append(&mut bytes);

    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
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

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let layout = Layout::from_size_align(len, 1).unwrap();
    unsafe { std_alloc(layout) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    let layout = Layout::from_size_align(len, 1).unwrap();
    unsafe { std_dealloc(ptr, layout) }
}

