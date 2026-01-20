mod abi;
#[cfg(feature = "hot-reload")]
mod hot_reload;
mod logger;

pub mod dom;
pub mod window;
use maud::{Markup, html};

pub use abi::{alloc, dealloc, from_host_string, to_host_string};
pub use dom::{Element, ElementCollection, document};
pub use logger::init_logger;
pub use window::Window;

pub fn init() {
    init_logger();
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    #[cfg(feature = "hot-reload")]
    if let Some(window) = web_sys::window() {
        hot_reload::setup_hot_reload(&window);
    }
}

pub fn load_wasm_module() -> Markup {
    html! {
        script type="module" {
            @if cfg!(feature = "hot-reload") {
                (maud::PreEscaped(r#"
                import init from '/pkg/site.js';
                window._reload_wasm = async () => {
                    const timestamp = Date.now();
                    const mod = await import(`/pkg/site.js?t=${timestamp}`);
                    await mod.default();
                };
                init();
                "#))
            } @else {
                "import init from '/pkg/site.js'; init();"
            }
        }
    }
}
