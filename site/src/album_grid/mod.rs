use log::debug;
use maud::html;
use std::{cell::RefCell, f64::consts::PI, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bridge::{Element, ElementCollection, Window, Worker, document};
use web_sys::HtmlCanvasElement;

mod messages;
mod state;
mod worker;

use messages::{MainMessage, WorkerMessage};
use state::AlbumGridState;

pub struct AlbumGrid {
    state: Rc<RefCell<AlbumGridState>>,
    _wheel_closure: Closure<dyn FnMut(web_sys::WheelEvent)>,
    _resize_closure: Closure<dyn FnMut(web_sys::UiEvent)>,
}

impl AlbumGrid {
    pub const ANGLE_DEG: f64 = 30.0;
    pub const FRICTION: f64 = 0.97;
    pub const AUTO_SCROLL: f64 = -0.01;

    pub fn cos_a() -> f64 {
        (Self::ANGLE_DEG.to_radians()).cos()
    }

    pub fn new() -> Self {
        let worker = Rc::new(RefCell::new(
            Worker::new_rust_worker("album_worker_entry").expect("Failed to create album worker"),
        ));
        let state = Rc::new(RefCell::new(AlbumGridState::new(worker.clone())));

        // setup worker message handler
        {
            let mut w = worker.borrow_mut();
            let state_clone = state.clone();
            w.set_onmessage(move |msg: MainMessage| match msg {
                MainMessage::Ready => {
                    log::info!("album worker is ready");
                    let mut state = state_clone.borrow_mut();
                    state.worker_ready = true;
                    let messages = std::mem::take(&mut state.pending_messages);
                    for msg in messages {
                        let _ = state.worker.borrow().post_message(&msg);
                    }
                }
                MainMessage::Cover { id, data_url } => {
                    if let Some(album_elem) = Element::by_id(&format!("album-{}", id)) {
                        album_elem.set_style("background-image", &format!("url('{}')", data_url));
                        album_elem.set_style("background-size", "cover");
                    }
                }
            });
        }

        let window = Window::get();
        let grid_elem = Element::by_id("album_grid").expect("album_grid not found");
        grid_elem.add_class("active");

        // initial setup
        Self::initialize_dom(&grid_elem, state.clone());

        // register wheel listener
        let s_clone = state.clone();
        let _wheel_closure = window.add_wheel_event_listener(move |e| {
            let mut dy = e.delta_y();
            match e.delta_mode() {
                web_sys::WheelEvent::DOM_DELTA_LINE => dy *= 33.0,
                web_sys::WheelEvent::DOM_DELTA_PAGE => {
                    dy *= Window::get().inner_height().unwrap_or(800.0)
                }
                _ => {}
            };
            s_clone.borrow_mut().velocity += dy * 0.02;
        });

        // Register resize listener
        let s_clone = state.clone();
        let _resize_closure = window.add_resize_event_listener(move |_| {
            let grid_elem = Element::by_id("album_grid").expect("album_grid not found");
            Self::initialize_dom(&grid_elem, s_clone.clone());
        });

        // Start animation loop
        let s_loop = state.clone();
        window.start_animation_loop(move |_| {
            let mut s = s_loop.borrow_mut();
            if !s.alive {
                return false;
            }
            s.update();
            true
        });

        Self {
            state,
            _wheel_closure,
            _resize_closure,
        }
    }

    fn generate_placeholder(size: f64, primary: &str, secondary: &str) -> String {
        let document = document();
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        // High DPI scaling (optional but looks better)
        let render_size = size / 2.0;
        let pixel_size = render_size.ceil() as u32;

        canvas.set_width(pixel_size);
        canvas.set_height(pixel_size);

        if let Some(ctx) = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()
        {
            ctx.set_image_smoothing_enabled(false);
            ctx.set_stroke_style_str(primary);
            ctx.set_fill_style_str(secondary);
            ctx.set_line_width(1.0);

            let center = render_size / 2.0;
            let r_outer = center;
            let r_hub = r_outer * 0.35;
            let r_hole = r_outer * 0.15;

            // Clear
            ctx.clear_rect(0.0, 0.0, render_size, render_size);

            // 1. outer rim
            ctx.begin_path();
            ctx.arc(center, center, r_outer - 0.5, 0.0, PI * 2.0)
                .unwrap();
            ctx.stroke();

            // hole
            ctx.begin_path();
            ctx.arc(center, center, r_hole, 0.0, PI * 2.0).unwrap();
            ctx.stroke();
        }

        canvas.to_data_url().unwrap_or_default()
    }

    fn initialize_dom(grid: &Element, state: Rc<RefCell<AlbumGridState>>) {
        let (w, h) = Window::get().size().unwrap_or((800.0, 600.0));

        // probe size
        grid.set_inner_html(
            &html! {
                .album_column {
                    .album {}
                    .album {}
                }
                .album_column {}
            }
            .into_string(),
        );

        let albums = ElementCollection::select("#album_grid .album");
        let columns = ElementCollection::select("#album_grid .album_column");

        let l_size = if let (Some(a1), Some(a2)) = (albums.iter().nth(0), albums.iter().nth(1)) {
            (a2.rect().1 - a1.rect().1).abs() / Self::cos_a()
        } else {
            170.0
        };

        let c_width = if let (Some(c1), Some(c2)) = (columns.iter().nth(0), columns.iter().nth(1)) {
            (c2.rect().0 - c1.rect().0).abs()
        } else {
            100.0
        };

        let album_width = if let Some(a) = albums.iter().next() {
            a.rect().2
        } else {
            100.0
        };

        grid.set_inner_html("");

        let cols_count = (w / c_width).ceil() as i32 + 10;
        let center_c = cols_count / 2;

        let markup = html! {
            @for c in 0..cols_count {
                .album_column style={"--anim-idx:" ((c - center_c).abs())} {}
            }
        };
        grid.set_inner_html(&markup.into_string());

        let initial_offset = (h / 2.0) / Self::cos_a();

        let (primary, secondary) = {
            let style = Window::get()
                .get_computed_style(&document().body().unwrap())
                .unwrap()
                .unwrap();
            (
                style.get_property_value("--primary-col").unwrap(),
                style.get_property_value("--secondary-col").unwrap(),
            )
        };

        // Generate and set placeholder
        let placeholder_url = Self::generate_placeholder(album_width, &primary, &secondary);
        grid.set_style("--cd-placeholder", &format!("url('{}')", placeholder_url));

        {
            let mut s = state.borrow_mut();
            s.logical_size = l_size;
            s.velocity = -20.0;
            s.current_y = 0.0;
            s.row_counts = (0, 0);
            s.offsets = (initial_offset, initial_offset);

            s.send_to_worker(WorkerMessage::SetSize {
                size: album_width,
                primary,
                secondary,
            });
        }

        grid.set_style("--scroll-y", "0px");
        grid.set_style("--offset-0", &format!("{}px", initial_offset));
        grid.set_style("--offset-1", &format!("{}px", initial_offset));
    }
}

impl Drop for AlbumGrid {
    fn drop(&mut self) {
        debug!("dropping AlbumGrid!");

        self.state.borrow_mut().alive = false;
        self.state.borrow_mut().worker.borrow().terminate();

        let window = Window::get();
        window.remove_listener("wheel", self._wheel_closure.as_ref());
        window.remove_listener("resize", self._resize_closure.as_ref());

        if let Some(grid) = Element::by_id("album_grid") {
            grid.remove_class("active");
            grid.set_inner_html("");
        }
    }
}

impl Default for AlbumGrid {
    fn default() -> Self {
        Self::new()
    }
}
