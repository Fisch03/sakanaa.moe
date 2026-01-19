use log::{Level, Log, Metadata, Record};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn host_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn host_error(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    pub fn host_warn(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = info)]
    pub fn host_info(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    pub fn host_debug(s: &str);
}

struct WasmBridgeLogger;

impl Log for WasmBridgeLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());

            match record.level() {
                Level::Error => host_error(&msg),
                Level::Warn => host_warn(&msg),
                Level::Info => host_info(&msg),
                Level::Debug => host_debug(&msg),
                Level::Trace => host_debug(&msg),
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: WasmBridgeLogger = WasmBridgeLogger;

pub fn init_logger() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Trace)
    }
}
