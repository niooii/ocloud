mod common;

use axum::http::StatusCode;
use common::{authenticate_random, cleanup_test_database, create_test_db};
use ocloud::api::{ApiClient, ApiError};

#[tokio::test]
async fn unauthenticated_file_access_fails() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    authenticate_random(&mut client).await;

    let files = client
        .upload_file("root/", "testfile", "hey man test file".into())
        .await
        .unwrap();
    let sfile = &files[0];

    let unauth = ApiClient::new_local(db_pool.clone()).await;

    // Try to access files without authentication (no session)
    let result = unauth
        .get_file(&format!("root/testfile?u={}", sfile.user_id.unwrap()), None)
        .await;

    // This should fail for non-public files
    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn authenticated_user_can_access_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Try to access files with authentication
    let result = client.get_file("root/", None).await;

    // This should succeed (even if the directory is empty)
    // The important part is that we get a successful response, not 401/403
    if let Err(ApiError::Http { status, body }) = result {
        panic!("Request failed with status {status}: {body}");
    }

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn authenticated_user_can_delete_files() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Try to delete a non-existent file (should fail with 404, not auth error)
    let result = client.delete_file("root/nonexistent.txt").await;

    if let Err(ApiError::Http { status, body: _ }) = result {
        // Should fail with 404 (not found), not 401/403 (auth errors)
        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "Should not be unauthorized"
        );
        assert_ne!(status, StatusCode::FORBIDDEN, "Should not be forbidden");
    } else {
        // It's also possible the operation succeeds if the file handling allows it
        panic!("Expected error for non-existent file deletion");
    }

    cleanup_test_database(db_pool).await;
}

#[tokio::test]
async fn invalid_session_fails() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;

    // Try to access files with invalid session
    client.set_session("invalid_session".to_string());
    let result = client.get_file("root/", None).await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    } else {
        panic!("Expected HTTP error with status 401");
    }

    cleanup_test_database(db_pool).await;
}
