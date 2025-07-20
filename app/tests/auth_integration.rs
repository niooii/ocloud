mod common;

use axum::http::StatusCode;
use common::{authenticate_random, cleanup_test_database, create_test_db};
use ocloud::api::{ApiClient, ApiError};
use ocloud::server::models::auth::{LoginRequest, RegisterRequest};
use uuid::Uuid;

#[tokio::test]
async fn user_registration_works() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    let register_request = RegisterRequest {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "securepassword123".to_string(),
    };

    let response = client
        .register(register_request)
        .await
        .expect("Failed to execute request");

    assert!(response["user"]["id"].is_number());
    assert_eq!(response["user"]["username"], "testuser");
    assert_eq!(response["user"]["email"], "test@example.com");
    assert!(response["user"]["created_at"].is_string());
    assert_eq!(response["message"], "User registered successfully");

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn duplicate_registration_fails() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    let register_request = RegisterRequest {
        username: "testuser_dup".to_string(),
        email: "testdup@example.com".to_string(),
        password: "securepassword123".to_string(),
    };

    // First registration should succeed
    let _response = client
        .register(register_request.clone())
        .await
        .expect("Failed to execute first registration");

    // Second registration should fail
    let result = client.register(register_request).await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::BAD_REQUEST);
    } else {
        panic!("Expected HTTP error with status 400");
    }

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn user_login_works() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    // First register a user
    let register_request = RegisterRequest {
        username: "testuser_login".to_string(),
        email: "testlogin@example.com".to_string(),
        password: "securepassword123".to_string(),
    };

    client
        .register(register_request)
        .await
        .expect("Failed to register user");

    // Then login
    let login_request = LoginRequest {
        username: "testuser_login".to_string(),
        password: "securepassword123".to_string(),
    };

    let response = client
        .login(login_request)
        .await
        .expect("Failed to execute request");

    assert!(response["user"]["id"].is_number());
    assert_eq!(response["user"]["username"], "testuser_login");
    assert!(response["session_id"].is_string());
    assert!(response["expires_at"].is_string());
    assert_eq!(response["message"], "Login successful");

    // Validate session ID is a proper UUID
    let session_id = response["session_id"].as_str().unwrap();
    assert!(Uuid::parse_str(session_id).is_ok());

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn login_with_email_works() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    // Register a user
    let register_request = RegisterRequest {
        username: "testuser_email".to_string(),
        email: "testemail@example.com".to_string(),
        password: "securepassword123".to_string(),
    };

    client
        .register(register_request)
        .await
        .expect("Failed to register user");

    // Login with email instead of username
    let login_request = LoginRequest {
        username: "testemail@example.com".to_string(), // Using email as username
        password: "securepassword123".to_string(),
    };

    let response = client
        .login(login_request)
        .await
        .expect("Failed to login with email");

    assert!(response["user"]["id"].is_number());
    assert_eq!(response["user"]["username"], "testuser_email");
    assert_eq!(response["user"]["email"], "testemail@example.com");
    assert!(response["session_id"].is_string());

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn invalid_login_fails() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    let login_request = LoginRequest {
        username: "nonexistent".to_string(),
        password: "wrongpassword".to_string(),
    };

    let result = client.login(login_request).await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn me_endpoint_works() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    let response = client.me().await.expect("Failed to get user info");

    assert!(response["user_id"].is_number());
    assert!(response["username"].is_string());
    assert!(response["permissions"].is_number());

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn logout_works() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Logout should succeed
    let response = client.logout().await.expect("Failed to logout");

    assert_eq!(response["message"], "Logout successful");

    // Using the session after logout should fail
    let result = client.me().await;
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401 after logout");
    }

    cleanup_test_database(db_pool).await;
}
