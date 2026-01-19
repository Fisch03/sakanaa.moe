use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocketUpgrade},
    },
    http::{StatusCode, Uri},
    response::{Html, IntoResponse, Response},
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

/// Main service struct handling the WASM runtime and file watching.
pub struct WasmService {
    state: Arc<WasmState>,
    _watcher: RecommendedWatcher,
    pkg_dir: PathBuf,
}

/// Shared state for the WASM service.
struct WasmState {
    engine: Engine,
    module: RwLock<Option<Module>>,
    wasm_path: PathBuf,
    reload_tx: broadcast::Sender<()>,
    last_hash: RwLock<u64>,
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
            last_hash: RwLock::new(0),
        });

        // Initial load
        Self::load_wasm(&state);
        if let Some(h) = Self::calculate_hash(&state.wasm_path) {
            *state.last_hash.write().unwrap() = h;
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
            .route("/_ws", get(ws_handler))
            .fallback(get(render_handler))
            .nest_service("/pkg", ServeDir::new(&self.pkg_dir))
            .with_state(self.state.clone())
    }

    fn setup_watcher(state: Arc<WasmState>) -> RecommendedWatcher {
        let watcher_state = state.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, _>| match res {
                Ok(event) => Self::handle_watch_event(&watcher_state, event),
                Err(e) => log::error!("watch error: {}", e),
            },
            Config::default(),
        )
        .expect("failed to create file watcher");

        if let Some(parent) = state.wasm_path.parent() {
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }
        watcher
    }

    fn handle_watch_event(state: &Arc<WasmState>, event: Event) {
        let is_target = event
            .paths
            .iter()
            .any(|p| p.file_name() == state.wasm_path.file_name());

        if !is_target {
            return;
        }

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Debounce slightly
                thread::sleep(Duration::from_millis(250));

                let new_hash = Self::calculate_hash(&state.wasm_path).unwrap_or(0);
                let mut last_hash = state.last_hash.write().unwrap();

                if new_hash != *last_hash {
                    info!("wasm changed (hash mismatch), reloading...");
                    *last_hash = new_hash;
                    drop(last_hash); // release lock before loading

                    Self::load_wasm(state);
                    let _ = state.reload_tx.send(());
                } else {
                    debug!("wasm event detected but hash identical, ignoring");
                }
            }
            _ => {}
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
                *state.module.write().unwrap() = Some(m);
                info!("wasm module loaded successfully");
            }
            Err(e) => error!("failed to load wasm module: {}", e),
        }
    }
}

// --- Handlers ---

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WasmState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.reload_tx.subscribe();
        while let Ok(()) = rx.recv().await {
            if socket.send(Message::Text("reload".into())).await.is_err() {
                break;
            }
        }
    })
}

async fn render_handler(State(state): State<Arc<WasmState>>, uri: Uri) -> Response {
    let module_guard = state.module.read().unwrap();
    let Some(module) = module_guard.as_ref() else {
        return Html(html! { h1 { "wasm module not loaded (yet)." } }).into_response();
    };

    match execute_wasm_render(&state.engine, module, uri.path()) {
        Ok(html_content) => Html(html_content).into_response(),
        Err(e) => {
            error!("WASM Render Error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", e),
            )
                .into_response()
        }
    }
}

// --- WASM Execution ---

fn execute_wasm_render(engine: &Engine, module: &Module, path_str: &str) -> Result<String, String> {
    let mut store = Store::new(engine, ());
    let mut linker = Linker::new(engine);

    setup_imports(&mut linker, module).map_err(|e| e.to_string())?;

    linker
        .define_unknown_imports_as_default_values(&mut store, module)
        .map_err(|e| format!("failed to define imports: {}", e))?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| format!("failed to instantiate: {}", e))?;

    // Allocation
    let input_str = WasmInputString::new(&mut store, &instance, path_str)
        .map_err(|e| format!("alloc failed: {}", e))?;

    // Call render
    let render_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "render")
        .map_err(|e| format!("missing render export: {}", e))?;

    let result_ptr = render_func
        .call(&mut store, (input_str.ptr, input_str.len))
        .map_err(|e| format!("render call failed: {}", e))?;

    input_str.dealloc(&mut store, &instance);

    // Read Output
    let output_str = WasmOutputString::from_ptr(&mut store, &instance, result_ptr)
        .map_err(|e| format!("read output failed: {}", e))?;

    let result = output_str.text.clone();
    output_str.dealloc(&mut store, &instance);

    Ok(result)
}

fn setup_imports(linker: &mut Linker<()>, module: &Module) -> anyhow::Result<()> {
    for import in module.imports() {
        let name = import.name();
        let module_name = import.module();

        // Helper macro to reduce repetition
        macro_rules! log_import {
            ($level:ident) => {
                linker.func_wrap(
                    module_name,
                    name,
                    |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                        let msg = read_wasm_string_raw(&mut caller, ptr, len);
                        log::$level!("[WASM]: {}", msg);
                    },
                )
            };
        }

        if name.contains("info") {
            log_import!(info)?;
        } else if name.contains("error") {
            log_import!(error)?;
        } else if name.contains("warn") {
            log_import!(warn)?;
        } else if name.contains("debug") {
            log_import!(debug)?;
        }
    }
    Ok(())
}

// --- ABI & Memory Helpers ---

fn read_wasm_string_raw(caller: &mut Caller<'_, ()>, ptr: i32, len: i32) -> String {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let data = memory.data(caller);
    let slice = &data[ptr as usize..(ptr + len) as usize];
    String::from_utf8_lossy(slice).to_string()
}

struct WasmInputString {
    pub ptr: i32,
    pub len: i32,
}

impl WasmInputString {
    fn new(store: &mut Store<()>, instance: &Instance, s: &str) -> anyhow::Result<Self> {
        let alloc = instance.get_typed_func::<i32, i32>(&mut *store, "alloc")?;

        let len = s.len() as i32;
        let ptr = alloc.call(&mut *store, len)?;

        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow::anyhow!("memory export not found"))?;

        memory.write(&mut *store, ptr as usize, s.as_bytes())?;

        Ok(Self { ptr, len })
    }

    fn dealloc(self, store: &mut Store<()>, instance: &Instance) {
        if let Ok(dealloc) = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc") {
            let _ = dealloc.call(store, (self.ptr, self.len));
        }
    }
}

struct WasmOutputString {
    pub ptr: i32,
    pub total_len: i32,
    pub text: String,
}

impl WasmOutputString {
    fn from_ptr(store: &mut Store<()>, instance: &Instance, ptr: i32) -> anyhow::Result<Self> {
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow::anyhow!("memory export not found"))?;
        let data = memory.data(store);

        if ptr < 0 || (ptr as usize) + 4 > data.len() {
            return Err(anyhow::anyhow!("invalid output pointer"));
        }

        let len_bytes = &data[ptr as usize..(ptr as usize + 4)];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

        let start = ptr as usize + 4;
        let end = start + len;

        if end > data.len() {
            return Err(anyhow::anyhow!("output string out of bounds"));
        }

        let str_bytes = &data[start..end];
        let text = String::from_utf8_lossy(str_bytes).to_string();

        Ok(Self {
            ptr,
            total_len: (len + 4) as i32,
            text,
        })
    }

    fn dealloc(self, store: &mut Store<()>, instance: &Instance) {
        if let Ok(dealloc) = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc") {
            let _ = dealloc.call(store, (self.ptr, self.total_len));
        }
    }
}

