mod web;
mod controllers;
pub mod error;
use web::*;
use std::{env, sync::Arc};
use axum::{middleware, response::{IntoResponse, Response}, serve::serve, Json, Router};
use error::ServerError;
use controllers::{files::{FileController, FileControllerInner}, model::ServerConfig};
use serde_json::json;                   
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::RwLock};
use crate::config::SERVER_CONFIG;

use error::ServerResult;

pub async fn init() -> ServerResult<()> {
    // Create all required dirs
    tokio::fs::create_dir_all(&SERVER_CONFIG.data_dir).await?;
    tokio::fs::create_dir_all(&SERVER_CONFIG.files_dir).await?;

    Ok(())
}

pub async fn run(host: &str, port: u16) -> ServerResult<()> {
    init().await?;

    let db_url = SERVER_CONFIG.postgres.to_url();

    let db_pool = PgPool::connect(&db_url).await?;

    sqlx::migrate!("./migrations").run(&db_pool).await
        .expect("Failed to run migrations.");

    let mc = Arc::new(FileControllerInner::new(db_pool).await);

    let routes = Router::new()
        .nest("", routes::routes(mc).await)
        .layer(middleware::map_response(main_response_mapper));

    let listener = TcpListener::bind(
        format!("{host}:{port}")
    ).await?;

    println!("Listening on {host}:{port}");
    serve(listener, routes.into_make_service()).await?;
    Ok(())
}

async fn main_response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<ServerError>();

    let sc_and_ce = error
        .map(error::ServerError::to_status_and_client_error);

    let error_response = sc_and_ce
        .as_ref()
        .map(
            |(status_code, client_err)| {
            let err_json = serde_json::to_value(client_err);
            let body = err_json.unwrap_or(json!("Failed to get error information."));
            println!("{client_err:?}");

            (*status_code, Json(body)).into_response()
            }
        );

    error_response.unwrap_or(res)
}

pub async fn file_controller() -> ServerResult<FileController> {
    init().await?;
    let db_url = SERVER_CONFIG.postgres.to_url();

    let db_pool = PgPool::connect(&db_url).await?;

    sqlx::migrate!("./migrations").run(&db_pool).await
        .expect("Failed to run migrations.");

    Ok(Arc::new(FileControllerInner::new(db_pool).await))
}