use serde::Serialize;
use strum_macros::{AsRefStr, Display};

pub type ServerResult<T> = core::result::Result<T, ServerError>;

#[derive(Clone, Debug, Serialize, AsRefStr, Display)]
#[serde(tag = "type", content = "why")]
pub enum ServerError {
    NoAuthError,
    DatabaseConnectionError,
    DatabaseQueryError { why: String },
    AxumError { why: String },
    IOError { why: String },
    Error { why: String },
    NoMediaFound,
    WrongPathType { why: String },
    BadOperation { why: String },
    PathDoesntExist,
    PathAlreadyExists
}

impl From<sqlx::Error> for ServerError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseQueryError { why: value.to_string() }
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Error { why: value.to_string() }
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError { why: value.to_string() }
    }
}