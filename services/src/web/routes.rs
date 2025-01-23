use axum::{body::Body, extract::{DefaultBodyLimit, Multipart, Path, Query, State}, http::{header, HeaderValue}, response::Response, routing::{get, post}, Json, Router};
use sha2::{Digest};

use crate::{storage::controller::StorageController, CONFIG};
use crate::handlers::base;

pub async fn routes(storage: StorageController) -> Router{
    Router::new()
        .route(
            "/media/*path", 
            get(base::get_media)
            .delete(base::delete_media)
            .post(base::upload_media)
            .layer(
                if let Some(s) = CONFIG.read().await.max_filesize {
                    DefaultBodyLimit::max(s)
                } else {
                    DefaultBodyLimit::disable()
                }
            )
        )
        .route("/ping", get(ping))
        .with_state(storage)
}

async fn ping() -> &'static str {
    "pong...?"
}           
