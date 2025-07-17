use crate::server::error::{ServerError, ServerResult};
use std::path::Path;

pub fn validate_filename(filename: &str) -> ServerResult<()> {
    if filename.is_empty() {
        return Err(ServerError::ValidationError {
            field: "filename cannot be empty".to_string(),
        });
    }

    if filename.len() > 255 {
        return Err(ServerError::ValidationError {
            field: "filename too long (max 255 characters)".to_string(),
        });
    }

    let invalid_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*', '\0'];
    if filename.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(ServerError::ValidationError {
            field: "filename contains invalid characters".to_string(),
        });
    }

    if filename.starts_with('.') && filename.len() <= 2 {
        return Err(ServerError::ValidationError {
            field: "filename cannot be '.' or '..'".to_string(),
        });
    }

    Ok(())
}

pub fn validate_path(path: &str) -> ServerResult<()> {
    if path.is_empty() {
        return Err(ServerError::ValidationError {
            field: "path cannot be empty".to_string(),
        });
    }

    if path.len() > 4096 {
        return Err(ServerError::ValidationError {
            field: "path too long (max 4096 characters)".to_string(),
        });
    }

    if path.contains('\0') {
        return Err(ServerError::ValidationError {
            field: "path contains null bytes".to_string(),
        });
    }

    if path.contains("..") {
        return Err(ServerError::ValidationError {
            field: "path traversal not allowed".to_string(),
        });
    }

    Ok(())
}

pub fn sanitize_path_component(component: &str) -> String {
    component
        .chars()
        .filter(|&c| c.is_alphanumeric() || "._-".contains(c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_filename() {
        assert!(validate_filename("valid_file.txt").is_ok());
        assert!(validate_filename("").is_err());
        assert!(validate_filename("file/with/slash").is_err());
        assert!(validate_filename("file<with>brackets").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
        assert!(validate_filename(&"a".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/valid/path").is_ok());
        assert!(validate_path("").is_err());
        assert!(validate_path("path/../traversal").is_err());
        assert!(validate_path("path\0with\0nulls").is_err());
        assert!(validate_path(&"a".repeat(4097)).is_err());
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("hello-world_123.txt"), "hello-world_123.txt");
        assert_eq!(sanitize_path_component("hello<>world"), "helloworld");
        assert_eq!(sanitize_path_component("file/with/slashes"), "filewithslashes");
    }
}