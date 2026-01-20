use log::debug;
use maud::html;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bridge::{Element, ElementCollection, Window};

struct State {
    velocity: f64,
    current_y: f64,
    offsets: (f64, f64),
    row_counts: (i32, i32),
    logical_size: f64,
    alive: bool,
}

pub struct AlbumGrid {
    state: Rc<RefCell<State>>,
    _wheel_closure: Closure<dyn FnMut(web_sys::WheelEvent)>,
    _resize_closure: Closure<dyn FnMut(web_sys::UiEvent)>,
}

impl AlbumGrid {
    const ANGLE_DEG: f64 = 30.0;
    const FRICTION: f64 = 0.97;
    const AUTO_SCROLL: f64 = -0.01;

    fn cos_a() -> f64 {
        (Self::ANGLE_DEG.to_radians()).cos()
    }

    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(State {
            velocity: -40.0,
            current_y: 0.0,
            offsets: (0.0, 0.0),
            row_counts: (0, 0),
            logical_size: 170.0,
            alive: true,
        }));

        let window = Window::get();
        let grid_elem = Element::by_id("album_grid").expect("album_grid not found");
        grid_elem.add_class("active");

        // Initial setup
        Self::initialize_dom(&grid_elem, state.clone());

        // Register wheel listener
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

    fn initialize_dom(grid: &Element, state: Rc<RefCell<State>>) {
        let (w, h) = Window::get().size().unwrap_or((800.0, 600.0));

        // Probe for dynamic sizes
        grid.set_inner_html("<div class=\"album_column\"><div class=\"album\"></div><div class=\"album\"></div></div><div class=\"album_column\"></div>");

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

        {
            let mut s = state.borrow_mut();
            s.logical_size = l_size;
            s.velocity = -20.0;
            s.current_y = 0.0;
            s.row_counts = (0, 0);
            s.offsets = (initial_offset, initial_offset);
        }

        grid.set_style("--scroll-y", "0px");
        grid.set_style("--offset-0", &format!("{}px", initial_offset));
        grid.set_style("--offset-1", &format!("{}px", initial_offset));
    }
}

impl State {
    fn update(&mut self) {
        let win_h = Window::get().inner_height().unwrap_or(800.0);

        // Velocity physics
        self.velocity *= AlbumGrid::FRICTION;
        self.velocity += AlbumGrid::AUTO_SCROLL;

        self.current_y += self.velocity;

        let curr_y = self.current_y;
        let grid = Element::by_id("album_grid").unwrap();
        grid.set_style("--scroll-y", &format!("{}px", curr_y));

        let cols = ElementCollection::select(".album_column");
        let logical_size = self.logical_size;
        let projected_size = logical_size * AlbumGrid::cos_a();

        let (mut off0, mut off1) = self.offsets;
        let (mut rows0, mut rows1) = self.row_counts;

        for parity in 0..2 {
            let current_off = if parity == 0 { &mut off0 } else { &mut off1 };
            let current_rows = if parity == 0 { &mut rows0 } else { &mut rows1 };
            let dir = if parity == 0 { 1.0 } else { -1.0 };

            let buffer = projected_size * 2.0;

            loop {
                let p_top = (curr_y * dir + *current_off) * AlbumGrid::cos_a();
                let col_h_p = (*current_rows as f64) * projected_size;

                if p_top > -buffer {
                    self.add_to_columns(&cols, parity, true);
                    *current_off -= logical_size;

                    if p_top + col_h_p > win_h + buffer + projected_size {
                        self.remove_from_columns(&cols, parity, false);
                    } else {
                        *current_rows += 1;
                    }
                } else if p_top + col_h_p < win_h + buffer {
                    self.add_to_columns(&cols, parity, false);
                    if p_top < -buffer - projected_size {
                        self.remove_from_columns(&cols, parity, true);
                        *current_off += logical_size;
                    } else {
                        *current_rows += 1;
                    }
                } else if *current_rows > 0
                    && p_top < -buffer - projected_size
                    && p_top + col_h_p > win_h + buffer + projected_size
                {
                    if p_top.abs() > (p_top + col_h_p - win_h).abs() {
                        self.remove_from_columns(&cols, parity, true);
                        *current_off += logical_size;
                    } else {
                        self.remove_from_columns(&cols, parity, false);
                    }
                    *current_rows -= 1;
                } else {
                    break;
                }
            }
        }

        self.offsets = (off0, off1);
        self.row_counts = (rows0, rows1);
        grid.set_style("--offset-0", &format!("{}px", off0));
        grid.set_style("--offset-1", &format!("{}px", off1));
    }

    fn add_to_columns(&self, cols: &ElementCollection, parity: usize, prepend: bool) {
        cols.iter()
            .enumerate()
            .filter(|(j, _)| j % 2 == parity)
            .for_each(|(_, col)| {
                let album = Element::create("div");
                album.add_class("album");
                if prepend {
                    col.prepend(&album);
                } else {
                    col.append(&album);
                }
            });
    }

    fn remove_from_columns(&self, cols: &ElementCollection, parity: usize, first: bool) -> bool {
        let mut removed = false;
        cols.iter()
            .enumerate()
            .filter(|(j, _)| j % 2 == parity)
            .for_each(|(_, col)| {
                let to_remove = if first {
                    col.first_child()
                } else {
                    col.last_child()
                };
                if let Some(child) = to_remove {
                    child.remove();
                    removed = true;
                }
            });
        removed
    }
}

impl Drop for AlbumGrid {
    fn drop(&mut self) {
        debug!("Dropping AlbumGrid!");

        self.state.borrow_mut().alive = false;

        let window = Window::get();
        window.remove_listener("wheel", self._wheel_closure.as_ref());
        window.remove_listener("resize", self._resize_closure.as_ref());

        if let Some(grid) = Element::by_id("album_grid") {
            grid.remove_class("active");
            grid.set_inner_html("");
        }
    }
}

