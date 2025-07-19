use axum::{
    routing::{get, post},
    Router, Json, Extension,
    response::Json as ResponseJson,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::server::{
    controllers::auth::AuthController,
    models::auth::*,
    error::ServerError,
    web::middleware::require_auth,
};

pub fn routes(auth_controller: AuthController) -> Router {
    let public_routes = Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler));
    
    let protected_routes = Router::new()
        .route("/auth/logout", post(logout_handler))
        .route("/auth/me", get(me_handler))
        .route("/auth/permissions/grant", post(grant_permission_handler))
        .route("/auth/permissions/revoke", post(revoke_permission_handler))
        .route("/auth/permissions/:resource_type", get(get_permissions_handler))
        .route("/auth/permissions/:resource_type/:resource_id", get(get_permissions_with_id_handler))
        .layer(axum::middleware::from_fn(require_auth));
    
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
}

async fn register_handler(
    Extension(auth_controller): Extension<AuthController>,
    Json(request): Json<RegisterRequest>,
) -> Result<ResponseJson<Value>, ServerError> {
    let user = auth_controller.register_user(request).await?;
    
    Ok(ResponseJson(json!({
        "user": UserInfo::from(user),
        "message": "User registered successfully"
    })))
}

async fn login_handler(
    Extension(auth_controller): Extension<AuthController>,
    Json(request): Json<LoginRequest>,
) -> Result<ResponseJson<Value>, ServerError> {
    let (user, session) = auth_controller.login(request).await?;
    
    Ok(ResponseJson(json!({
        "user": UserInfo::from(user),
        "session_id": session.id.to_string(),
        "expires_at": session.expires_at_utc(),
        "message": "Login successful"
    })))
}

async fn logout_handler(
    Extension(auth_controller): Extension<AuthController>,
    Extension(session_id): Extension<Uuid>,
) -> Result<ResponseJson<Value>, ServerError> {
    auth_controller.delete_session(session_id).await?;
    
    Ok(ResponseJson(json!({
        "message": "Logout successful"
    })))
}

async fn me_handler(
    Extension(auth_context): Extension<AuthContext>,
) -> Result<ResponseJson<Value>, ServerError> {
    Ok(ResponseJson(json!({
        "user_id": auth_context.user_id,
        "username": auth_context.username,
        "permissions": auth_context.permissions.len()
    })))
}

async fn grant_permission_handler(
    Extension(auth_controller): Extension<AuthController>,
    Extension(auth_context): Extension<AuthContext>,
    Json(request): Json<GrantPermissionRequest>,
) -> Result<ResponseJson<Value>, ServerError> {
    auth_controller.grant_permission(auth_context.user_id, request).await?;
    
    Ok(ResponseJson(json!({
        "message": "Permission granted successfully"
    })))
}

async fn revoke_permission_handler(
    Extension(auth_controller): Extension<AuthController>,
    Extension(auth_context): Extension<AuthContext>,
    Json(request): Json<RevokePermissionRequest>,
) -> Result<ResponseJson<Value>, ServerError> {
    auth_controller.revoke_permission(auth_context.user_id, request).await?;
    
    Ok(ResponseJson(json!({
        "message": "Permission revoked successfully"
    })))
}

async fn get_permissions_handler(
    Extension(auth_controller): Extension<AuthController>,
    Extension(auth_context): Extension<AuthContext>,
    axum::extract::Path(resource_type): axum::extract::Path<String>,
) -> Result<ResponseJson<Value>, ServerError> {
    let resource_id = None; // For now, we'll handle simple case
    
    // Check if user has permission to view permissions
    if !auth_context.has_permission(&resource_type, resource_id, Permission::Read) {
        return Err(ServerError::AuthorizationError {
            message: "Insufficient permissions to view resource permissions".to_string(),
        });
    }
    
    let permissions = auth_controller.get_resource_permissions(&resource_type, resource_id).await?;
    
    Ok(ResponseJson(json!({
        "resource_type": resource_type,
        "resource_id": resource_id,
        "permissions": permissions
    })))
}

async fn get_permissions_with_id_handler(
    Extension(auth_controller): Extension<AuthController>,
    Extension(auth_context): Extension<AuthContext>,
    axum::extract::Path((resource_type, resource_id)): axum::extract::Path<(String, u64)>,
) -> Result<ResponseJson<Value>, ServerError> {
    // Check if user has permission to view permissions
    if !auth_context.has_permission(&resource_type, Some(resource_id as i64), Permission::Read) {
        return Err(ServerError::AuthorizationError {
            message: "Insufficient permissions to view resource permissions".to_string(),
        });
    }
    
    let permissions = auth_controller.get_resource_permissions(&resource_type, Some(resource_id as i64)).await?;
    
    Ok(ResponseJson(json!({
        "resource_type": resource_type,
        "resource_id": resource_id,
        "permissions": permissions
    })))
}