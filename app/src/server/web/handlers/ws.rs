use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use tracing::info;

use crate::server::{controllers::{websocket::WebSocketController, files::FileController}, ServerState};

#[derive(Clone)]
pub struct WebSocketState {
    pub ws_controller: WebSocketController,
    pub file_controller: FileController,
    pub server_state: ServerState,
}

pub fn routes(ws_controller: WebSocketController, file_controller: FileController, server_state: ServerState) -> Router {
    let state = WebSocketState {
        ws_controller,
        file_controller,
        server_state,
    };
    
    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebSocketState>,
) -> Response {
    info!("WebSocket upgrade requested");
    
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = state.ws_controller.add_connection(socket, &state.server_state).await {
            tracing::error!("Failed to handle WebSocket connection: {}", e);
        }
    })
}