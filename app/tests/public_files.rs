use ocloud::server::controllers::files::FileVisibility;
use reqwest::StatusCode;

use crate::common::TEST_APP;

mod common;

/// Test that anonymous users can access public files
#[tokio::test]
async fn anonymous_user_can_access_public_files() {
    let app = common::TestApp::spawn().await;
    
    // Create an authenticated user and upload a file
    let authenticated_client = TEST_APP.create_authenticated_user("user1", "user1@example.com", "password").await;

    // Upload a file as authenticated user
    let response = authenticated_client
        .post(format!("{}/files/root/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("testfile.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    // Make the file public using the controller API
    TEST_APP.state.file_controller.set_file_visibility_by_name("testfile.txt", FileVisibility::Public)
        .await
        .expect("Failed to make file public");

    // Now try to access the file as anonymous user (no auth header)
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .get(format!("{}/files/root/testfile.txt", TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);


}

/// Test that anonymous users cannot access private files
#[tokio::test]
async fn anonymous_user_cannot_access_private_files() {
    let app = common::TestApp::spawn().await;

    // Create an authenticated user and upload a private file
    let authenticated_client = TEST_APP.create_authenticated_user("user2", "user2@example.com", "password").await;

    // Upload a private file as authenticated user (default is private)
    let response = authenticated_client
        .post(format!("{}/files/root/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("privatefile.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Try to access the private file as anonymous user (no auth header)
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .get(format!("{}/files/root/privatefile.txt", TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    // Should get 401 Unauthorized for private files when not authenticated
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

/// Test that anonymous users can list public directories
#[tokio::test]
async fn anonymous_user_can_list_public_directories() {
    let app = common::TestApp::spawn().await;

    // Create an authenticated user and upload files in a directory
    let authenticated_client = TEST_APP.create_authenticated_user("user3", "user3@example.com", "password").await;

    // Create a directory by uploading files to it
    let response = authenticated_client
        .post(format!("{}/files/root/public_dir/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("file1.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Make the directory public using the controller API
    TEST_APP.state.file_controller.set_directory_visibility_by_name("public_dir", FileVisibility::Public)
        .await
        .expect("Failed to make directory public");

    // Try to list the public directory as anonymous user
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .get(format!("{}/files/root/public_dir/", TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);

    // Should be able to see the directory contents
    let files: Vec<serde_json::Value> = response.json().await.expect("Failed to parse JSON");
    assert!(!files.is_empty());


}

/// Test that anonymous users cannot list private directories
#[tokio::test]
async fn anonymous_user_cannot_list_private_directories() {
    let app = common::TestApp::spawn().await;

    // Create an authenticated user and upload files in a private directory
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "testuser@example.com", "password").await;

    // Create a private directory by uploading files to it (default is private)
    let response = authenticated_client
        .post(format!("{}/files/root/private_dir/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("file1.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Try to list the private directory as anonymous user
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .get(format!("{}/files/root/private_dir/", TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    // Should get 401 Unauthorized for private directories when not authenticated
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

/// Test that anonymous users cannot upload files
#[tokio::test]
async fn anonymous_user_cannot_upload_files() {
    let app = common::TestApp::spawn().await;

    // Try to upload a file as anonymous user (no auth header)
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .post(format!("{}/files/root/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("anonymous_file.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");
    
    // Should get 401 Unauthorized for upload attempts when not authenticated
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}

/// Test that anonymous users cannot delete files (even public ones)
#[tokio::test]
async fn anonymous_user_cannot_delete_files() {
    let app = common::TestApp::spawn().await;

    // First, create a user and upload a public file as authenticated user
    let authenticated_client = TEST_APP.create_authenticated_user("testuser", "testuser@example.com", "password").await;

    // Upload a file and make it public
    let response = authenticated_client
        .post(format!("{}/files/root/", TEST_APP.address))
        .multipart(
            common::TestApp::test_multipart_file("public_to_delete.txt")
        )
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Make the file public using the controller API
    TEST_APP.state.file_controller.set_file_visibility_by_name("public_to_delete.txt", FileVisibility::Public)
        .await
        .expect("Failed to make file public");

    // Try to delete the public file as anonymous user (no auth header)
    let anonymous_client = TEST_APP.anonymous_client();
    let response = anonymous_client
        .delete(format!("{}/files/root/public_to_delete.txt", TEST_APP.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    // Should get 401 Unauthorized - even public files require authentication to delete
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);


}