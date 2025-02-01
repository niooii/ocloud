use reqwest::StatusCode;
use crate::{server, Cli};

pub type CliResult<T> = std::result::Result<T, CliError>;

#[derive(Debug)]
pub enum CliError {
    ServerError { err: server::error::ServerError },
    NoFileFound,
    IoError { err: String },
    ReqwestError { err: reqwest::Error },
    FailStatusCode { status_code: StatusCode },
    UrlParseError { issue: String },
    DockerError { err: bollard::errors::Error }
}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError { err: value.to_string() }
    }
}

impl From<reqwest::Error> for CliError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError { err: value }
    }
}

impl From<url::ParseError> for CliError {
    fn from(value: url::ParseError) -> Self {
        Self::UrlParseError { issue: value.to_string() }
    }
}

impl From<server::error::ServerError> for CliError {
    fn from(value: server::error::ServerError) -> Self {
        Self::ServerError { err: value }
    }
}

impl From<bollard::errors::Error> for CliError {
    fn from(value: bollard::errors::Error) -> Self {
        Self::DockerError { err: value }
    }
}