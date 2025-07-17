pub mod web;
pub mod controllers;
pub mod error;
pub mod validation;
pub mod db_utils;
use tracing::{trace, warn};
use web::*;
use std::{env, sync::Arc};
use axum::{middleware, response::{IntoResponse, Response}, serve::serve, Json, Router};
use error::ServerError;
use controllers::{files::{FileController, FileControllerInner}};
use serde_json::json;                   
use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::{net::TcpListener, sync::RwLock};
use crate::config::SETTINGS;

use error::ServerResult;

pub async fn init() -> ServerResult<()> {
    // Create all required dirs
    trace!("Creating data directory: {:?}", &SETTINGS.directories.data_dir);
    tokio::fs::create_dir_all(&SETTINGS.directories.data_dir).await?;
    trace!("Creating files directory: {:?}", &SETTINGS.directories.files_dir);
    tokio::fs::create_dir_all(&SETTINGS.directories.files_dir).await?;
    trace!("Directories created successfully");

    // Try creating the ocloud database
    let pool_url = SETTINGS.database.connection_string_without_db();
    trace!("Trying to connect to database via url {pool_url}...");
    let pool = 
    PgPool::connect(&pool_url).await?;

    let res = sqlx::query(
        &format!(
            "CREATE DATABASE {}", 
            SETTINGS.database.database_name
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

pub async fn create_server(db_pool: PgPool) -> Router {
    trace!("Creating file controller...");
    let mc = Arc::new(FileControllerInner::new(db_pool).await);
    trace!("File controller created.");

    trace!("Setting up routes...");
    let routes = Router::new()
        .nest("", routes::routes(mc).await)
        .layer(middleware::map_response(main_response_mapper))
        .layer(middleware::from_fn(web::middleware::trace_request));
    trace!("Routes set up.");

    routes
}

pub async fn run(host: &str, port: u16, pg_connect_opts: PgConnectOptions) -> ServerResult<()> {
    init().await?;
    trace!("Created required directories and databases.");

    let db_pool = PgPool::connect_lazy_with(pg_connect_opts);

    sqlx::migrate!("./migrations").run(&db_pool).await
        .expect("Failed to run migrations.");
    trace!("Ran database migrations.");

    let routes = create_server(db_pool).await;

    trace!("Binding to {host}:{port}...");
    let listener = TcpListener::bind(
        format!("{host}:{port}")
    ).await?;
    trace!("Listener bound successfully.");

    println!("Listening on {host}:{port}");
    serve(listener, routes.into_make_service()).await?;
    Ok(())
}

async fn main_response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<ServerError>();

    let error_response = error.map(|err| {
        let (status_code, client_err) = err.to_status_and_client_error();
        warn!("Request error: {}", err);
        (status_code, Json(client_err)).into_response()
    });

    error_response.unwrap_or(res)
}

pub async fn file_controller() -> ServerResult<FileController> {
    init().await?;
    let db_url = SETTINGS.database.connection_string();

    let db_pool = PgPool::connect(&db_url).await?;

    sqlx::migrate!("./migrations").run(&db_pool).await
        .expect("Failed to run migrations.");

    Ok(Arc::new(FileControllerInner::new(db_pool).await))
}