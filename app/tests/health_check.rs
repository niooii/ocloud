mod common;

use common::TestApp;

use crate::common::TEST_APP;

#[tokio::test]
async fn health_check_works() {
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}