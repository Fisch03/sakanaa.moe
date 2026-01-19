use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
    routing::get,
};
use log::*;
use maud::html;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use wasmtime::*;

pub struct WasmService {
    state: Arc<WasmState>,
    _watcher: RecommendedWatcher,
    pkg_dir: PathBuf,
}

struct WasmState {
    engine: Engine,
    module: RwLock<Option<Module>>,
    wasm_path: PathBuf,
    // Channel to notify active WebSocket connections to reload
    reload_tx: broadcast::Sender<()>,
    // Store the last known file hash to dedup events
    last_hash: RwLock<u64>,
}

impl WasmService {
    pub fn new<P: Into<PathBuf>>(wasm_path: P) -> Self {
        let wasm_path = wasm_path.into();

        // Create a broadcast channel for reload signals
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
            last_hash: RwLock::new(0),
        });

        Self::load_wasm(&state);
        // Initialize hash
        if let Some(h) = Self::calculate_hash(&state.wasm_path) {
            *state.last_hash.write().unwrap() = h;
        }

        let watcher_state = state.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, _>| match res {
                Ok(event) => {
                    let is_target_file = event
                        .paths
                        .iter()
                        .any(|p| p.file_name() == watcher_state.wasm_path.file_name());

                    if !is_target_file {
                        return;
                    }

                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            thread::sleep(Duration::from_millis(250));

                            let new_hash =
                                Self::calculate_hash(&watcher_state.wasm_path).unwrap_or(0);
                            let mut last_hash = watcher_state.last_hash.write().unwrap();

                            if new_hash != *last_hash {
                                info!("wasm changed (hash mismatch), reloading...");
                                *last_hash = new_hash;
                                drop(last_hash);

                                Self::load_wasm(&watcher_state);

                                let _ = watcher_state.reload_tx.send(());
                            } else {
                                debug!("wasm event detected but hash identical, ignoring");
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => log::error!("watch error: {}", e),
            },
            Config::default(),
        )
        .expect("failed to create file watcher");

        if let Some(parent) = wasm_path.parent() {
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }

        let pkg_dir = wasm_path.parent().unwrap_or(&wasm_path).to_path_buf();

        WasmService {
            state,
            _watcher: watcher,
            pkg_dir,
        }
    }

    fn calculate_hash(path: &Path) -> Option<u64> {
        let bytes = std::fs::read(path).ok()?;
        let mut hasher = DefaultHasher::new();
        hasher.write(&bytes);
        Some(hasher.finish())
    }

    fn load_wasm(state: &WasmState) {
        if !state.wasm_path.exists() {
            warn!("wasm file does not exist yet, waiting for build...");
            return;
        }

        info!("loading wasm module from {:?}", state.wasm_path);
        match Module::from_file(&state.engine, &state.wasm_path) {
            Ok(m) => {
                let mut lock = state.module.write().unwrap();
                *lock = Some(m);
                info!("wasm module loaded successfully");
            }
            Err(e) => error!("failed to load wasm module: {}", e),
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/", get(render_handler))
            // Websocket route for HMR
            .route("/_ws", get(ws_handler))
            .fallback(get(render_handler))
            .nest_service("/pkg", ServeDir::new(&self.pkg_dir))
            .with_state(self.state.clone())
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WasmState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        // Subscribe to the reload broadcast channel
        let mut rx = state.reload_tx.subscribe();

        // Wait for a reload signal
        while let Ok(()) = rx.recv().await {
            if socket.send(Message::Text("reload".into())).await.is_err() {
                // Client disconnected
                break;
            }
        }
    })
}

async fn render_handler(
    State(state): State<Arc<WasmState>>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let module_guard = state.module.read().unwrap();
    let module = match module_guard.as_ref() {
        Some(m) => m,
        None => {
            return Html(html! {
                h1 { "wasm module not loaded (yet)." }
            })
            .into_response();
        }
    };

    let mut store = Store::new(&state.engine, ());
    let mut linker = Linker::new(&state.engine);

    for import in module.imports() {
        let name = import.name();
        let module_name = import.module();

        if name.contains("info") {
            linker
                .func_wrap(
                    module_name,
                    name,
                    |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                        let msg = read_wasm_string(&mut caller, ptr, len);
                        info!("[WASM]: {}", msg);
                    },
                )
                .unwrap();
        } else if name.contains("error") {
            linker
                .func_wrap(
                    module_name,
                    name,
                    |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                        let msg = read_wasm_string(&mut caller, ptr, len);
                        error!("[WASM]: {}", msg);
                    },
                )
                .unwrap();
        } else if name.contains("warn") {
            linker
                .func_wrap(
                    module_name,
                    name,
                    |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                        let msg = read_wasm_string(&mut caller, ptr, len);
                        warn!("[WASM]: {}", msg);
                    },
                )
                .unwrap();
        } else if name.contains("debug") {
            linker
                .func_wrap(
                    module_name,
                    name,
                    |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                        let msg = read_wasm_string(&mut caller, ptr, len);
                        debug!("[WASM]: {}", msg);
                    },
                )
                .unwrap();
        }
    }

    let Ok(()) = linker.define_unknown_imports_as_default_values(&mut store, module) else {
        return Html(html! {
            h1 { "failed to define imports for wasm module." }
        })
        .into_response();
    };

    let instance = linker.instantiate(&mut store, module).unwrap();

    // --- CALL SERVER RENDER ---
    let path_str = uri.path().to_string();

    // Allocate input string in WASM memory
    let input_str = WasmInputString::new(&mut store, &instance, &path_str);

    let render_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "render")
        .unwrap();

    let result_ptr = render_func
        .call(&mut store, (input_str.ptr, input_str.len))
        .unwrap();

    // Free the input string now that we are done with it
    input_str.dealloc(&mut store, &instance);

    // Read the output string from WASM memory
    let output_str = WasmOutputString::from_ptr(&mut store, &instance, result_ptr);
    let html_content = output_str.text.clone();

    // Free the output string
    output_str.dealloc(&mut store, &instance);

    Html(html_content).into_response()
}

fn read_wasm_string(caller: &mut Caller<'_, ()>, ptr: i32, len: i32) -> String {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let data = memory.data(&caller);
    let slice = &data[ptr as usize..(ptr + len) as usize];
    String::from_utf8_lossy(slice).to_string()
}

struct WasmInputString {
    pub ptr: i32,
    pub len: i32,
}

impl WasmInputString {
    fn new(store: &mut Store<()>, instance: &Instance, s: &str) -> Self {
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut *store, "alloc")
            .expect("wasm module must export 'alloc'");

        let len = s.len() as i32;
        let ptr = alloc.call(&mut *store, len).unwrap();

        let memory = instance.get_memory(&mut *store, "memory").unwrap();
        memory
            .write(&mut *store, ptr as usize, s.as_bytes())
            .unwrap();

        Self { ptr, len }
    }

    fn dealloc(self, store: &mut Store<()>, instance: &Instance) {
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc")
            .expect("wasm module must export 'dealloc'");
        dealloc.call(store, (self.ptr, self.len)).unwrap();
    }
}

struct WasmOutputString {
    pub ptr: i32,
    pub total_len: i32,
    pub text: String,
}

impl WasmOutputString {
    fn from_ptr(store: &mut Store<()>, instance: &Instance, ptr: i32) -> Self {
        let memory = instance.get_memory(&mut *store, "memory").unwrap();
        let data = memory.data(&store);

        let len_bytes = &data[ptr as usize..(ptr as usize + 4)];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

        let start = ptr as usize + 4;
        let end = start + len;
        let str_bytes = &data[start..end];
        let text = String::from_utf8_lossy(str_bytes).to_string();

        Self {
            ptr,
            total_len: (len + 4) as i32,
            text,
        }
    }

    fn dealloc(self, store: &mut Store<()>, instance: &Instance) {
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc")
            .expect("wasm module must export 'dealloc'");
        dealloc.call(store, (self.ptr, self.total_len)).unwrap();
    }
}
