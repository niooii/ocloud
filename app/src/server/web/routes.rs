use axum::http::{self, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::server::controllers::files::FileController;
use super::handlers::files;
use super::handlers::auth;
use super::middleware;

pub async fn routes(controller: FileController) -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ])
        .allow_credentials(true);

    Router::new()
        .nest("/", files::routes(controller))
        .nest("/", auth::routes())
        .route("/ping", get(ping))
        .route("/health", get(health_check))
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
