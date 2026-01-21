mod abi;
#[cfg(feature = "hot-reload")]
mod hot_reload;
mod loader;
mod logger;

pub mod dom;
pub mod window;
pub mod worker;

pub use abi::{alloc, dealloc, from_host_string, to_host_string};
pub use dom::{Element, ElementCollection, document};
pub use loader::load_wasm_module;
pub use logger::init_logger;
pub use window::Window;
pub use worker::Worker;

pub fn init() {
    init_logger();
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    #[cfg(feature = "hot-reload")]
    if let Some(window) = web_sys::window() {
        hot_reload::setup_hot_reload(&window);
    }
}

pub fn init_worker() {
    init_logger();

    // #[cfg(debug_assertions)]
    // console_error_panic_hook::set_once();
}
