use axum::http::{self, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::server::controllers::files::FileController;
use super::handlers::files;

use super::handlers::auth;

pub async fn routes(controller: FileController) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ]);

    Router::new()
        .nest("/", files::routes(controller))
        .nest("/", auth::routes())
        .route("/ping", get(ping))
        .layer(cors)
}

async fn ping() -> &'static str {
    "pong...?"
}           
