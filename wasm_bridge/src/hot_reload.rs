use log::{info, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

static mut HAS_CONNECTED: bool = false;

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
