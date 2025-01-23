use serde::Serialize;
use strum_macros::AsRefStr;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, AsRefStr)]
#[serde(tag = "type", content = "why")]
pub enum Error {
    NoAuthError,
    DatabaseConnectionError,
    DatabaseQueryError { why: String },
    AxumError { why: String },
    IOError { why: String },
    Error { why: String },
    NoMediaFound,
    WrongPathType { why: String },
    PathDoesntExist,
    PathAlreadyExists
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseQueryError { why: value.to_string() }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Error { why: value.to_string() }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError { why: value.to_string() }
    }
}