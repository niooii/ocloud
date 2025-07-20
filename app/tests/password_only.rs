// Simple test for password functionality only

#[tokio::test]
async fn password_hashing_basic_test() {
    use ocloud::server::models::auth::password;

    let password = "test_password_123".to_string();

    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash password");

    // Verify the hash is in PHC format (starts with $argon2id$)
    assert!(hash.starts_with("$argon2id$"));

    // Verify the password can be verified
    let is_valid = password::verify_password(password, hash)
        .await
        .expect("Failed to verify password");

    assert!(is_valid);
}

#[tokio::test]
async fn password_wrong_verification() {
    use ocloud::server::models::auth::password;

    let correct_password = "correct_password".to_string();
    let wrong_password = "wrong_password".to_string();

    let hash = password::hash_password(correct_password)
        .await
        .expect("Failed to hash password");

    let is_valid = password::verify_password(wrong_password, hash)
        .await
        .expect("Failed to verify password");

    assert!(!is_valid);
}
