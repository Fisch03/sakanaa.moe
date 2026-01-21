use maud::{Markup, html};

pub fn load_wasm_module() -> Markup {
    html! {
        script type="module" {
            @if cfg!(feature = "hot-reload") {
                (maud::PreEscaped(r#"
                // HMR Worker Cleanup Logic
                window._active_workers = [];
                window._register_worker = (w) => {
                    window._active_workers.push(w);
                };
                window._cleanup_workers = () => {
                    if (window._active_workers) {
                         console.log("cleaning up " + window._active_workers.length + " old workers...");
                         window._active_workers.forEach(w => w.terminate());
                         window._active_workers = [];
                    }
                };

                window._reload_wasm = async () => {
                    // Cleanup workers before reloading
                    window._cleanup_workers();

                    const timestamp = Date.now();
                    const mod = await import(`/pkg/site.js?t=${timestamp}`);
                    await mod.default();
                    mod.run();
                };
                
                // Initial load with cache busting
                (async () => {
                    const timestamp = Date.now();
                    const mod = await import(`/pkg/site.js?t=${timestamp}`);
                    await mod.default();
                    mod.run();
                })();
                "#))
            } @else {
                "import init, { run } from '/pkg/site.js'; init().then(run);"
            }
        }
    }
}
