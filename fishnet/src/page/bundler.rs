use std::path::Path;
use std::sync::Arc;

use tracing::{debug, info, instrument};

use esbuild_rs::{transform, Format, TransformOptionsBuilder};

/// A JavaScript source file
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum ScriptType {
    /// An inline script
    Inline(String),
    /// An external script file. When this is used, the [`Website`](crate::website::Website) should be configured to serve static files.
    External(String),
}

#[instrument(skip_all, level = "debug")]
pub async fn bundle_script(script: &ScriptType) -> String {
    let start = std::time::Instant::now();

    let mut options = TransformOptionsBuilder::new();
    options.format = Format::IIFE;
    options.minify_syntax = true;
    options.minify_whitespace = true;
    options.minify_identifiers = true;
    let options = options.build();

    let script: Vec<u8> = match script {
        ScriptType::Inline(script) => {
            debug!("minifying inline script");
            script.as_bytes().to_vec()
        }
        ScriptType::External(path) => {
            let path = Path::new("static/").join(path);

            debug!(?path, "minifying external script");

            std::fs::read(&path).unwrap()
        }
    };

    let in_size = script.len();
    let script = transform(Arc::new(script), options.clone()).await;

    let script_out = script.code.to_string();
    debug!(
        "minfied script, {:?} bytes -> {:?} bytes. took {:?}",
        in_size,
        script_out.len(),
        start.elapsed()
    );

    script.code.to_string()
}
