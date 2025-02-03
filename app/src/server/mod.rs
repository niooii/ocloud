mod web;
mod controllers;
pub mod error;
use tracing::{trace, warn};
use web::*;
use std::{env, sync::Arc};
use axum::{middleware, response::{IntoResponse, Response}, serve::serve, Json, Router};
use error::ServerError;
use controllers::{files::{FileController, FileControllerInner}, model::ServerConfig};
use serde_json::json;                   
use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::{net::TcpListener, sync::RwLock};
use crate::config::SERVER_CONFIG;

use error::ServerResult;

pub async fn init() -> ServerResult<()> {
    // Create all required dirs
    tokio::fs::create_dir_all(&SERVER_CONFIG.data_dir).await?;
    tokio::fs::create_dir_all(&SERVER_CONFIG.files_dir).await?;

    // Try creating the ocloud database
    let pool_url = SERVER_CONFIG.postgres.to_url_default_db();
    trace!("Trying to connect to database at {pool_url}...");
    let pool = 
    PgPool::connect(&pool_url).await?;

    let res = sqlx::query(
        &format!(
            "CREATE DATABASE {}", 
            SERVER_CONFIG.postgres.database
        )
    ).execute(&pool).await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.to_string().contains("already exists") {
                Ok(())
            } else {
                Err(e.into())
            }
        }
    }
}

pub async fn run(host: &str, port: u16, pg_connect_opts: PgConnectOptions) -> ServerResult<()> {
    init().await?;
    trace!("Created required directories and databases.");

    let db_pool = PgPool::connect_with(pg_connect_opts)
    .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await
    .expect("Failed to run migrations.");
    trace!("Ran database migrations.");

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

            warn!("Error: {client_err:?}");

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