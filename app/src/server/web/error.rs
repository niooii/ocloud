#![allow(unreachable_patterns)]

use axum::{http::StatusCode, response::{IntoResponse, Response}};
use serde::Serialize;
use strum_macros::AsRefStr;
use crate::server;

#[derive(Clone, Debug, AsRefStr, Serialize)]
#[allow(non_camel_case_types)]
pub enum ClientError {
    NO_AUTH,
    BAD_REQUEST {why: String},
    INTERNAL_ERROR {why: String}
}

impl server::error::ServerError { 
    pub fn to_status_and_client_error(&self) -> (StatusCode, ClientError) {
        match self {    
            Self::NoAuthError => (
                StatusCode::FORBIDDEN,
                ClientError::NO_AUTH
            ),
            Self::PathAlreadyExists => (
                StatusCode::BAD_REQUEST,
                ClientError::BAD_REQUEST { why: "Path already exists".to_string() }
            ),
            Self::PathDoesntExist => (
                StatusCode::BAD_REQUEST,
                ClientError::BAD_REQUEST { why: "Path does not exist".to_string() }
            ),
            Self::DatabaseConnectionError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::INTERNAL_ERROR { why: "Database connection error".to_string() }
            ),
            Self::DatabaseQueryError { why } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::INTERNAL_ERROR { why: why.clone() }
            ),
            Self::AxumError { why } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::INTERNAL_ERROR { why: why.clone() }
            ),
            Self::WrongPathType { why } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::BAD_REQUEST { why: why.clone() }
            ),
            Self::Error { why } => (
                StatusCode::BAD_REQUEST,
                ClientError::BAD_REQUEST { why: why.clone() }
            ),
            Self::IOError { why } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::INTERNAL_ERROR { why: why.clone() }
            ),
            Self::NoMediaFound => (
                StatusCode::BAD_REQUEST,
                ClientError::BAD_REQUEST { why: "No file found.".to_string() }
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::INTERNAL_ERROR { why: "Something went wrong...".to_string() }
            )
        }
    }
}               

impl IntoResponse for server::error::ServerError {
    fn into_response(self) -> Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        response.extensions_mut().insert(self);

        response
    }
}
