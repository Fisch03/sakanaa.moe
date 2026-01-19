use anyhow::Result;
use lol_html::{element, HtmlRewriter, Settings};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use wasmtime::*;

pub fn execute_wasm_render(engine: &Engine, module: &Module) -> Result<String, String> {
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
    let input_str = WasmInputString::new(&mut store, &instance, "/")
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

pub fn execute_wasm_style(engine: &Engine, module: &Module) -> Result<String, String> {
    let mut store = Store::new(engine, ());
    let mut linker = Linker::new(engine);

    setup_imports(&mut linker, module).map_err(|e| e.to_string())?;

    linker
        .define_unknown_imports_as_default_values(&mut store, module)
        .map_err(|e| format!("failed to define imports: {}", e))?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| format!("failed to instantiate: {}", e))?;

    // Call style
    let style_func = instance
        .get_typed_func::<(), i32>(&mut store, "style")
        .map_err(|e| format!("missing style export: {}", e))?;

    let result_ptr = style_func
        .call(&mut store, ())
        .map_err(|e| format!("style call failed: {}", e))?;

    // Read Output
    let output_str = WasmOutputString::from_ptr(&mut store, &instance, result_ptr)
        .map_err(|e| format!("read output failed: {}", e))?;

    let result = output_str.text.clone();
    output_str.dealloc(&mut store, &instance);

    Ok(result)
}

pub fn sanitize_html(html: &str) -> String {
    let mut output = Vec::new();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![element!("*[data-hmr-ignore]", |el| {
                el.remove();
                Ok(())
            })],
            ..Settings::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );

    let _ = rewriter.write(html.as_bytes());
    let _ = rewriter.end();

    String::from_utf8(output).unwrap_or_else(|_| html.to_string())
}

pub fn calculate_string_hash(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn setup_imports(linker: &mut Linker<()>, module: &Module) -> Result<()> {
    for import in module.imports() {
        let name = import.name();
        let module_name = import.module();

        if name.starts_with("__wbindgen") {
            continue;
        }

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
    fn new(store: &mut Store<()>, instance: &Instance, s: &str) -> Result<Self> {
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
    fn from_ptr(store: &mut Store<()>, instance: &Instance, ptr: i32) -> Result<Self> {
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