mod common;

use common::{TestApp};
use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TEST_APP;


#[tokio::test]
async fn unauthenticated_file_access_fails() {
    
    // Try to access files without authentication
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .get(format!("{}/files/root/", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn unauthenticated_file_upload_fails() {
    
    // Try to upload file without authentication
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("testfile.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn unauthenticated_file_delete_fails() {
    
    // Try to delete file without authentication
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .delete(format!("{}/files/root/nonexistent.txt", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

#[tokio::test]
async fn authenticated_user_can_upload_files() {
    
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "test@example.com", "password").await;

    // Upload a file
    let response = authenticated_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("testfile.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());


}

#[tokio::test]
async fn authenticated_user_can_create_directories() {
    
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "test@example.com", "password").await;

    // Create a directory
    let response = authenticated_client
        .post(format!("{}/files/root/testdir/", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to create directory");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body.is_array());


}

#[tokio::test]
async fn file_owner_can_access_their_files() {
    
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "test@example.com", "password").await;

    // First upload a file
    let upload_response = authenticated_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("testfile.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(upload_response.status(), StatusCode::OK);

    // Now try to access the file
    let response = authenticated_client
        .get(format!("{}/files/root/testfile.txt", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to access file");

    assert_eq!(response.status(), StatusCode::OK);


}

#[tokio::test]
async fn file_owner_can_delete_their_files() {
    
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "test@example.com", "password").await;

    // First upload a file
    let upload_response = authenticated_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("deleteme.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(upload_response.status(), StatusCode::OK);

    // Now delete the file
    let response = authenticated_client
        .delete(format!("{}/files/root/deleteme.txt", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to delete file");

    assert_eq!(response.status(), StatusCode::OK);


}

#[tokio::test]
async fn non_owner_cannot_access_other_users_files() {
    let app = TestApp::spawn().await;

    // Create two users
    let owner_client = TEST_APP.create_authenticated_user("user1", "user1@example.com", "password").await;
    let other_client = TEST_APP.create_authenticated_user("user2", "user2@example.com", "password").await;

    // Owner uploads a file
    let upload_response = owner_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("private.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(upload_response.status(), StatusCode::OK);

    // Other user tries to access the file (should fail)
    let response = other_client
        .get(format!("{}/files/root/private.txt", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);


}

#[tokio::test]
async fn non_owner_cannot_delete_other_users_files() {
    let app = TestApp::spawn().await;

    // Create two users
    let owner_client = TEST_APP.create_authenticated_user("user1", "user1@example.com", "password").await;
    let other_client = TEST_APP.create_authenticated_user("user2", "user2@example.com", "password").await;

    // Owner uploads a file
    let upload_response = owner_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("protected.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(upload_response.status(), StatusCode::OK);

    // Other user tries to delete the file (should fail)
    let response = other_client
        .delete(format!("{}/files/root/protected.txt", &TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);


}

#[tokio::test]
async fn non_owner_cannot_move_other_users_files() {
    let app = TestApp::spawn().await;

    // Create two users
    let owner_client = TEST_APP.create_authenticated_user("user1", "user1@example.com", "password").await;
    let other_client = TEST_APP.create_authenticated_user("user2", "user2@example.com", "password").await;

    // Owner uploads a file
    let upload_response = owner_client
        .post(format!("{}/files/root/", &TEST_APP.address))
        .multipart(
            TestApp::test_multipart_file("moveme.txt")
        )
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(upload_response.status(), StatusCode::OK);

    // Other user tries to move the file (should fail)
    let move_data = json!({
        "from": "root/moveme.txt",
        "to": "root/moved.txt"
    });

    let response = other_client
        .patch(format!("{}/files", &TEST_APP.address))
        .json(&move_data)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);


}