use axum::http::{self, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::server::{controllers::{files::FileController, websocket::WebSocketController}, ServerState};
use super::handlers::{files, auth, ws};

pub async fn routes(controller: FileController, ws_controller: Option<WebSocketController>, server_state: ServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ])
        .allow_credentials(true);

    let mut router = Router::new()
        .nest("/", files::routes(controller.clone()))
        .nest("/", auth::routes(server_state.auth_controller.clone()))
        .route("/ping", get(ping))
        .route("/health", get(health_check));
    
    // Add WebSocket routes if WebSocket controller is provided
    if let Some(ws_ctrl) = ws_controller {
        router = router.nest("/", ws::routes(ws_ctrl, controller.clone(), server_state));
    }
    
    router
        .layer(cors)
        // Rate limiting temporarily disabled
        // .layer(middleware::rate_limiting_layer())
}

async fn ping() -> &'static str {
    "pong...?"
}

async fn health_check() -> &'static str {
    ""
}           
