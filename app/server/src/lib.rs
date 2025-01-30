mod web;
mod storage;
mod error;
use web::*;
use std::{env, sync::Arc};
use axum::{middleware, response::{IntoResponse, Response}, serve::serve, Json, Router};
use error::Error;
use storage::{controller::{StorageController, StorageControllerInner}, model::ServerConfig};
use serde_json::json;                   
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::RwLock};
use config::SERVER_CONFIG;

async fn run() {
    dotenvy::dotenv().expect("Failed to get env variables from .env");

    // Create file save directory if it doesn't exist already
    tokio::fs::create_dir_all(&SERVER_CONFIG.data_dir).await
        .expect("Failed to create save directory");

    let db_pool = PgPool::connect(
        env::var("DATABASE_URL").expect("Could not find DATABASE_URL in env").as_str()
    ).await.expect("Failed to connect to database");

    sqlx::migrate!("./migrations").run(&db_pool).await.expect("Failed to run migrations.");

    let mc = Arc::new(StorageControllerInner::new(db_pool).await);

    let routes = Router::new()
        .nest("", routes::routes(mc).await)
        .layer(middleware::map_response(main_response_mapper));

    let listener = TcpListener::bind("127.0.0.1:9101").await.unwrap();

    println!("Initialization complete..");
    serve(listener, routes.into_make_service()).await.expect("Failed to start listening");
}

async fn main_response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<Error>();

    let sc_and_ce = error
        .map(|e| e.to_status_and_client_error());

    let error_response = sc_and_ce
        .as_ref()
        .map(
            |(status_code, client_err)| {
            let err_json = serde_json::to_value(client_err);
            let body = err_json.unwrap_or(json!("Failed to get error information."));
            println!("{:?}", client_err);

            (*status_code, Json(body)).into_response()
            }
        );

    error_response.unwrap_or(res)
}                                                                     