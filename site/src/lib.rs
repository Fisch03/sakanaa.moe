use maud::{DOCTYPE, html};
use wasm_bindgen::prelude::*;
use wasm_bridge::Window;

pub mod components {
    pub mod head;
}

pub mod album_grid;
use album_grid::AlbumGrid;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: lol_alloc::AssumeSingleThreaded<lol_alloc::FreeListAllocator> =
    unsafe { lol_alloc::AssumeSingleThreaded::new(lol_alloc::FreeListAllocator::new()) };

#[unsafe(no_mangle)]
pub extern "C" fn style() -> *mut u8 {
    static CSS: &str = grass::include!("site/style/main.scss");
    wasm_bridge::to_host_string(CSS.to_string())
}

/// # Safety
/// path_ptr must point to a valid UTF-8 string of length path_len
#[unsafe(no_mangle)]
pub unsafe extern "C" fn render(path_ptr: *const u8, path_len: usize) -> *mut u8 {
    wasm_bridge::init();
    let path = unsafe { wasm_bridge::from_host_string(path_ptr, path_len) };
    let html = html! {
        (DOCTYPE)
        html {
            head {
                title { "sakanaa!" }
                (components::head::shared())
            }
            body {
                #album_grid onclick="window.trigger_action()" {}
                #main {
                    .column {
                        .container {
                            h1 { "hello world!" }
                            p { "path: " (path) }
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

#[wasm_bindgen]
pub fn run() {
    wasm_bridge::init();

    let mut grid = None::<AlbumGrid>;
    let window = Window::get();

    window.attach_fn("trigger_action", move || {
        if grid.take().is_none() {
            grid = Some(AlbumGrid::new());
        }
    });
}
