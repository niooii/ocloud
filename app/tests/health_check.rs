mod common;

use common::TestApp;

#[tokio::test]
async fn health_check_works() {
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}