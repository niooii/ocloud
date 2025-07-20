use ocloud::server::models::auth::password;

#[tokio::test]
async fn password_hashing_works() {
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
async fn password_verification_fails_for_wrong_password() {
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

#[tokio::test]
async fn same_password_produces_different_hashes() {
    let password = "same_password".to_string();

    let hash1 = password::hash_password(password.clone())
        .await
        .expect("Failed to hash password 1");

    let hash2 = password::hash_password(password.clone())
        .await
        .expect("Failed to hash password 2");

    // Hashes should be different due to unique salts
    assert_ne!(hash1, hash2);

    // But both should verify the same password
    let is_valid1 = password::verify_password(password.clone(), hash1)
        .await
        .expect("Failed to verify password 1");
    let is_valid2 = password::verify_password(password, hash2)
        .await
        .expect("Failed to verify password 2");

    assert!(is_valid1);
    assert!(is_valid2);
}

#[tokio::test]
async fn empty_password_can_be_hashed() {
    let password = "".to_string();

    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash empty password");

    assert!(hash.starts_with("$argon2id$"));

    let is_valid = password::verify_password(password, hash)
        .await
        .expect("Failed to verify empty password");

    assert!(is_valid);
}

#[tokio::test]
async fn long_password_works() {
    let password = "a".repeat(1000);

    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash long password");

    assert!(hash.starts_with("$argon2id$"));

    let is_valid = password::verify_password(password, hash)
        .await
        .expect("Failed to verify long password");

    assert!(is_valid);
}

#[tokio::test]
async fn unicode_password_works() {
    let password = "🔒 secure password with émojis and àccénts 🔑".to_string();

    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash unicode password");

    assert!(hash.starts_with("$argon2id$"));

    let is_valid = password::verify_password(password, hash)
        .await
        .expect("Failed to verify unicode password");

    assert!(is_valid);
}

#[tokio::test]
async fn invalid_hash_format_fails_gracefully() {
    let password = "test_password".to_string();
    let invalid_hash = "not_a_valid_hash".to_string();

    let result = password::verify_password(password, invalid_hash).await;

    // Should return an error, not panic
    assert!(result.is_err());
}

#[tokio::test]
async fn hash_consistency_test() {
    let password = "consistency_test".to_string();

    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash password");

    // Verify multiple times to ensure consistency
    for _ in 0..5 {
        let is_valid = password::verify_password(password.clone(), hash.clone())
            .await
            .expect("Failed to verify password");
        assert!(is_valid);
    }
}

#[tokio::test]
async fn timing_safety_test() {
    use std::time::Instant;

    let password = "timing_test_password".to_string();
    let hash = password::hash_password(password.clone())
        .await
        .expect("Failed to hash password");

    // Test correct password timing
    let start = Instant::now();
    let _valid = password::verify_password(password, hash.clone())
        .await
        .expect("Failed to verify correct password");
    let correct_duration = start.elapsed();

    // Test wrong password timing
    let start = Instant::now();
    let _invalid = password::verify_password("wrong_password".to_string(), hash)
        .await
        .expect("Failed to verify wrong password");
    let wrong_duration = start.elapsed();

    // The timing should be similar (within reasonable bounds)
    // This is a basic test - in practice, timing attacks are more sophisticated
    let ratio = if correct_duration > wrong_duration {
        correct_duration.as_nanos() as f64 / wrong_duration.as_nanos() as f64
    } else {
        wrong_duration.as_nanos() as f64 / correct_duration.as_nanos() as f64
    };

    // Allow up to 10x difference (this is generous, but accounts for system variance)
    assert!(ratio < 10.0, "Timing difference too large: {ratio:.2}x");
}
