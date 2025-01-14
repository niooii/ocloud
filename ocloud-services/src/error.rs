use serde::Serialize;
use strum_macros::AsRefStr;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, AsRefStr)]
#[serde(tag = "type", content = "why")]
pub enum Error {
    NoAuthError,
    DatabaseConnectionError,
    DatabaseQueryError,
    AxumError { why: String },
    IOError { why: String },
    Error { why: String },
    // This can also be returned if the checksum provided was invalid
    NoMediaFound
}