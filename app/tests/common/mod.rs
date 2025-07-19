use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool, postgres::PgConnectOptions};
use uuid::Uuid;
use axum::serve::serve;
use tokio::net::TcpListener;
use ocloud::config::SETTINGS;
use ocloud::server::ServerState;
use reqwest::multipart::Form;
use std::sync::Arc;
use tokio::runtime::Runtime;

static INIT: Lazy<()> = Lazy::new(|| {
    // Load .env file for tests
    dotenv::dotenv().ok();
    
    // Setup tracing
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
        .execute(format!(r#"CREATE DATABASE "{db_name}";"#).as_str())
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
                r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}';"#
            )
            .as_str(),
        )
        .await
        .ok();

    connection
        .execute(format!(r#"DROP DATABASE "{db_name}";"#).as_str())
        .await
        .ok();
}

async fn run_test_server_with_listener(listener: TcpListener, pg_connect_opts: PgConnectOptions) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_pool = PgPool::connect_lazy_with(pg_connect_opts);

    // Run migrations on the test database
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    // Use the shared server creation logic
    let (routes, _server_state) = ocloud::server::create_server(db_pool).await;

    // Serve with existing listener
    serve(listener, routes.into_make_service()).await?;
    Ok(())
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
    pub state: ServerState,
    runtime: Runtime,
}

// Static test app instance shared across all tests
pub static TEST_APP: Lazy<Arc<TestApp>> = Lazy::new(|| {
    Arc::new(futures::executor::block_on(TestApp::init()))
});

impl Drop for TestApp {
    fn drop(&mut self) {
        // Clean up the test database when the TestApp is dropped
        let db_name = self.db_name.clone();
        self.runtime.block_on(async {
            cleanup_database(db_name).await;
        });
    }
}

impl TestApp {
    /// Get the shared test app instance
    pub async fn get() -> Arc<TestApp> {
        Lazy::force(&INIT);
        TEST_APP.clone()
    }

    /// Initialize the test app (called once by the static)
    async fn init() -> TestApp {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");

        // Use database name from test.yaml config instead of random UUID
        let db_name = SETTINGS.database.database_name.clone();
        let db_pool = configure_test_database(&db_name).await;

        let db_opts = PgConnectOptions::new()
            .host(&SETTINGS.database.host)
            .username(&SETTINGS.database.username)
            .password(&SETTINGS.database.password)
            .port(SETTINGS.database.port)
            .database(&db_name);

        // Create server state for test access
        let (_routes, server_state) = ocloud::server::create_server(db_pool.clone()).await;

        let _server_handle = tokio::spawn(run_test_server_with_listener(listener, db_opts));

        TestApp {
            address,
            db_pool,
            db_name,
            state: server_state,
            runtime: Runtime::new().expect("Failed to create runtime for TestApp"),
        }
    }

    /// Legacy method for backward compatibility
    pub async fn spawn() -> Arc<TestApp> {
        Self::get().await
    }

    /// Create a random authenticated user and return their client
    /// Generates UUID-based username, email, and password
    pub async fn create_random_authenticated_user(&self) -> reqwest::Client {
        let uuid = Uuid::new_v4();
        let username = format!("user_{}", uuid.simple());
        let email = format!("{}@gmail.com", uuid.simple());
        let password = format!("pass_{}", uuid.simple());
        
        self.create_authenticated_user(&username, &email, &password).await
    }

    /// Register a new user and return their session ID
    pub async fn register_and_login(&self, username: &str, email: &str, password: &str) -> String {
        let client = reqwest::Client::new();

        // Register user
        let user_data = serde_json::json!({
            "username": username,
            "email": email,
            "password": password
        });

        client
            .post(format!("{}/auth/register", &self.address))
            .json(&user_data)
            .send()
            .await
            .expect("Failed to register user");

        // Login and get session
        let login_data = serde_json::json!({
            "username": username,
            "password": password
        });

        let login_response = client
            .post(format!("{}/auth/login", &self.address))
            .json(&login_data)
            .send()
            .await
            .expect("Failed to login");

        let login_body: serde_json::Value = login_response.json().await.expect("Failed to parse login response");
        login_body["session_id"].as_str().unwrap().to_string()
    }

    /// Create an authenticated reqwest client with the given session ID
    pub fn authenticated_client(&self, session_id: &str) -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {session_id}"))
                        .expect("Invalid session ID")
                );
                headers
            })
            .build()
            .expect("Failed to build authenticated client")
    }

    /// Create an anonymous reqwest client (no authentication headers)
    pub fn anonymous_client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    /// Convenience method to register a user and get an authenticated client
    pub async fn create_authenticated_user(&self, username: &str, email: &str, password: &str) -> reqwest::Client {
        let session_id = self.register_and_login(username, email, password).await;
        self.authenticated_client(&session_id)
    }

    /// Helper function to create a test multipart file
    pub fn test_multipart_file(name: &str) -> Form {
        Form::new()
            .part(name.to_string(), reqwest::multipart::Part::text("test file content"))
    }

}