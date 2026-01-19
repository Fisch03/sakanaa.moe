use log::*;
use wasm_bindgen::prelude::*;

pub mod components {
    pub mod head;
}

use maud::{DOCTYPE, html};

#[unsafe(no_mangle)]
pub extern "C" fn style() -> *mut u8 {
    include_str!("../style/main.scss");
    static CSS: &str = grass::include!("site/style/main.scss");

    wasm_bridge::to_host_string(CSS.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn render(path_ptr: *const u8, path_len: usize) -> *mut u8 {
    wasm_bridge::init();

    let path = unsafe { wasm_bridge::from_host_string(path_ptr, path_len) };

    info!("rendering page for path: {}", path);

    let html = html! {
        (DOCTYPE)
        html {
            head {
                title { "sakanaa!" }
                (components::head::shared())
            }
            body {
                #main {
                    .column {
                        .container {
                            h1 { "hello world!" }
                            p { "path: " (path) }
                            p { "status: " span id="status" { "server rendered" } }
                            button onclick="window.trigger_action()" { "click me!" }
                        }
                    }
                }

                (wasm_bridge::load_wasm_module())
            }
        }
    };

    wasm_bridge::to_host_string(html.into_string())
}

#[wasm_bindgen(start)]
pub fn run() {
    wasm_bridge::init();
    info!("wasm module initialized");

    wasm_bridge::attach_window_fn("trigger_action", || {
        info!("button clicked!");
        wasm_bridge::host_update_element("status", "updated by client!");
    });
}
