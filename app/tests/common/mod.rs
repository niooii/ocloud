use ocloud::api::ApiClient;
use ocloud::config::SETTINGS;
use ocloud::server::models::auth::{LoginRequest, RegisterRequest};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

// initialize the test environment
static INIT: Lazy<()> = Lazy::new(|| {
    // Load .env file for tests
    dotenv::dotenv().ok();

    // Ensure we're using the test environment
    std::env::set_var("APP_ENVIRONMENT", "testing");

    // Debug: print what database name we're using
    println!(
        "TEST CONFIG: Using database: {}",
        SETTINGS.database.database_name
    );
    println!(
        "TEST CONFIG: Environment: {:?}",
        SETTINGS.application.environment
    );

    // Setup tracing
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter_level)),
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

    // Create fresh database
    connection
        .execute(format!(r#"CREATE DATABASE "{db_name}";"#).as_str())
        .await
        .expect("Failed to create test database");

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

async fn cleanup_database(db_name: &str) {
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
        .execute(format!(r#"DROP DATABASE IF EXISTS "{db_name}";"#).as_str())
        .await
        .ok();
}

/// Create a new database pool for testing with a unique database
/// Returns the database pool which can be used to create ApiClient instances
pub async fn create_test_db() -> PgPool {
    Lazy::force(&INIT);

    // Create unique database name using config prefix + UUID
    let uuid = Uuid::new_v4();
    let db_name = format!("{}_{}", SETTINGS.database.database_name, uuid.simple());

    configure_test_database(&db_name).await
}

/// Cleanup the test database
pub async fn cleanup_test_database(db_pool: PgPool) {
    // Get the database name from the pool's connection string
    let db_name = extract_database_name_from_pool(&db_pool);

    // Close the pool first
    db_pool.close().await;

    cleanup_database(&db_name).await;
}

/// Extract database name from a PgPool
fn extract_database_name_from_pool(pool: &PgPool) -> String {
    // This is a bit hacky but works for our test setup
    // The database name is in the format: "{prefix}_{uuid}"
    // We can get it from the pool's connection options
    let connect_options = pool.connect_options();
    connect_options
        .get_database()
        .unwrap_or("unknown")
        .to_string()
}

/// Create a random authenticated user and set their session on the client
/// Generates UUID-based username, email, and password
/// Returns the user info after setting the session on the client
pub async fn authenticate_random(client: &mut ApiClient) -> serde_json::Value {
    let uuid = Uuid::new_v4();
    let username = format!("user_{}", uuid.simple());
    let email = format!("{}@gmail.com", uuid.simple());
    let password = format!("pass_{}", uuid.simple());

    let (session_id, user) = register_and_login(client, &username, &email, &password).await;
    client.set_session(session_id);
    user
}

/// Register a new user and return their session ID and user info
async fn register_and_login(
    client: &ApiClient,
    username: &str,
    email: &str,
    password: &str,
) -> (String, serde_json::Value) {
    // Register user
    let register_request = RegisterRequest {
        username: username.to_string(),
        email: email.to_string(),
        password: password.to_string(),
    };

    client
        .register(register_request)
        .await
        .expect("Failed to register user");

    // Login and get session
    let login_request = LoginRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    let login_response = client.login(login_request).await.expect("Failed to login");

    let session_id = login_response["session_id"].as_str().unwrap().to_string();
    let user = login_response["user"].clone();
    (session_id, user)
}
