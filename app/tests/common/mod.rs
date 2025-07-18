use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool, postgres::PgConnectOptions};
use uuid::Uuid;
use axum::serve::serve;
use tokio::net::TcpListener;
use ocloud::config::SETTINGS;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter_level))
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
    } else {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::ERROR)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
    }
});

async fn configure_test_database(db_name: &str) -> PgPool {
    let connection_string_without_db = SETTINGS.database.connection_string_without_db();
    let mut connection = PgConnection::connect(&connection_string_without_db)
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("Failed to create database");

    let test_connection_string = format!(
        "postgres://{}:{}@{}:{}/{}",
        SETTINGS.database.username,
        SETTINGS.database.password,
        SETTINGS.database.host,
        SETTINGS.database.port,
        db_name
    );

    let connection_pool = PgPool::connect(&test_connection_string)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

async fn cleanup_database(db_name: String) {
    let connection_string_without_db = SETTINGS.database.connection_string_without_db();
    let mut connection = PgConnection::connect(&connection_string_without_db)
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(
            format!(
                r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}';"#,
                db_name
            )
            .as_str(),
        )
        .await
        .ok();

    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, db_name).as_str())
        .await
        .ok();
}

async fn run_test_server_with_listener(listener: TcpListener, pg_connect_opts: PgConnectOptions) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_pool = PgPool::connect_lazy_with(pg_connect_opts);

    // Run migrations on the test database
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    // Use the shared server creation logic
    let routes = ocloud::server::create_server(db_pool).await;

    // Serve with existing listener
    serve(listener, routes.into_make_service()).await?;
    Ok(())
}