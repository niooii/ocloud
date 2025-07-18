pub mod web;
pub mod controllers;
pub mod models;
pub mod error;
pub mod validation;
pub mod db_utils;
use tracing::{trace, warn};
use web::*;
use std::sync::Arc;
use axum::{middleware, response::{IntoResponse, Response}, serve::serve, Json, Router};
use error::ServerError;
use controllers::{files::{FileController, FileControllerInner}, websocket::{WebSocketController, WebSocketControllerInner}, auth::AuthController};

#[derive(Clone)]
pub struct ServerState {
    pub file_controller: FileController,
    pub ws_controller: WebSocketController,
    pub auth_controller: AuthController,
}
                   
use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::net::TcpListener;
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
    trace!("Creating WebSocket controller...");
    let ws_controller = Arc::new(WebSocketControllerInner::new());
    trace!("WebSocket controller created.");

    trace!("Creating auth controller...");
    let auth_controller = AuthController::new(db_pool.clone());
    trace!("Auth controller created.");

    trace!("Creating file controller with WebSocket support...");
    let file_controller = Arc::new(FileControllerInner::new(db_pool, Arc::clone(&ws_controller)).await);
    trace!("File controller created.");

    trace!("Creating server state...");
    let server_state = ServerState {
        file_controller: file_controller.clone(),
        ws_controller: ws_controller.clone(),
        auth_controller: auth_controller.clone(),
    };
    trace!("Server state created.");

    trace!("Setting up routes...");
    let routes = Router::new()
        .nest("", routes::routes(file_controller, Some(ws_controller), server_state).await)
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

    Ok(Arc::new(FileControllerInner::new_no_ws(db_pool).await))
}

/// Only useful for nuking lol
pub async fn file_controller_no_migrate() -> ServerResult<FileController> {
    init().await?;
    let db_url = SETTINGS.database.connection_string();

    let db_pool = PgPool::connect(&db_url).await?;

    Ok(Arc::new(FileControllerInner::new_no_ws(db_pool).await))
}