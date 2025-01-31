use axum::{routing::MethodRouter, Router};

pub fn routes() -> Router {
    Router::new()
        .route(
            "/auth", 
            MethodRouter::new()
        )
}