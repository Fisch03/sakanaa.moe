use std::{cell::RefCell, rc::Rc};
use wasm_bridge::{Element, ElementCollection, Window, Worker};

use super::messages::WorkerMessage;
use super::AlbumGrid;

pub struct AlbumGridState {
    pub velocity: f64,
    pub current_y: f64,
    pub offsets: (f64, f64),
    pub row_counts: (i32, i32),
    pub logical_size: f64,
    pub alive: bool,
    pub worker: Rc<RefCell<Worker>>,
    pub worker_ready: bool,
    pub pending_messages: Vec<WorkerMessage>,
}

impl AlbumGridState {
    pub fn new(worker: Rc<RefCell<Worker>>) -> Self {
        Self {
            velocity: -40.0,
            current_y: 0.0,
            offsets: (0.0, 0.0),
            row_counts: (0, 0),
            logical_size: 170.0,
            alive: true,
            worker,
            worker_ready: false,
            pending_messages: Vec::new(),
        }
    }

    pub fn update(&mut self) {
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

    fn add_to_columns(&mut self, cols: &ElementCollection, parity: usize, prepend: bool) {
        cols.iter()
            .enumerate()
            .filter(|(j, _)| j % 2 == parity)
            .for_each(|(_, col)| {
                let id: u64 = rand::random();
                let id_str = id.to_string();

                let album = Element::create("div");
                album.add_class("album");
                album.set_attribute("id", &format!("album-{}", id_str));

                self.send_to_worker(WorkerMessage::Fetch { id: id_str });

                if prepend {
                    col.prepend(&album);
                } else {
                    col.append(&album);
                }
            });
    }

    pub fn send_to_worker(&mut self, msg: WorkerMessage) {
        if self.worker_ready {
            let _ = self.worker.borrow().post_message(&msg);
        } else {
            self.pending_messages.push(msg);
        }
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
