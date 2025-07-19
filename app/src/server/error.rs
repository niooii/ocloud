use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use tracing::{error, warn};

pub type ServerResult<T> = core::result::Result<T, ServerError>;

#[derive(Error, Debug, Clone)]
pub enum ServerError {
    #[error("Authentication required")]
    NoAuthError,
    #[error("Database connection failed")]
    DatabaseConnectionError,
    #[error("Database query failed: {message}")]
    DatabaseQueryError { message: String },
    #[error("HTTP server error: {message}")]
    AxumError { message: String },
    #[error("I/O operation failed: {message}")]
    IOError { message: String },
    #[error("Internal server error: {message}")]
    InternalError { message: String },
    #[error("Media not found")]
    NoMediaFound,
    #[error("Invalid path type: {details}")]
    WrongPathType { details: String },
    #[error("Invalid operation: {details}")]
    BadOperation { details: String },
    #[error("Path does not exist")]
    PathDoesntExist,
    #[error("Path already exists")]
    PathAlreadyExists,
    #[error("Validation failed: {message}")]
    ValidationError { message: String },
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String },
    #[error("Authorization failed: {message}")]
    AuthorizationError { message: String },
    #[error("Database error: {message}")]
    DatabaseError { message: String },
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl ServerError {
    pub fn to_status_and_client_error(&self) -> (StatusCode, ErrorResponse) {
        match self {
            ServerError::NoAuthError => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: "Authentication required".to_string(),
                    details: None,
                },
            ),
            ServerError::NoMediaFound | ServerError::PathDoesntExist => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: "Resource not found".to_string(),
                    details: None,
                },
            ),
            ServerError::ValidationError { message } => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: "Validation failed".to_string(),
                    details: Some(message.clone()),
                },
            ),
            ServerError::AuthenticationError { message } => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: "Authentication failed".to_string(),
                    details: Some(message.clone()),
                },
            ),
            ServerError::AuthorizationError { message } => (
                StatusCode::FORBIDDEN,
                ErrorResponse {
                    error: "Access denied".to_string(),
                    details: Some(message.clone()),
                },
            ),
            ServerError::PathAlreadyExists => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error: "Resource already exists".to_string(),
                    details: None,
                },
            ),
            ServerError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorResponse {
                    error: "Rate limit exceeded".to_string(),
                    details: Some("Please retry after some time".to_string()),
                },
            ),
            ServerError::DatabaseConnectionError
            | ServerError::DatabaseQueryError { .. }
            | ServerError::DatabaseError { .. }
            | ServerError::IOError { .. }
            | ServerError::InternalError { .. } => {
                error!("Internal server error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Internal server error".to_string(),
                        details: None,
                    },
                )
            }
            _ => {
                error!("Unhandled server error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Internal server error".to_string(),
                        details: None,
                    },
                )
            }
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        // Warn about the error happening (maybe we want to see!)
        warn!("Server error occurred: {:?}", self);
        let (status, error_response) = self.to_status_and_client_error();
        (status, Json(error_response)).into_response()
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(value: serde_json::Error) -> Self {
        Self::InternalError { message: format!("JSON serialization error: {value}") }
    }
}

impl From<sqlx::Error> for ServerError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseQueryError { message: value.to_string() }
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError { message: value.to_string() }
    }
}