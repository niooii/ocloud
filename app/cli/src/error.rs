use reqwest::{StatusCode, Url};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NoFileFound,
    IoError { err: String },
    ReqwestError { err: reqwest::Error },
    FailStatusCode { status_code: StatusCode },
    UrlParseError { issue: String },
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError { err: value.to_string() }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError { err: value.into() }
    }
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Self::UrlParseError { issue: value.to_string() }
    }
}