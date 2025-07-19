mod common;

use common::TestApp;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::common::TEST_APP;

#[tokio::test]
async fn user_registration_works() {
    let client = Client::new();

    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    let response = client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["user"]["id"].is_number());
    assert_eq!(body["user"]["username"], "testuser");
    assert_eq!(body["user"]["email"], "test@example.com");
    assert!(body["user"]["created_at"].is_string());
    assert_eq!(body["message"], "User registered successfully");


}

#[tokio::test]
async fn duplicate_registration_fails() {
    let client = Client::new();

    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    // First registration should succeed
    let response = client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Second registration should fail
    let response = client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["error"], "Validation failed");


}

#[tokio::test]
async fn user_login_works() {
    let client = Client::new();

    // First register a user
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    // Then login
    let login_data = json!({
        "username": "testuser",
        "password": "securepassword123"
    });

    let response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["user"]["id"].is_number());
    assert_eq!(body["user"]["username"], "testuser");
    assert!(body["session_id"].is_string());
    assert!(body["expires_at"].is_string());
    assert_eq!(body["message"], "Login successful");
    
    // Validate session ID is a proper UUID
    let session_id = body["session_id"].as_str().unwrap();
    assert!(Uuid::parse_str(session_id).is_ok());


}

#[tokio::test]
async fn login_with_email_works() {
    let client = Client::new();

    // Register a user
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    // Login with email instead of username
    let login_data = json!({
        "username": "test@example.com",
        "password": "securepassword123"
    });

    let response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["user"]["username"], "testuser");
    assert_eq!(body["user"]["email"], "test@example.com");


}

#[tokio::test]
async fn invalid_login_fails() {
    let client = Client::new();

    // Register a user
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    // Try login with wrong password
    let login_data = json!({
        "username": "testuser",
        "password": "wrongpassword"
    });

    let response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["error"], "Authentication failed");


}

#[tokio::test]
async fn nonexistent_user_login_fails() {
    let client = Client::new();

    let login_data = json!({
        "username": "nonexistent",
        "password": "password"
    });

    let response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["error"], "Authentication failed");


}

#[tokio::test]
async fn me_endpoint_requires_authentication() {
    let client = Client::new();

    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn me_endpoint_works_with_valid_session() {
    let client = Client::new();

    // Register and login
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    let login_data = json!({
        "username": "testuser",
        "password": "securepassword123"
    });

    let login_response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    let login_body: Value = login_response.json().await.expect("Failed to parse login response");
    let session_id = login_body["session_id"].as_str().unwrap();

    // Use session to access /me
    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .header("Authorization", format!("Bearer {session_id}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["user_id"].is_number());
    assert_eq!(body["username"], "testuser");
    assert!(body["permissions"].is_number());


}

#[tokio::test]
async fn logout_works() {
    let client = Client::new();

    // Register and login
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "securepassword123"
    });

    client
        .post(format!("{}/auth/register", &TEST_APP.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    let login_data = json!({
        "username": "testuser",
        "password": "securepassword123"
    });

    let login_response = client
        .post(format!("{}/auth/login", &TEST_APP.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    let login_body: Value = login_response.json().await.expect("Failed to parse login response");
    let session_id = login_body["session_id"].as_str().unwrap();

    // Logout
    let response = client
        .post(format!("{}/auth/logout", &TEST_APP.address))
        .header("Authorization", format!("Bearer {session_id}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["message"], "Logout successful");

    // Try to use the session again - should fail
    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .header("Authorization", format!("Bearer {session_id}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn invalid_bearer_token_fails() {
    let client = Client::new();

    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .header("Authorization", "Bearer invalid-token")
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn missing_authorization_header_fails() {
    let client = Client::new();

    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn malformed_authorization_header_fails() {
    let client = Client::new();

    let response = client
        .get(format!("{}/auth/me", &TEST_APP.address))
        .header("Authorization", "InvalidFormat")
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

