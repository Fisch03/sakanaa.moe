use wasm_bindgen::prelude::*;
use wasm_bridge::worker::WorkerContextExt;
use web_sys::{DedicatedWorkerGlobalScope, Request, RequestInit, RequestMode, Response};

use super::messages::{MainMessage, WorkerMessage};

#[wasm_bindgen]
pub async fn album_worker_entry(scope: DedicatedWorkerGlobalScope) {
    wasm_bridge::init();

    let mut pending_requests = Vec::new();
    let mut queue: Vec<f64> = Vec::new();
    let min_batch_size = 50;
    let min_threshold = 25;

    // Channel for communication
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Setup message handler
    let tx_clone = tx.clone();
    scope.set_onmessage_typed(move |msg: WorkerMessage| match msg {
        WorkerMessage::Fetch { id } => {
            let _ = tx_clone.send(id);
        }
    });

    log::info!("rust worker initialized");

    let _ = scope.post_message_typed(&MainMessage::Ready);

    loop {
        // Check for new requests
        while let Ok(id) = rx.try_recv() {
            pending_requests.push(id);
        }

        // Process queue
        while !pending_requests.is_empty() && !queue.is_empty() {
            let id = pending_requests.remove(0);
            let cover_id = queue.remove(0);

            let _ = scope.post_message_typed(&MainMessage::Cover { id, cover_id });
        }

        // Fetch if needed
        if queue.len() < min_threshold {
            let origin = scope.location().origin();
            let batch_size = std::cmp::max(min_batch_size, pending_requests.len() + min_threshold);
            let url = format!("{}/api/v1/music/album/random/{}", origin, batch_size);

            let opts = RequestInit::new();
            opts.set_method("GET");
            opts.set_mode(RequestMode::Cors);

            let request = Request::new_with_str_and_init(&url, &opts).unwrap();

            // Perform fetch
            match wasm_bindgen_futures::JsFuture::from(scope.fetch_with_request(&request)).await {
                Ok(resp_value) => {
                    let resp: Response = resp_value.dyn_into().unwrap();
                    if resp.ok()
                        && let Ok(json) =
                            wasm_bindgen_futures::JsFuture::from(resp.json().unwrap()).await
                        && let Ok(albums) = json.dyn_into::<js_sys::Array>()
                    {
                        for album in albums.iter() {
                            let album_obj: js_sys::Object = album.dyn_into().unwrap();
                            let cover_id_val =
                                js_sys::Reflect::get(&album_obj, &"cover_id".into()).unwrap();

                            if let Some(cover_id) = cover_id_val.as_f64() {
                                queue.push(cover_id);
                            }
                        }
                    }
                }
                Err(e) => log::error!("Worker fetch error: {:?}", e),
            }

            // If we still have pending requests but empty queue, try again soon
            if queue.is_empty() && !pending_requests.is_empty() {
                // simple delay
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(
                    &mut |resolve, _| {
                        let _ = js_sys::global()
                            .unchecked_into::<web_sys::DedicatedWorkerGlobalScope>()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1000);
                    },
                ))
                .await;
            }
        }

        // Yield to event loop
        if pending_requests.is_empty() && queue.len() >= min_threshold {
            // Wait for next message
            if let Some(id) = rx.recv().await {
                pending_requests.push(id);
            } else {
                break; // Channel closed
            }
        } else {
            // Just yield briefly
            let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL))
                .await;
        }
    }
}
