use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{Response, IntoResponse};
use std::time::Instant;
use std::collections::HashSet;
// Temporarily disabled rate limiting due to complex configuration
// use tower_governor::{
//     governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
// };
use tracing::info_span;
use uuid::Uuid;

use crate::server::{
    controllers::auth::AuthController,
    error::ServerError,
};

pub async fn trace_request(mut request: Request, next: Next) -> Response {
    let start = Instant::now();
    
    let request_id = Uuid::new_v4().to_string();
    
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();
    
    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid"))
    );

    let span = info_span!(
        "request",
        method = %method,
        uri = %uri,
        version = ?version,
        request_id = %request_id,
    );

    let _enter = span.enter();

    let response = next.run(request).await;
    
    let latency = start.elapsed();
    let status = response.status();
    
    tracing::info!(
        latency_ms = latency.as_millis(),
        status = %status,
        "Request completed"
    );

    response
}

// Temporarily disabled rate limiting due to complex configuration
// pub fn rate_limiting_layer() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
//     let governor_conf = GovernorConfigBuilder::default()
//         .per_second(30)
//         .burst_size(60)
//         .finish()
//         .unwrap();

//     GovernorLayer {
//         config: std::sync::Arc::new(governor_conf),
//     }
// }

/// Middleware for requiring authentication
#[derive(Clone)]
pub struct RequireAuth {
    excluded_paths: HashSet<String>,
}

impl Default for RequireAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl RequireAuth {
    pub fn new() -> Self {
        Self {
            excluded_paths: HashSet::new(),
        }
    }

    pub fn exclude_paths(mut self, paths: Vec<String>) -> Self {
        self.excluded_paths = paths.into_iter().collect();
        self
    }

    pub async fn middleware(
        self,
        mut request: Request,
        next: Next,
    ) -> Result<Response, impl IntoResponse> {
        let path = request.uri().path();

        // Skip authentication for excluded paths
        if self.excluded_paths.contains(path) {
            return Ok(next.run(request).await);
        }

        // Get auth controller from extensions
        let auth_controller = match request.extensions().get::<AuthController>() {
            Some(controller) => controller.clone(),
            None => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Auth controller not available"
                ).into_response());
            }
        };

        // Extract session ID from Authorization header
        let session_id = match extract_session_id(&request) {
            Some(id) => id,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Missing or invalid authorization header"
                ).into_response());
            }
        };

        // Validate session
        let (user, _session) = match auth_controller.validate_session(session_id).await {
            Ok(result) => result,
            Err(ServerError::AuthenticationError { .. }) => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired session"
                ).into_response());
            }
            Err(e) => {
                tracing::error!("Auth validation error: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authentication service error"
                ).into_response());
            }
        };

        // Build auth context
        let auth_context = match auth_controller.build_auth_context(user.id).await {
            Ok(context) => context,
            Err(e) => {
                tracing::error!("Failed to build auth context: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load user permissions"
                ).into_response());
            }
        };

        // Add auth context and session ID to request extensions
        request.extensions_mut().insert(auth_context);
        request.extensions_mut().insert(session_id);

        Ok(next.run(request).await)
    }
}

/// Extract session ID from Authorization header
/// Expected format: "Bearer <session_id>"
fn extract_session_id(request: &Request) -> Option<Uuid> {
    let auth_header = request
        .headers()
        .get("authorization")?
        .to_str()
        .ok()?;

    if !auth_header.starts_with("Bearer ") {
        return None;
    }

    let token = auth_header.strip_prefix("Bearer ")?;
    Uuid::parse_str(token).ok()
}

/// Session-based authentication middleware function
/// Middleware that optionally extracts auth context without requiring it
pub async fn optional_auth(mut request: Request, next: Next) -> Response {
    // Get auth controller from extensions
    if let Some(auth_controller) = request.extensions().get::<AuthController>() {
        let auth_controller = auth_controller.clone();
        
        // Try to extract authorization header
        if let Some(auth_header) = request.headers().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if let Ok(session_uuid) = token.parse::<Uuid>() {
                        // Try to validate session and build auth context
                        if let Ok((user, _session)) = auth_controller.validate_session(session_uuid).await {
                            if let Ok(auth_context) = auth_controller.build_auth_context(user.id).await {
                                // Add auth context to request extensions
                                request.extensions_mut().insert(auth_context);
                            }
                        }
                    }
                }
            }
        }
    }
    
    next.run(request).await
}

pub async fn require_auth(
    mut request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = request.uri().path();

    // Skip authentication for excluded paths
    if path == "/auth/register" || path == "/auth/login" {
        return Ok(next.run(request).await);
    }
    // Get auth controller from extensions
    let auth_controller = match request.extensions().get::<AuthController>() {
        Some(controller) => controller.clone(),
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auth controller not available"
            ).into_response());
        }
    };

    // Extract session ID from Authorization header
    let session_id = match extract_session_id(&request) {
        Some(id) => id,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing or invalid authorization header"
            ).into_response());
        }
    };

    // Validate session
    let (user, _session) = match auth_controller.validate_session(session_id).await {
        Ok(result) => result,
        Err(ServerError::AuthenticationError { .. }) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid or expired session"
            ).into_response());
        }
        Err(e) => {
            tracing::error!("Auth validation error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authentication service error"
            ).into_response());
        }
    };

    // Build auth context
    let auth_context = match auth_controller.build_auth_context(user.id).await {
        Ok(context) => context,
        Err(e) => {
            tracing::error!("Failed to build auth context: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load user permissions"
            ).into_response());
        }
    };

    // Add auth context and session ID to request extensions
    request.extensions_mut().insert(auth_context);
    request.extensions_mut().insert(session_id);

    Ok(next.run(request).await)
}