mod abi;
pub mod dom;
#[cfg(feature = "hot-reload")]
mod hot_reload;
mod logger;

use maud::{Markup, html};

pub use abi::{alloc, dealloc, from_host_string, to_host_string};
pub use dom::{attach_window_fn, host_update_element};
pub use logger::init_logger;

pub fn init() {
    init_logger();
    console_error_panic_hook::set_once();

    #[cfg(feature = "hot-reload")]
    if let Some(window) = web_sys::window() {
        hot_reload::setup_hot_reload(&window);
    }
}

pub fn load_wasm_module() -> Markup {
    html! {
        script type="module" {
            "import init from '/pkg/site.js'; init();"
        }
    }
}

