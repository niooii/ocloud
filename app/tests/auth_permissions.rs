mod common;

use common::TestApp;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

async fn register_and_login(app: &TestApp, username: &str, email: &str, password: &str) -> String {
    let client = Client::new();

    // Register user
    let user_data = json!({
        "username": username,
        "email": email,
        "password": password
    });

    client
        .post(format!("{}/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    // Login and get session
    let login_data = json!({
        "username": username,
        "password": password
    });

    let login_response = client
        .post(format!("{}/auth/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    let login_body: Value = login_response.json().await.expect("Failed to parse login response");
    login_body["session_id"].as_str().unwrap().to_string()
}


#[tokio::test]
async fn cannot_view_permissions_without_access() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let user_session = register_and_login(&app, "testuser", "test@example.com", "password123").await;

    // Try to view permissions for a resource the user has no access to
    let response = client
        .get(format!("{}/auth/permissions/sfile/999", &app.address))
        .header("Authorization", format!("Bearer {user_session}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["error"], "Access denied");

    app.cleanup().await;
}