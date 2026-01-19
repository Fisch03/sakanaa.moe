mod handlers;
mod runtime;
mod state;

pub use state::WasmState;

use axum::{Router, routing::get};
use log::*;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use wasmtime::*;

use handlers::{render_handler, style_handler, ws_handler};
use runtime::{calculate_string_hash, execute_wasm_render, execute_wasm_style, sanitize_html};
use state::{WasmHash, WasmUpdate};

/// Main service struct handling the WASM runtime and file watching.
pub struct WasmService {
    state: Arc<WasmState>,
    _watcher: Debouncer<RecommendedWatcher>,
    pkg_dir: PathBuf,
}

impl WasmService {
    pub fn new<P: Into<PathBuf>>(wasm_path: P) -> Self {
        let wasm_path = wasm_path.into();
        let (reload_tx, _) = broadcast::channel(16);

        let mut config = wasmtime::Config::new();
        #[cfg(debug_assertions)]
        {
            config.cranelift_opt_level(OptLevel::None);
            config.cranelift_regalloc_algorithm(RegallocAlgorithm::SinglePass);
        }

        let engine = Engine::new(&config).expect("failed to create wasmtime engine");

        let state = Arc::new(WasmState {
            engine: engine.clone(),
            module: RwLock::new(None),
            wasm_path: wasm_path.clone(),
            reload_tx,
            file_hash: RwLock::new(WasmHash(0)),
            html_hash: RwLock::new(WasmHash(0)),
            css_hash: RwLock::new(WasmHash(0)),
            css: RwLock::new(String::new()),
        });

        // Initial load
        Self::load_wasm(&state);
        if let Some(h) = Self::calculate_hash(&state.wasm_path) {
            *state.file_hash.write().unwrap() = h;
        }

        // Setup watcher
        let watcher = Self::setup_watcher(state.clone());
        let pkg_dir = wasm_path.parent().unwrap_or(&wasm_path).to_path_buf();

        WasmService {
            state,
            _watcher: watcher,
            pkg_dir,
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/", get(render_handler))
            .route("/style/main.css", get(style_handler))
            .route("/_ws", get(ws_handler))
            .fallback(get(render_handler))
            .nest_service("/pkg", ServeDir::new(&self.pkg_dir))
            .with_state(self.state.clone())
    }

    fn setup_watcher(state: Arc<WasmState>) -> Debouncer<RecommendedWatcher> {
        let watcher_state = state.clone();
        let rt_handle = tokio::runtime::Handle::current();

        let mut watcher = new_debouncer(
            Duration::from_millis(10),
            move |res: DebounceEventResult| match res {
                Ok(events) => Self::handle_watch_event(&watcher_state, events, &rt_handle),
                Err(e) => log::error!("watch error: {:?}", e),
            },
        )
        .expect("failed to create debounced file watcher");

        if let Some(parent) = state.wasm_path.parent() {
            let _ = watcher.watcher().watch(parent, RecursiveMode::NonRecursive);
        }
        watcher
    }

    fn handle_watch_event(
        state: &Arc<WasmState>,
        events: Vec<notify_debouncer_mini::DebouncedEvent>,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let is_target = events.iter().any(|e| {
            e.path
                .file_name()
                .map_or(false, |n| n == state.wasm_path.file_name().unwrap())
        });

        if !is_target {
            return;
        }

        let state_clone = state.clone();
        rt_handle.spawn_blocking(move || {
            Self::check_and_reload(&state_clone);
        });
    }

    fn check_and_reload(state: &Arc<WasmState>) {
        let new_file_hash = Self::calculate_hash(&state.wasm_path).unwrap_or(WasmHash(0));
        let mut file_hash_guard = state.file_hash.write().unwrap();

        if new_file_hash != *file_hash_guard {
            info!("wasm changed (hash mismatch), reloading and checking for changes...");
            *file_hash_guard = new_file_hash;
            drop(file_hash_guard); // release lock before loading

            if let Some(new_module) = Self::load_wasm_module(state) {
                let new_css = execute_wasm_style(&state.engine, &new_module).unwrap_or_default();
                let new_html =
                    execute_wasm_render(&state.engine, &new_module).unwrap_or_default();

                let new_css_hash = WasmHash(calculate_string_hash(&new_css));
                let new_html_hash = WasmHash(calculate_string_hash(&sanitize_html(&new_html)));

                *state.html_hash.write().unwrap() = new_html_hash;
                *state.css_hash.write().unwrap() = new_css_hash;
                *state.css.write().unwrap() = new_css.clone();
                *state.module.write().unwrap() = Some(new_module);

                let update = WasmUpdate {
                    html_hash: new_html_hash,
                    css_hash: new_css_hash,
                    css: new_css,
                };

                let _ = state.reload_tx.send(update);
                debug!(
                    "broadcasted wasm update (html: {:?}, css: {:?})",
                    new_html_hash, new_css_hash
                );
            }
        } else {
            debug!("wasm event detected but hash identical, ignoring");
        }
    }

    fn calculate_hash(path: &Path) -> Option<WasmHash> {
        let bytes = std::fs::read(path).ok()?;
        let mut hasher = DefaultHasher::new();
        hasher.write(&bytes);
        Some(WasmHash(hasher.finish()))
    }

    fn load_wasm_module(state: &WasmState) -> Option<Module> {
        if !state.wasm_path.exists() {
            warn!("wasm file does not exist yet, waiting for build...");
            return None;
        }

        info!("loading wasm module from {:?}", state.wasm_path);
        match Module::from_file(&state.engine, &state.wasm_path) {
            Ok(m) => Some(m),
            Err(e) => {
                error!("failed to load wasm module: {}", e);
                None
            }
        }
    }

    fn load_wasm(state: &WasmState) {
        if let Some(module) = Self::load_wasm_module(state) {
            let css = execute_wasm_style(&state.engine, &module).unwrap_or_default();
            let html = execute_wasm_render(&state.engine, &module).unwrap_or_default();

            *state.css_hash.write().unwrap() = WasmHash(calculate_string_hash(&css));
            *state.html_hash.write().unwrap() =
                WasmHash(calculate_string_hash(&sanitize_html(&html)));
            *state.css.write().unwrap() = css;
            *state.module.write().unwrap() = Some(module);
            info!("wasm module loaded successfully");
        }
    }
}

