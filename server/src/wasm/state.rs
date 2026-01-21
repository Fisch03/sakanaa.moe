use serde::{Deserialize, Serialize, Serializer};
use std::{path::PathBuf, sync::RwLock};
use tokio::sync::broadcast;
use wasmtime::{Engine, Module};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
pub struct WasmHash(pub u64);

impl Serialize for WasmHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WasmUpdate {
    pub html_hash: WasmHash,
    pub css_hash: WasmHash,
    pub css: String,
}

/// Shared state for the WASM service.
pub struct WasmState {
    pub engine: Engine,
    pub module: RwLock<Option<Module>>,
    pub wasm_path: PathBuf,
    pub reload_tx: broadcast::Sender<WasmUpdate>,
    pub file_hash: RwLock<WasmHash>,
    pub html_hash: RwLock<WasmHash>,
    pub css_hash: RwLock<WasmHash>,
    pub css: RwLock<String>,
}
