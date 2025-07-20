use axum::body::to_bytes;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use reqwest;
use serde::Serialize;
use sqlx::PgPool;
use std::convert::Infallible;
use std::sync::Arc;
use thiserror::Error;
use tower::Service;

use crate::server::{create_server, models::auth::*, models::files::SFile};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP error {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Service error: {0}")]
    Service(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Body conversion error: {0}")]
    Body(#[from] axum::Error),
    #[error("Infallible error")]
    Infallible(#[from] Infallible),
}

// Helper structs to make requests serializable
#[derive(Debug, Serialize)]
pub struct ApiRegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl From<RegisterRequest> for ApiRegisterRequest {
    fn from(req: RegisterRequest) -> Self {
        Self {
            username: req.username,
            email: req.email,
            password: req.password,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiLoginRequest {
    pub username: String,
    pub password: String,
}

impl From<LoginRequest> for ApiLoginRequest {
    fn from(req: LoginRequest) -> Self {
        Self {
            username: req.username,
            password: req.password,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MoveFileRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct ChangeVisibilityRequest {
    pub path: String,
    pub public: bool, // true for public, false for private
}

#[derive(Debug, Serialize)]
pub struct PermissionOperation {
    pub target_user_id: u64,
    pub relationship: String, // "owner", "editor", "viewer"
    pub action: String, // "grant" or "revoke"
}

#[derive(Debug, Serialize)]
pub struct FilePermissionRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>, // Optional - true for public, false for private
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionOperation>, // Optional - permissions object
}

pub enum ApiClient {
    Http {
        client: reqwest::Client,
        base_url: String,
        session_id: Option<String>,
    },
    Local {
        router: Arc<Router>,
        session_id: Option<String>,
    },
}

impl ApiClient {
    /// Create a new HTTP client for network requests
    pub fn new_http(base_url: String) -> Self {
        Self::Http {
            client: reqwest::Client::new(),
            base_url,
            session_id: None,
        }
    }

    /// Create a new local client that calls the router directly
    pub async fn new_local(db_pool: PgPool) -> Self {
        let (router, _state) = create_server(db_pool).await;
        Self::Local {
            router: Arc::new(router),
            session_id: None,
        }
    }

    /// Set the session ID for authenticated requests
    pub fn set_session(&mut self, session_id: String) {
        match self {
            ApiClient::Http {
                session_id: ref mut stored_session,
                ..
            } => {
                *stored_session = Some(session_id);
            }
            ApiClient::Local {
                session_id: ref mut stored_session,
                ..
            } => {
                *stored_session = Some(session_id);
            }
        }
    }

    /// Clear the stored session ID
    pub fn clear_session(&mut self) {
        match self {
            ApiClient::Http {
                session_id: ref mut stored_session,
                ..
            } => {
                *stored_session = None;
            }
            ApiClient::Local {
                session_id: ref mut stored_session,
                ..
            } => {
                *stored_session = None;
            }
        }
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest) -> Result<serde_json::Value, ApiError> {
        let api_request = ApiRegisterRequest::from(request);
        match self {
            ApiClient::Http {
                client, base_url, ..
            } => {
                let url = format!("{base_url}/auth/register");
                let response = client.post(&url).json(&api_request).send().await?;

                if response.status().is_success() {
                    let result = response.json::<serde_json::Value>().await?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, .. } => {
                let body = serde_json::to_string(&api_request)?;
                let request = Request::builder()
                    .method(Method::POST)
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let result: serde_json::Value = serde_json::from_slice(&body_bytes)?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Login with credentials
    pub async fn login(&self, request: LoginRequest) -> Result<serde_json::Value, ApiError> {
        let api_request = ApiLoginRequest::from(request);
        match self {
            ApiClient::Http {
                client, base_url, ..
            } => {
                let url = format!("{base_url}/auth/login");
                let response = client.post(&url).json(&api_request).send().await?;

                if response.status().is_success() {
                    let result = response.json::<serde_json::Value>().await?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, .. } => {
                let body = serde_json::to_string(&api_request)?;
                let request = Request::builder()
                    .method(Method::POST)
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let result: serde_json::Value = serde_json::from_slice(&body_bytes)?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Get current user info (requires session to be set)
    pub async fn me(&self) -> Result<serde_json::Value, ApiError> {
        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/auth/me");
                let response = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .send()
                    .await?;

                if response.status().is_success() {
                    let result = response.json::<serde_json::Value>().await?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let request = Request::builder()
                    .method(Method::GET)
                    .uri("/auth/me")
                    .header("Authorization", format!("Bearer {session}"))
                    .body(Body::empty())
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let result: serde_json::Value = serde_json::from_slice(&body_bytes)?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Logout (requires session to be set)
    pub async fn logout(&mut self) -> Result<serde_json::Value, ApiError> {
        let result = match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/auth/logout");
                let response = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .send()
                    .await?;

                if response.status().is_success() {
                    let result = response.json::<serde_json::Value>().await?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let request = Request::builder()
                    .method(Method::POST)
                    .uri("/auth/logout")
                    .header("Authorization", format!("Bearer {session}"))
                    .body(Body::empty())
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let result: serde_json::Value = serde_json::from_slice(&body_bytes)?;
                    Ok(result)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        };

        // Clear session after logout
        self.clear_session();
        result
    }

    /// Get file content as bytes (uses stored session if available)
    pub async fn get_file(
        &self,
        path: &str,
        for_user_id: Option<i64>,
    ) -> Result<Vec<u8>, ApiError> {
        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let url = if let Some(user_id) = for_user_id {
                    format!("{base_url}/files/{path}?u={user_id}")
                } else {
                    format!("{base_url}/files/{path}")
                };
                let mut request = client.get(&url);

                if let Some(session) = session_id {
                    request = request.header("Authorization", format!("Bearer {session}"));
                }

                let response = request.send().await?;

                if response.status().is_success() {
                    let bytes = response.bytes().await?;
                    Ok(bytes.to_vec())
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let url = if let Some(user_id) = for_user_id {
                    format!("/files/{path}?u={user_id}")
                } else {
                    format!("/files/{path}")
                };
                let mut request_builder = Request::builder().method(Method::GET).uri(url);

                if let Some(session) = session_id {
                    request_builder =
                        request_builder.header("Authorization", format!("Bearer {session}"));
                }

                let request = request_builder.body(Body::empty()).unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    Ok(body_bytes.to_vec())
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// List directory contents (uses stored session if available)
    pub async fn list_directory(
        &self,
        path: &str,
        for_user_id: Option<i64>,
    ) -> Result<Vec<SFile>, ApiError> {
        // Ensure path ends with / for directory listing
        let dir_path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };

        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let url = if let Some(user_id) = for_user_id {
                    format!("{base_url}/files/{dir_path}?u={user_id}")
                } else {
                    format!("{base_url}/files/{dir_path}")
                };
                let mut request = client.get(&url);

                if let Some(session) = session_id {
                    request = request.header("Authorization", format!("Bearer {session}"));
                }

                let response = request.send().await?;

                if response.status().is_success() {
                    let files = response.json::<Vec<SFile>>().await?;
                    Ok(files)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let url = if let Some(user_id) = for_user_id {
                    format!("/files/{dir_path}?u={user_id}")
                } else {
                    format!("/files/{dir_path}")
                };
                let mut request_builder = Request::builder().method(Method::GET).uri(url);

                if let Some(session) = session_id {
                    request_builder =
                        request_builder.header("Authorization", format!("Bearer {session}"));
                }

                let request = request_builder.body(Body::empty()).unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let files: Vec<SFile> = serde_json::from_slice(&body_bytes)?;
                    Ok(files)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Delete a file (requires session to be set)
    pub async fn delete_file(&self, path: &str) -> Result<(), ApiError> {
        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/files/{path}");
                let response = client
                    .delete(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .send()
                    .await?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let request = Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/files/{path}"))
                    .header("Authorization", format!("Bearer {session}"))
                    .body(Body::empty())
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Health check
    pub async fn health(&self) -> Result<(), ApiError> {
        match self {
            ApiClient::Http {
                client, base_url, ..
            } => {
                let url = format!("{base_url}/health");
                let response = client.get(&url).send().await?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, .. } => {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Ping endpoint
    pub async fn ping(&self) -> Result<String, ApiError> {
        match self {
            ApiClient::Http {
                client, base_url, ..
            } => {
                let url = format!("{base_url}/ping");
                let response = client.get(&url).send().await?;

                if response.status().is_success() {
                    let text = response.text().await?;
                    Ok(text)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, .. } => {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri("/ping")
                    .body(Body::empty())
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let text = String::from_utf8_lossy(&body_bytes).to_string();
                    Ok(text)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Upload a file (requires session to be set)
    pub async fn upload_file(
        &self,
        directory_path: &str,
        filename: &str,
        content: Vec<u8>,
    ) -> Result<Vec<SFile>, ApiError> {
        // Ensure directory path ends with /
        let dir_path = if directory_path.ends_with('/') {
            directory_path.to_string()
        } else {
            format!("{directory_path}/")
        };

        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/files/{dir_path}");

                let part = reqwest::multipart::Part::bytes(content)
                    .file_name(filename.to_string())
                    .mime_str("application/octet-stream")
                    .map_err(ApiError::Request)?;

                let form = reqwest::multipart::Form::new().part(filename.to_string(), part);

                let response = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .multipart(form)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let files = response.json::<Vec<SFile>>().await?;
                    Ok(files)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                // For local testing, we'll create a simple multipart body manually
                // This is more complex but allows testing without actual HTTP
                let boundary = "----ApiClientBoundary";
                let mut body_content = Vec::new();

                // Start boundary
                body_content.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
                body_content.extend_from_slice(format!("Content-Disposition: form-data; name=\"{filename}\"; filename=\"{filename}\"\r\n").as_bytes());
                body_content.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
                body_content.extend_from_slice(&content);
                body_content.extend_from_slice(b"\r\n");

                // End boundary
                body_content.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

                let request = Request::builder()
                    .method(Method::POST)
                    .uri(format!("/files/{dir_path}"))
                    .header("Authorization", format!("Bearer {session}"))
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body_content))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let files: Vec<SFile> = serde_json::from_slice(&body_bytes)?;
                    Ok(files)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Move/rename a file (requires session to be set)
    pub async fn move_file(&self, from_path: &str, to_path: &str) -> Result<SFile, ApiError> {
        let move_request = MoveFileRequest {
            from: from_path.to_string(),
            to: to_path.to_string(),
        };

        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/files");
                let response = client
                    .put(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .json(&move_request)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let file = response.json::<SFile>().await?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let body = serde_json::to_string(&move_request)?;
                let request = Request::builder()
                    .method(Method::PUT)
                    .uri("/files")
                    .header("Authorization", format!("Bearer {session}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let file: SFile = serde_json::from_slice(&body_bytes)?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Change file visibility (requires session to be set)
    pub async fn change_file_visibility(
        &self,
        path: &str,
        is_public: bool,
    ) -> Result<SFile, ApiError> {
        let visibility_request = ChangeVisibilityRequest {
            path: path.to_string(),
            public: is_public,
        };

        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/files");
                let response = client
                    .patch(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .json(&visibility_request)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let file = response.json::<SFile>().await?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let body = serde_json::to_string(&visibility_request)?;
                let request = Request::builder()
                    .method(Method::PATCH)
                    .uri("/files")
                    .header("Authorization", format!("Bearer {session}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let file: SFile = serde_json::from_slice(&body_bytes)?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Unified file operations - change visibility and/or permissions (requires session to be set)
    pub async fn set_permissions_and_visibility(
        &self,
        path: &str,
        public: Option<bool>,
        permissions: Option<PermissionOperation>,
    ) -> Result<SFile, ApiError> {
        let unified_request = FilePermissionRequest {
            path: path.to_string(),
            public,
            permissions,
        };

        match self {
            ApiClient::Http {
                client,
                base_url,
                session_id,
            } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let url = format!("{base_url}/files");
                let response = client
                    .patch(&url)
                    .header("Authorization", format!("Bearer {session}"))
                    .json(&unified_request)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let file = response.json::<SFile>().await?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    Err(ApiError::Http { status, body })
                }
            }
            ApiClient::Local { router, session_id } => {
                let session = session_id.as_ref().ok_or_else(|| ApiError::Http {
                    status: StatusCode::UNAUTHORIZED,
                    body: "No session set. Call set_session() first.".to_string(),
                })?;

                let body = serde_json::to_string(&unified_request)?;
                let request = Request::builder()
                    .method(Method::PATCH)
                    .uri("/files")
                    .header("Authorization", format!("Bearer {session}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap();

                let mut service = router.as_ref().clone();
                let response = Service::<Request<Body>>::call(&mut service, request)
                    .await
                    .map_err(|e| {
                        ApiError::Service(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    })?;

                if response.status().is_success() {
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let file: SFile = serde_json::from_slice(&body_bytes)?;
                    Ok(file)
                } else {
                    let status = response.status();
                    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();
                    Err(ApiError::Http { status, body })
                }
            }
        }
    }

    /// Grant permissions to a user on a file (requires session to be set)
    pub async fn grant_file_permission(
        &self,
        path: &str,
        target_user_id: u64,
        relationship: &str,
    ) -> Result<SFile, ApiError> {
        self.set_permissions_and_visibility(
            path,
            None,
            Some(PermissionOperation {
                target_user_id,
                relationship: relationship.to_string(),
                action: "grant".to_string(),
            }),
        )
        .await
    }

    /// Revoke permissions from a user on a file (requires session to be set)
    pub async fn revoke_file_permission(
        &self,
        path: &str,
        target_user_id: u64,
        relationship: &str,
    ) -> Result<SFile, ApiError> {
        self.set_permissions_and_visibility(
            path,
            None,
            Some(PermissionOperation {
                target_user_id,
                relationship: relationship.to_string(),
                action: "revoke".to_string(),
            }),
        )
        .await
    }
}
