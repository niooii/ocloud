mod common;

use axum::http::StatusCode;
use common::{authenticate_random, cleanup_test_database, create_test_db, create_multiple_users};
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
        .change_file_visibility("root/testfile.txt", true)
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
        .change_file_visibility("root/public_dir/", true)
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
        .change_file_visibility("root/public_to_delete.txt", true)
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

/// Test unified endpoint: set visibility only
#[tokio::test]
async fn unified_endpoint_set_visibility_only() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Upload a file
    let file_content = b"test file for visibility".to_vec();
    let files = client
        .upload_file("root", "visibility_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    assert!(!files[0].is_public); // Should be private by default

    // Use unified endpoint to set visibility only
    let updated_file = client
        .set_permissions_and_visibility("root/visibility_test.txt", Some(true), None)
        .await
        .expect("Failed to set visibility");

    assert!(updated_file.is_public);
    assert_eq!(updated_file.top_level_name, "visibility_test.txt");

    // Test setting back to private
    let updated_file = client
        .set_permissions_and_visibility("root/visibility_test.txt", Some(false), None)
        .await
        .expect("Failed to set visibility to private");

    assert!(!updated_file.is_public);

    cleanup_test_database(db_pool).await;
}

/// Test unified endpoint: grant permissions only
#[tokio::test]
async fn unified_endpoint_grant_permissions_only() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 2).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut viewer_client, _viewer_info, viewer_id) = users.remove(0);

    // Owner uploads a private file
    let file_content = b"private file for permissions test".to_vec();
    let files = owner_client
        .upload_file("root", "permissions_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    assert!(!files[0].is_public); // Should be private by default

    // Viewer should not be able to access private file initially
    let result = viewer_client
        .get_file("root/permissions_test.txt", files[0].user_id)
        .await;
    assert!(result.is_err());

    // Owner grants viewer permission using unified endpoint
    let perm_op = ocloud::api::PermissionOperation {
        target_user_id: viewer_id,
        relationship: "viewer".to_string(),
        action: "grant".to_string(),
    };
    
    let updated_file = owner_client
        .set_permissions_and_visibility("root/permissions_test.txt", None, Some(perm_op))
        .await
        .expect("Failed to grant permissions");

    assert_eq!(updated_file.top_level_name, "permissions_test.txt");
    assert!(!updated_file.is_public); // Should still be private

    // Viewer should now be able to access the file
    let content = viewer_client
        .get_file("root/permissions_test.txt", files[0].user_id)
        .await
        .expect("Viewer should be able to access file after permission grant");

    assert_eq!(content, file_content);

    cleanup_test_database(db_pool).await;
}

/// Test unified endpoint: revoke permissions
#[tokio::test]
async fn unified_endpoint_revoke_permissions() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 2).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut editor_client, _editor_info, editor_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"file for revoke test".to_vec();
    let files = owner_client
        .upload_file("root", "revoke_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    // Grant editor permission first
    let _updated_file = owner_client
        .grant_file_permission("root/revoke_test.txt", editor_id, "editor")
        .await
        .expect("Failed to grant editor permissions");

    // Editor should be able to access the file
    let content = editor_client
        .get_file("root/revoke_test.txt", files[0].user_id)
        .await
        .expect("Editor should be able to access file");
    assert_eq!(content, file_content);

    // Owner revokes editor permission
    let _updated_file = owner_client
        .revoke_file_permission("root/revoke_test.txt", editor_id, "editor")
        .await
        .expect("Failed to revoke permissions");

    // Editor should no longer be able to access the file
    let result = editor_client
        .get_file("root/revoke_test.txt", files[0].user_id)
        .await;
    assert!(result.is_err(), "Editor should not be able to access file after permission revocation");

    cleanup_test_database(db_pool).await;
}

/// Test unified endpoint: set both visibility and permissions
#[tokio::test]
async fn unified_endpoint_set_visibility_and_permissions() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 2).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut viewer_client, _viewer_info, viewer_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"file for combined test".to_vec();
    let files = owner_client
        .upload_file("root", "combined_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    assert!(!files[0].is_public);

    // Set both visibility and permissions in one request
    let perm_op = ocloud::api::PermissionOperation {
        target_user_id: viewer_id,
        relationship: "viewer".to_string(),
        action: "grant".to_string(),
    };
    
    let updated_file = owner_client
        .set_permissions_and_visibility("root/combined_test.txt", Some(true), Some(perm_op))
        .await
        .expect("Failed to set visibility and permissions");

    assert!(updated_file.is_public);
    assert_eq!(updated_file.top_level_name, "combined_test.txt");

    // Viewer should be able to access the now-public file
    let content = viewer_client
        .get_file("root/combined_test.txt", files[0].user_id)
        .await
        .expect("Viewer should be able to access public file");
    assert_eq!(content, file_content);

    // Anonymous user should also be able to access the public file
    viewer_client.clear_session();
    let content = viewer_client
        .get_file("root/combined_test.txt", files[0].user_id)
        .await
        .expect("Anonymous user should be able to access public file");
    assert_eq!(content, file_content);

    cleanup_test_database(db_pool).await;
}

/// Test error cases: non-owner trying to change permissions
#[tokio::test]
async fn unified_endpoint_non_owner_cannot_change_permissions() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 3).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut non_owner_client, _non_owner_info, _non_owner_id) = users.remove(0);
    let (_third_client, _third_info, third_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"protected file".to_vec();
    let _files = owner_client
        .upload_file("root", "protected.txt", file_content)
        .await
        .expect("Failed to upload file");

    // Non-owner tries to grant permissions to third user
    let perm_op = ocloud::api::PermissionOperation {
        target_user_id: third_id,
        relationship: "viewer".to_string(),
        action: "grant".to_string(),
    };
    
    let result = non_owner_client
        .set_permissions_and_visibility("root/protected.txt", None, Some(perm_op))
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::NOT_FOUND);
    } else {
        panic!("Expected HTTP 404 error - from the server's perspective the file does not exist for the client, as long as it's concerned");
    }

    cleanup_test_database(db_pool).await;
}

/// Test error cases: invalid permission action
#[tokio::test]
async fn unified_endpoint_invalid_permission_action() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 2).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (_viewer_client, _viewer_info, viewer_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"test file".to_vec();
    let _files = owner_client
        .upload_file("root", "test.txt", file_content)
        .await
        .expect("Failed to upload file");

    // Try invalid permission action
    let perm_op = ocloud::api::PermissionOperation {
        target_user_id: viewer_id,
        relationship: "viewer".to_string(),
        action: "invalid_action".to_string(),
    };
    
    let result = owner_client
        .set_permissions_and_visibility("root/test.txt", None, Some(perm_op))
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::BAD_REQUEST);
    } else {
        panic!("Expected HTTP 400 error for invalid action");
    }

    cleanup_test_database(db_pool).await;
}

/// Test error cases: operating on non-existent file
#[tokio::test]
async fn unified_endpoint_nonexistent_file() {
    let db_pool = create_test_db().await;
    let mut client = ApiClient::new_local(db_pool.clone()).await;
    let _user = authenticate_random(&mut client).await;

    // Try to set visibility on non-existent file
    let result = client
        .set_permissions_and_visibility("root/nonexistent.txt", Some(true), None)
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::NOT_FOUND);
    } else {
        panic!("Expected HTTP 404 error for non-existent file");
    }

    cleanup_test_database(db_pool).await;
}

/// Test permission inheritance: editor can access but not change permissions
#[tokio::test]
async fn permission_hierarchy_editor_limitations() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 3).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut editor_client, _editor_info, editor_id) = users.remove(0);
    let (_viewer_client, _viewer_info, viewer_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"file for hierarchy test".to_vec();
    let files = owner_client
        .upload_file("root", "hierarchy_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    // Owner grants editor permission
    let _updated_file = owner_client
        .grant_file_permission("root/hierarchy_test.txt", editor_id, "editor")
        .await
        .expect("Failed to grant editor permissions");

    // Editor should be able to access the file
    let content = editor_client
        .get_file("root/hierarchy_test.txt", files[0].user_id)
        .await
        .expect("Editor should be able to access file");
    assert_eq!(content, file_content);

    // Editor should NOT be able to grant permissions to others
    let perm_op = ocloud::api::PermissionOperation {
        target_user_id: viewer_id,
        relationship: "viewer".to_string(),
        action: "grant".to_string(),
    };
    
    let result = editor_client
        .set_permissions_and_visibility("root/hierarchy_test.txt", None, Some(perm_op))
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::NOT_FOUND);
    } else {
        panic!("Expected HTTP 404 error - editors cannot grant permissions, because the files are not owned by the user and the server will not consider shared files.");
    }

    cleanup_test_database(db_pool).await;
}

/// Test that viewers can only read files
#[tokio::test]
async fn permission_hierarchy_viewer_read_only() {
    let db_pool = create_test_db().await;
    let mut users = create_multiple_users(&db_pool, 2).await;
    let (mut owner_client, _owner_info, _owner_id) = users.remove(0);
    let (mut viewer_client, _viewer_info, viewer_id) = users.remove(0);

    // Owner uploads a file
    let file_content = b"file for viewer test".to_vec();
    let files = owner_client
        .upload_file("root", "viewer_test.txt", file_content.clone())
        .await
        .expect("Failed to upload file");

    // Owner grants viewer permission
    let _updated_file = owner_client
        .grant_file_permission("root/viewer_test.txt", viewer_id, "viewer")
        .await
        .expect("Failed to grant viewer permissions");

    // Viewer should be able to read the file
    let content = viewer_client
        .get_file("root/viewer_test.txt", files[0].user_id)
        .await
        .expect("Viewer should be able to read file");
    assert_eq!(content, file_content);

    // Viewer should NOT be able to delete the file
    let result = viewer_client
        .delete_file("root/viewer_test.txt")
        .await;
    assert!(result.is_err());

    // Viewer should NOT be able to change visibility
    let result = viewer_client
        .set_permissions_and_visibility("root/viewer_test.txt", Some(true), None)
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body: _ }) = result {
        assert_eq!(status, StatusCode::NOT_FOUND);
    } else {
        panic!("Expected HTTP 404 error - viewers of shared files cannot change permissions of said shared files, from the server's perspective, the shared files do not exist.");
    }

    cleanup_test_database(db_pool).await;
}