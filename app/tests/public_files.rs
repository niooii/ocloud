mod common;

use axum::http::StatusCode;
use common::{authenticate_random, cleanup_test_database, create_test_db};
use ocloud::api::{ApiClient, ApiError};

/// Test that anonymous users can access public files
#[tokio::test]
async fn anon_can_access_public_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // Create an authenticated user and upload a file
    let _user = authenticate_random(&mut client).await;

    // Upload a file as authenticated user
    let file_content = b"test file content".to_vec();
    let files = client
        .upload_file("root", "testfile.txt", file_content)
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());
    let uploaded_file = &files[0];
    assert_eq!(uploaded_file.top_level_name, "testfile.txt");
    assert!(!uploaded_file.is_public); // Should be private by default

    // Make the file public
    let updated_file = client
        .change_file_visibility("root/testfile.txt", "public")
        .await
        .expect("Failed to make file public");

    assert!(updated_file.is_public);

    // Now try to access the file as anonymous user (clear session)
    client.clear_session();
    let content = client
        .get_file("root/testfile.txt", updated_file.user_id)
        .await
        .expect("Failed to access public file anonymously");

    assert_eq!(content, b"test file content".to_vec());

    cleanup_test_database(db_pool).await;
}

/// Test that anonymous users cannot access private files
#[tokio::test]
async fn anon_cannot_access_private_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // Create an authenticated user and upload a private file
    let _user = authenticate_random(&mut client).await;

    // Upload a private file as authenticated user (default is private)
    let file_content = b"private content".to_vec();
    let files = client
        .upload_file("root", "privatefile.txt", file_content)
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());
    assert!(!files[0].is_public); // Should be private by default

    // Try to access the private file as anonymous user (clear session)
    client.clear_session();
    let result = client
        .get_file("root/privatefile.txt", files[0].user_id)
        .await;

    // Should get 401 Unauthorized for private files when not authenticated
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

/// Test that anonymous users can list public directories
#[tokio::test]
async fn anon_can_list_public_directories() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // Create an authenticated user and upload files in a directory
    let _user = authenticate_random(&mut client).await;

    // Create a directory by uploading files to it
    let file_content = b"file1 content".to_vec();
    let files = client
        .upload_file("root/public_dir", "file1.txt", file_content)
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());

    // Make the directory public by making its content public
    client
        .change_file_visibility("root/public_dir/", "public")
        .await
        .expect("Failed to make directory public");

    // Try to list the public directory as anonymous user (clear session)
    client.clear_session();
    let files = client
        .list_directory("root/public_dir", files[0].user_id)
        .await
        .expect("Failed to list public directory");

    assert!(!files.is_empty());
    assert!(files.iter().any(|f| f.top_level_name == "file1.txt"));

    cleanup_test_database(db_pool).await;
}

/// Test that anonymous users cannot list private directories
#[tokio::test]
async fn anon_cannot_list_private_directories() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // Create an authenticated user and upload files in a private directory
    let _user = authenticate_random(&mut client).await;

    // Create a private directory by uploading files to it (default is private)
    let file_content = b"private file content".to_vec();
    let files = client
        .upload_file("root/private_dir", "file1.txt", file_content)
        .await
        .expect("Failed to upload file");

    // Try to list the private directory as anonymous user (clear session)
    client.clear_session();
    let result = client
        .list_directory("root/private_dir", files[0].user_id)
        .await;

    // Should get 401 Unauthorized for private directories when not authenticated
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

/// Test that anonymous users cannot upload files
#[tokio::test]
async fn anon_cannot_upload_files() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    // Try to upload a file as anonymous user (no session)
    let file_content = b"anonymous content".to_vec();
    let result = client
        .upload_file("root", "anonymous_file.txt", file_content)
        .await;

    // Should get 401 Unauthorized for upload attempts when not authenticated
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

/// Test that anonymous users cannot delete files (even public ones)
#[tokio::test]
async fn anon_cannot_delete_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // First, create a user and upload a public file as authenticated user
    let _user = authenticate_random(&mut client).await;

    // Upload a file and make it public
    let file_content = b"public file to delete".to_vec();
    let files = client
        .upload_file("root", "public_to_delete.txt", file_content)
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());

    // Make the file public
    client
        .change_file_visibility("root/public_to_delete.txt", "public")
        .await
        .expect("Failed to make file public");

    // Try to delete the public file as anonymous user (clear session)
    client.clear_session();
    let result = client.delete_file("root/public_to_delete.txt").await;

    // Should get 401 Unauthorized - even public files require authentication to delete
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

/// Test that authenticated users can access their own files
#[tokio::test]
async fn authenticated_user_can_access_own_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Upload a private file
    let file_content = b"owner's private file".to_vec();
    let files = client
        .upload_file("root", "owners_file.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());
    assert!(!files[0].is_public); // Should be private by default

    // Owner should be able to access their own private file
    let content = client
        .get_file("root/owners_file.txt", files[0].user_id)
        .await
        .expect("Owner should be able to access own file");

    assert_eq!(content, file_content);

    cleanup_test_database(db_pool).await;
}

/// Test file move functionality
#[tokio::test]
async fn authenticated_user_can_move_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Upload a file
    let file_content = b"file to move".to_vec();
    let files = client
        .upload_file("root", "original.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    assert!(!files.is_empty());

    // Move the file
    let moved_file = client
        .move_file("root/original.txt", "root/moved.txt")
        .await
        .expect("Failed to move file");

    assert_eq!(moved_file.top_level_name, "moved.txt");

    // Should be able to access file at new location
    let content = client
        .get_file("root/moved.txt", None)
        .await
        .expect("Failed to access moved file");

    assert_eq!(content, file_content);

    // Should not be able to access file at old location
    let result = client.get_file("root/original.txt", None).await;
    assert!(result.is_err());

    cleanup_test_database(db_pool).await;
}

/// Test file move functionality
#[tokio::test]
async fn anon_can_access_root() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    let ls_result = client.list_directory("root/", None).await;
    assert!(ls_result.is_err());
    cleanup_test_database(db_pool).await;
}