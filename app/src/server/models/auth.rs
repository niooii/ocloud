use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String, // PHC format
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_active: bool,
    pub last_login: Option<NaiveDateTime>,
}

impl User {
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.and_utc()
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.and_utc()
    }

    pub fn last_login_utc(&self) -> Option<DateTime<Utc>> {
        self.last_login.map(|dt| dt.and_utc())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Resource {
    pub id: i64,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "relationship_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RelationshipType {
    Owner,
    Editor,
    Viewer,
    // TODO! this isn't really implemented or done anything with anywhere
    None
}

impl RelationshipType {
    /// Check if this relationship can perform the given action
    pub fn can_perform(&self, action: &Permission) -> bool {
        match (self, action) {
            // Owner permissions
            (RelationshipType::Owner, Permission::Read) => true,
            (RelationshipType::Owner, Permission::Write) => true,
            (RelationshipType::Owner, Permission::Delete) => true,
            (RelationshipType::Owner, Permission::Share) => true,
            (RelationshipType::Owner, Permission::ChangePermissions) => true,

            // Editor permissions
            (RelationshipType::Editor, Permission::Read) => true,
            (RelationshipType::Editor, Permission::Write) => true,
            (RelationshipType::Editor, Permission::Delete) => false,
            (RelationshipType::Editor, Permission::Share) => false,
            (RelationshipType::Editor, Permission::ChangePermissions) => false,

            // Viewer permissions
            (RelationshipType::Viewer, Permission::Read) => true,
            (RelationshipType::Viewer, _) => false,
            (RelationshipType::None, _) => false
        }
    }

    /// Get all permissions this relationship grants
    pub fn permissions(&self) -> HashSet<Permission> {
        let mut perms = HashSet::new();
        for perm in [
            Permission::Read,
            Permission::Write,
            Permission::Delete,
            Permission::Share,
            Permission::ChangePermissions,
        ] {
            if self.can_perform(&perm) {
                perms.insert(perm);
            }
        }
        perms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Write,
    Delete,
    Share,
    ChangePermissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Grant,
    Revoke,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserResourceRelationship {
    pub id: i64,
    pub user_id: i64,
    pub resource_id: i64,
    pub relationship: RelationshipType,
    pub granted_by: Option<i64>,
    pub granted_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

impl UserResourceRelationship {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < chrono::Utc::now().naive_utc()
        } else {
            false
        }
    }

    pub fn granted_at_utc(&self) -> DateTime<Utc> {
        self.granted_at.and_utc()
    }

    pub fn expires_at_utc(&self) -> Option<DateTime<Utc>> {
        self.expires_at.map(|dt| dt.and_utc())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: i64,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub last_accessed: NaiveDateTime,
}

impl UserSession {
    pub fn is_expired(&self) -> bool {
        self.expires_at < chrono::Utc::now().naive_utc()
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.and_utc()
    }

    pub fn expires_at_utc(&self) -> DateTime<Utc> {
        self.expires_at.and_utc()
    }

    pub fn last_accessed_utc(&self) -> DateTime<Utc> {
        self.last_accessed.and_utc()
    }
}

// DTOs for API requests/responses
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        let created_at = user.created_at_utc();
        let last_login = user.last_login_utc();
        Self {
            id: user.id as u64,
            username: user.username,
            email: user.email,
            created_at,
            last_login,
        }
    }
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            id: user.id as u64,
            username: user.username.clone(),
            email: user.email.clone(),
            created_at: user.created_at_utc(),
            last_login: user.last_login_utc(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GrantPermissionRequest {
    pub target_user_id: u64,
    pub resource_type: String,
    pub resource_id: Option<u64>,
    pub relationship: RelationshipType,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RevokePermissionRequest {
    pub target_user_id: u64,
    pub resource_type: String,
    pub resource_id: Option<u64>,
    pub relationship: RelationshipType,
}

#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    pub user: UserInfo,
    pub relationship: RelationshipType,
    pub granted_by: Option<u64>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ReBAC context for checking permissions
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: i64,
    pub username: String,
    pub permissions: HashSet<(String, Option<i64>, RelationshipType)>, // (resource_type, resource_id, relationship)
}

impl AuthContext {
    pub fn new(user_id: i64, username: String) -> Self {
        Self {
            user_id,
            username,
            permissions: HashSet::new(),
        }
    }

    pub fn has_permission(
        &self,
        resource_type: &str,
        resource_id: Option<i64>,
        required_permission: Permission,
    ) -> bool {
        // Check if user has any relationship that grants the required permission
        for (res_type, res_id, relationship) in &self.permissions {
            if res_type == resource_type
                && *res_id == resource_id
                && relationship.can_perform(&required_permission)
            {
                return true;
            }
        }

        false
    }

    pub fn add_permission(
        &mut self,
        resource_type: String,
        resource_id: Option<i64>,
        relationship: RelationshipType,
    ) {
        self.permissions
            .insert((resource_type, resource_id, relationship));
    }
}

// Password utils
pub mod password {
    use crate::server::error::{ServerError, ServerResult};
    use argon2::{
        password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
        Argon2,
    };
    use rand_core::OsRng;

    /// Hash a password using Argon2id (PHC format)
    pub async fn hash_password(password: String) -> ServerResult<String> {
        // spawn_blocking to avoid blocking the async executor
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            argon2
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| ServerError::InternalError {
                    message: format!("Failed to hash password: {e}"),
                })
                .map(|hash| hash.to_string())
        })
        .await
        .map_err(|e| ServerError::InternalError {
            message: format!("Task join error: {e}"),
        })?
    }

    /// Verify a password against its hash
    pub async fn verify_password(password: String, hash: String) -> ServerResult<bool> {
        // Use spawn_blocking to avoid blocking the async executor
        tokio::task::spawn_blocking(move || {
            let parsed_hash = PasswordHash::new(&hash).map_err(|e| ServerError::InternalError {
                message: format!("Invalid password hash: {e}"),
            })?;

            let argon2 = Argon2::default();

            Ok(argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok())
        })
        .await
        .map_err(|e| ServerError::InternalError {
            message: format!("Task join error: {e}"),
        })?
    }
}
