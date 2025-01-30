use axum::routing::get;
use axum::Router;

use crate::storage::controller::StorageController;
use crate::handlers::base;

pub async fn routes(controller: StorageController) -> Router {
    Router::new()
        .nest("/", base::routes(controller))
        .route("/ping", get(ping))
}

async fn ping() -> &'static str {
    "pong...?"
}           
