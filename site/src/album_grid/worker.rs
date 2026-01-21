use std::f64::consts::PI;

use futures::future::FutureExt;
use futures::stream::{FuturesUnordered, StreamExt};
use wasm_bindgen::prelude::*;
use wasm_bridge::OffscreenCanvasElement;
use wasm_bridge::worker::WorkerContextExt;
use web_sys::{Blob, DedicatedWorkerGlobalScope, Request, RequestInit, RequestMode, Response, Url};

use super::messages::{MainMessage, WorkerMessage};

fn draw_cd_template(
    ctx: &web_sys::OffscreenCanvasRenderingContext2d,
    size: f64,
    primary: &str,
    secondary: &str,
) {
    ctx.set_image_smoothing_enabled(false);
    ctx.set_stroke_style_str(primary);
    ctx.set_fill_style_str("transparent");
    ctx.set_line_width(1.0);

    let center = size / 2.0;
    let r_outer = center;
    let r_hole = r_outer * 0.15;

    // Clear
    ctx.clear_rect(0.0, 0.0, size, size);

    // outer rim
    ctx.begin_path();
    ctx.arc(center, center, r_outer - 0.5, 0.0, PI * 2.0)
        .unwrap();
    ctx.stroke();

    // hole
    ctx.begin_path();
    ctx.arc(center, center, r_hole, 0.0, PI * 2.0).unwrap();
    ctx.fill();

    ctx.begin_path();
    ctx.arc(center, center, r_hole, 0.0, PI * 2.0).unwrap();
    ctx.stroke();
}

#[wasm_bindgen]
pub async fn album_worker_entry(scope: DedicatedWorkerGlobalScope) {
    wasm_bridge::init_worker();

    let mut pending_requests: Vec<String> = Vec::new();
    let mut raw_queue: Vec<String> = Vec::new(); // Store cover_ids as Strings
    let mut processed_queue: Vec<String> = Vec::new(); // Store Object URLs
    let min_batch_size = 50;
    let min_threshold = 25;
    let max_concurrent_fetches = 8; // Adjust based on browser limits

    // CD Template Canvas (Persistent)
    let template_canvas = OffscreenCanvasElement::new(1, 1);

    // Processing Canvas (Persistent)
    let process_canvas = OffscreenCanvasElement::new(1, 1);

    // Current CD config
    let mut current_size = 0.0;
    let mut current_primary = String::from("#000");
    let mut current_secondary = String::from("#fff");

    // Channel for communication
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WorkerMessage>();

    // Setup message handler
    let tx_clone = tx.clone();
    scope.set_onmessage_typed(move |msg: WorkerMessage| {
        let _ = tx_clone.send(msg);
    });

    log::info!("rust worker initialized");

    let _ = scope.post_message_typed(&MainMessage::Ready);

    // Pending fetch tasks
    let mut tasks = FuturesUnordered::new();

    loop {
        // 1. Refill tasks
        // If we have raw items and haven't hit concurrency limit, spawn tasks
        // Only spawn if we have a valid size set, otherwise we can't fetch anyway
        // (Wait, we can fetch, but we can't draw. We could fetch -> Blob, but for simplicity let's wait)
        while tasks.len() < max_concurrent_fetches && !raw_queue.is_empty() && current_size > 0.0 {
            let cover_id = raw_queue.remove(0);
            let origin = scope.location().origin();
            let url = format!("{}/api/v1/music/cover/{}", origin, cover_id);
            let scope_clone = scope.clone();

            // Spawn fetch task
            // Returns: Option<(id, ImageBitmap)>
            tasks.push(async move {
                let opts = RequestInit::new();
                opts.set_method("GET");
                opts.set_mode(RequestMode::Cors);

                let request = Request::new_with_str_and_init(&url, &opts).ok()?;
                let resp_val =
                    wasm_bindgen_futures::JsFuture::from(scope_clone.fetch_with_request(&request))
                        .await
                        .ok()?;
                let resp: Response = resp_val.dyn_into().ok()?;

                if !resp.ok() {
                    return None;
                }

                let blob_val = wasm_bindgen_futures::JsFuture::from(resp.blob().ok()?)
                    .await
                    .ok()?;
                let blob: Blob = blob_val.dyn_into().ok()?;

                let bitmap_promise = scope_clone.create_image_bitmap_with_blob(&blob).ok()?;
                let bitmap_val = wasm_bindgen_futures::JsFuture::from(bitmap_promise)
                    .await
                    .ok()?;
                let bitmap: web_sys::ImageBitmap = bitmap_val.dyn_into().ok()?;

                Some((cover_id, bitmap))
            });
        }

        // 2. Wait for either:
        //    A) A message from main thread
        //    B) A task to complete (if any)

        let msg_future = rx.recv().boxed_local();
        // If no tasks, we just wait for message forever
        // If tasks, we wait for next task
        let task_future = if tasks.is_empty() {
            futures::future::pending().boxed_local()
        } else {
            tasks.next().boxed_local()
        };

        match futures::future::select(msg_future, task_future).await {
            futures::future::Either::Left((msg_opt, _)) => {
                // Handle message
                if let Some(msg) = msg_opt {
                    match msg {
                        WorkerMessage::Fetch { id } => {
                            pending_requests.push(id);
                        }
                        WorkerMessage::SetSize {
                            size,
                            primary,
                            secondary,
                        } => {
                            let render_size = size / 2.0;
                            let canvas_size = render_size.ceil() as u32;
                            current_size = render_size;
                            current_primary = primary;
                            current_secondary = secondary;

                            template_canvas.set_size(canvas_size, canvas_size);
                            process_canvas.set_size(canvas_size, canvas_size);

                            if let Some(ctx) = template_canvas.get_context_2d() {
                                draw_cd_template(
                                    &ctx,
                                    render_size,
                                    &current_primary,
                                    &current_secondary,
                                );
                            }
                            processed_queue.clear();
                            // Note: We don't clear running tasks. They will complete and be processed with NEW size?
                            // No, they return Bitmap. We draw using `current_size` in the main loop.
                            // So as long as we update `current_size` here, any task that finishes AFTER this
                            // will be drawn with the NEW size. Perfect.
                        }
                    }
                } else {
                    break; // Channel closed
                }
            }
            futures::future::Either::Right((task_res_opt, _)) => {
                // Task completed
                if let Some(res) = task_res_opt {
                    // res is Option<(id, bitmap)>
                    if let Some((_cover_id, bitmap)) = res {
                        // Process synchronously
                        if let Some(ctx) = process_canvas.get_context_2d() {
                            let s = current_size;
                            // Draw
                            let _ = ctx.draw_image_with_image_bitmap_and_dw_and_dh(
                                &bitmap, 0.0, 0.0, s, s,
                            );

                            // Punch a hole in the center
                            ctx.set_global_composite_operation("destination-out")
                                .unwrap();
                            ctx.begin_path();
                            let center = s / 2.0;
                            let r_hole = center * 0.15;
                            ctx.arc(center, center, r_hole, 0.0, PI * 2.0).unwrap();
                            ctx.fill();
                            ctx.set_global_composite_operation("source-over").unwrap();

                            let _ = ctx.draw_image_with_offscreen_canvas_and_dw_and_dh(
                                template_canvas.inner(),
                                0.0,
                                0.0,
                                s,
                                s,
                            );

                            // Convert
                            if let Ok(blob_promise) = process_canvas.inner().convert_to_blob() {
                                if let Ok(blob_res) =
                                    wasm_bindgen_futures::JsFuture::from(blob_promise).await
                                {
                                    if let Ok(final_blob) = blob_res.dyn_into::<Blob>() {
                                        if let Ok(object_url) =
                                            Url::create_object_url_with_blob(&final_blob)
                                        {
                                            if !pending_requests.is_empty() {
                                                let id = pending_requests.remove(0);
                                                let _ =
                                                    scope.post_message_typed(&MainMessage::Cover {
                                                        id,
                                                        data_url: object_url,
                                                    });
                                            } else {
                                                processed_queue.push(object_url);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Fulfill pending requests immediately (if we have processed items)
        while !pending_requests.is_empty() && !processed_queue.is_empty() {
            let id = pending_requests.remove(0);
            let data_url = processed_queue.remove(0);
            let _ = scope.post_message_typed(&MainMessage::Cover { id, data_url });
        }

        // 4. Check if we need to fetch more RAW IDs
        // We only do this if we are running low on work
        if raw_queue.len() < min_threshold {
            let origin = scope.location().origin();
            let batch_size = std::cmp::max(min_batch_size, pending_requests.len() + min_threshold);
            let url = format!("{}/api/v1/music/album/random/{}", origin, batch_size);
            let opts = RequestInit::new();
            opts.set_method("GET");
            opts.set_mode(RequestMode::Cors);

            // Spawn this as a background task via wasm_bindgen_futures, BUT we need the result in raw_queue.
            // Since we are in a loop, we can just spawn it as a 'task' if we wanted,
            // but these tasks expect (id, bitmap).
            // Let's just do it here sequentially (it's one request) or add a separate fetch logic?
            // Actually, waiting for this ONE request is fine, it's rare (once every 50 items).
            // But we should try to avoid blocking the entire pipeline.
            // For now, let's keep it simple: blocking await is okay for the ID refill.

            if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                if let Ok(resp_val) =
                    wasm_bindgen_futures::JsFuture::from(scope.fetch_with_request(&request)).await
                {
                    // ... handle response ...
                    // (Same logic as before)
                    if let Ok(resp) = resp_val.dyn_into::<Response>() {
                        if resp.ok() {
                            if let Ok(json) =
                                wasm_bindgen_futures::JsFuture::from(resp.json().unwrap()).await
                            {
                                if let Ok(albums) = json.dyn_into::<js_sys::Array>() {
                                    for album in albums.iter() {
                                        if let Ok(album_obj) = album.dyn_into::<js_sys::Object>() {
                                            if let Ok(cover_id_val) =
                                                js_sys::Reflect::get(&album_obj, &"cover_id".into())
                                            {
                                                if let Some(cid) = cover_id_val.as_string() {
                                                    raw_queue.push(cid);
                                                } else if let Some(cid) = cover_id_val.as_f64() {
                                                    raw_queue.push(cid.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If completely empty, pause a bit so we don't spin
            if raw_queue.is_empty() && tasks.is_empty() {
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(
                    &mut |resolve, _| {
                        let _ = js_sys::global()
                            .unchecked_into::<web_sys::DedicatedWorkerGlobalScope>()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 500);
                    },
                ))
                .await;
            }
        }
    }
}
