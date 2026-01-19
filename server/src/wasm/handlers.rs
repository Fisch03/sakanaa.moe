use super::runtime::execute_wasm_render;
use super::state::{WasmState, WasmUpdate};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    http::header::{CONTENT_TYPE, HeaderValue},
    response::{Html, IntoResponse, Response},
};
use log::*;
use maud::html;
use std::sync::Arc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WasmState>>,
) -> impl IntoResponse {
    #[cfg(debug_assertions)]
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<WasmState>) {
    // Send initial state
    {
        let html_hash = *state.html_hash.read().unwrap();
        let css_hash = *state.css_hash.read().unwrap();
        let css = state.css.read().unwrap().clone();

        let initial_update = WasmUpdate {
            html_hash,
            css_hash,
            css,
        };

        if let Ok(msg) = serde_json::to_string(&initial_update)
            && socket.send(Message::Text(msg.into())).await.is_err()
        {
            return;
        }
    }

    let mut rx = state.reload_tx.subscribe();

    while let Ok(update) = rx.recv().await
        && let Ok(msg) = serde_json::to_string(&update)
    {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }
}

pub async fn style_handler(State(state): State<Arc<WasmState>>) -> Response {
    let css = state.css.read().unwrap().clone();
    ([(CONTENT_TYPE, HeaderValue::from_static("text/css"))], css).into_response()
}

pub async fn render_handler(State(state): State<Arc<WasmState>>) -> Response {
    let module_guard = state.module.read().unwrap();
    let Some(module) = module_guard.as_ref() else {
        return Html(html! { h1 { "wasm module not loaded (yet)." } }).into_response();
    };

    match execute_wasm_render(&state.engine, module) {
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

