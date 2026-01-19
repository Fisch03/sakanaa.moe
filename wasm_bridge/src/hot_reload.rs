use log::{debug, info, warn};
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

static mut HAS_CONNECTED: bool = false;

thread_local! {
    static LAST_HTML_HASH: RefCell<String> = RefCell::new(String::new());
    static LAST_CSS_HASH: RefCell<String> = RefCell::new(String::new());
    static ACTIVE_WS: RefCell<Option<web_sys::WebSocket>> = RefCell::new(None);
}

pub fn setup_hot_reload(window: &web_sys::Window) {
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

fn connect_hmr(ws_url: String) {
    // Close existing connection if any
    ACTIVE_WS.with(|ws_cell| {
        if let Some(ws) = ws_cell.borrow().as_ref() {
            let _ = ws.close();
        }
        *ws_cell.borrow_mut() = None;
    });

    let ws = match web_sys::WebSocket::new(&ws_url) {
        Ok(ws) => ws,
        Err(_) => {
            schedule_reconnect(ws_url);
            return;
        }
    };

    // Store the new WebSocket
    ACTIVE_WS.with(|ws_cell| {
        *ws_cell.borrow_mut() = Some(ws.clone());
    });

    let onopen = Closure::wrap(Box::new(move || unsafe {
        if HAS_CONNECTED {
            info!("reconnected to server. requesting state...");
            // The server sends the state immediately upon connection
        } else {
            debug!("connected to hot reload server");
            HAS_CONNECTED = true;
        }
    }) as Box<dyn FnMut()>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmsg = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Ok(txt_js) = e.data().dyn_into::<js_sys::JsString>() {
            let txt: String = txt_js.into();
            if let Ok(val) = js_sys::JSON::parse(&txt) {
                let html_hash = js_sys::Reflect::get(&val, &"html_hash".into())
                    .ok()
                    .and_then(|v| v.as_string());
                let css_hash = js_sys::Reflect::get(&val, &"css_hash".into())
                    .ok()
                    .and_then(|v| v.as_string());
                let css = js_sys::Reflect::get(&val, &"css".into())
                    .ok()
                    .and_then(|v| v.as_string());

                if let (Some(html_hash), Some(css_hash), Some(css)) = (html_hash, css_hash, css) {
                    handle_update(html_hash, css_hash, css);
                }
            } else {
                if txt == "reload" {
                    let _ = web_sys::window().unwrap().location().reload();
                }
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
        // If the socket was closed intentionally (e.g. during reload_wasm), we might not want to reconnect immediately
        // But checking how we closed it is hard. For now, reconnection logic stays.
        // However, if we are in the process of replacing WASM, the new WASM will take over.
        schedule_reconnect(ws_url_clone.clone());
    }) as Box<dyn FnMut(web_sys::CloseEvent)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
}

fn handle_update(html_hash: String, css_hash: String, css: String) {
    LAST_HTML_HASH.with(|h| {
        LAST_CSS_HASH.with(|c| {
            let mut stored_html_hash = h.borrow_mut();
            let mut stored_css_hash = c.borrow_mut();

            if stored_html_hash.is_empty() {
                *stored_html_hash = html_hash;
                *stored_css_hash = css_hash;
                return;
            }

            let html_changed = *stored_html_hash != html_hash;
            let css_changed = *stored_css_hash != css_hash;

            if html_changed {
                info!("HTML changed, reloading page...");
                *stored_html_hash = html_hash;
                *stored_css_hash = css_hash;
                let _ = web_sys::window().unwrap().location().reload();
            } else {
                if css_changed {
                    info!("CSS changed, updating styles...");
                    *stored_css_hash = css_hash;
                    update_style(&css);
                }

                close_ws();
                reload_wasm();
            }
        })
    });
}

fn close_ws() {
    ACTIVE_WS.with(|ws_cell| {
        if let Some(ws) = ws_cell.borrow().as_ref() {
            ws.set_onclose(None);
            let _ = ws.close();
            debug!("closed old websocket connection");
        }
        *ws_cell.borrow_mut() = None;
    });
}

fn reload_wasm() {
    debug!("reloading wasm...");
    let window = web_sys::window().unwrap();
    let _ = js_sys::Reflect::get(&window, &"_reload_wasm".into())
        .and_then(|f| f.dyn_into::<js_sys::Function>())
        .map(|f| f.call0(&JsValue::NULL));
}

fn update_style(css: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    if let Some(style) = document.get_element_by_id("hmr-style") {
        style.set_inner_html(css);
    } else if let Ok(style) = document.create_element("style") {
        style.set_id("hmr-style");
        style.set_inner_html(css);
        let html_doc: web_sys::HtmlDocument = document.dyn_into().unwrap();
        if let Some(head) = html_doc.head() {
            let _ = head.append_child(&style);
        }
    }
}

fn schedule_reconnect(ws_url: String) {
    let closure = Closure::wrap(Box::new(move || {
        connect_hmr(ws_url.clone());
    }) as Box<dyn FnMut()>);

    let _ = web_sys::window()
        .unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            1000,
        );
    closure.forget();
}
