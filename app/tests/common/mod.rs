use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool, postgres::PgConnectOptions};
use uuid::Uuid;
use axum::serve::serve;
use tokio::net::TcpListener;

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

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
}

impl TestApp {
    pub async fn spawn() -> TestApp {
        Lazy::force(&TRACING);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{}", port);

        let db_name = format!("test_{}", Uuid::new_v4().simple());
        let db_pool = configure_test_database(&db_name).await;

        let pg_opts = sqlx::postgres::PgConnectOptions::new()
            .host("localhost")
            .username("user")
            .password("pass")
            .database(&db_name)
            .port(9432);

        let server = run_test_server_with_listener(listener, pg_opts.clone());

        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("Server failed to start: {:?}", e);
            }
        });

        // Give the server a moment to start up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        TestApp {
            address,
            db_pool,
            db_name,
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        tokio::spawn(cleanup_database(self.db_name.clone()));
    }
}

async fn configure_test_database(db_name: &str) -> PgPool {
    let mut connection = PgConnection::connect("postgres://user:pass@localhost:9432/postgres")
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect(&format!(
        "postgres://user:pass@localhost:9432/{}",
        db_name
    ))
    .await
    .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

async fn cleanup_database(db_name: String) {
    let mut connection = PgConnection::connect("postgres://user:pass@localhost:9432/postgres")
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