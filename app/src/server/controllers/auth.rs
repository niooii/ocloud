use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::server::{
    error::{ServerError, ServerResult},
    models::auth::*,
};

#[derive(Clone)]
pub struct AuthController {
    db: PgPool,
}

impl AuthController {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Register a new user
    pub async fn register_user(&self, request: RegisterRequest) -> ServerResult<User> {
        // Check if username or email already exists
        let existing = sqlx::query!(
            "SELECT id FROM users WHERE username = $1 OR email = $2",
            request.username,
            request.email
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to check existing user: {e}"),
        })?;

        if existing.is_some() {
            return Err(ServerError::ValidationError {
                message: "Username or email already exists".to_string(),
            });
        }

        // Hash the password
        let password_hash = password::hash_password(request.password).await?;

        // Insert new user
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, email, password_hash, created_at, updated_at, is_active)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, true)
            RETURNING id, username, email, password_hash, created_at, updated_at, is_active, last_login
            "#,
            request.username,
            request.email,
            password_hash
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to create user: {e}"),
        })?;

        Ok(user)
    }

    /// Authenticate a user and create a session
    pub async fn login(&self, request: LoginRequest) -> ServerResult<(User, UserSession)> {
        // Find user by username or email
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, email, password_hash, created_at, updated_at, is_active, last_login 
             FROM users WHERE (username = $1 OR email = $1) AND is_active = true",
            request.username
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find user: {e}"),
        })?
        .ok_or_else(|| ServerError::AuthenticationError {
            message: "Invalid credentials".to_string(),
        })?;

        // Verify password (constant-time comparison as per book.md)
        let password_valid = password::verify_password(request.password, user.password_hash.clone()).await?;
        
        if !password_valid {
            // Still do some work to prevent timing attacks (as per book.md)
            let _ = password::verify_password("dummy".to_string(), user.password_hash.clone()).await;
            return Err(ServerError::AuthenticationError {
                message: "Invalid credentials".to_string(),
            });
        }

        // Update last login
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users 
            SET last_login = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING id, username, email, password_hash, created_at, updated_at, is_active, last_login
            "#,
            user.id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to update last login: {e}"),
        })?;

        // Create session (expires in 24 hours)
        let session_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::hours(24);

        let session = sqlx::query_as!(
            UserSession,
            r#"
            INSERT INTO user_sessions (id, user_id, created_at, expires_at, last_accessed)
            VALUES ($1, $2, CURRENT_TIMESTAMP, $3, CURRENT_TIMESTAMP)
            RETURNING id, user_id, created_at, expires_at, last_accessed
            "#,
            session_id,
            user.id,
            expires_at.naive_utc()
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to create session: {e}"),
        })?;

        Ok((user, session))
    }

    /// Validate a session and return the user
    pub async fn validate_session(&self, session_id: Uuid) -> ServerResult<(User, UserSession)> {
        let session = sqlx::query_as!(
            UserSession,
            "SELECT id, user_id, created_at, expires_at, last_accessed 
             FROM user_sessions WHERE id = $1",
            session_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find session: {e}"),
        })?
        .ok_or_else(|| ServerError::AuthenticationError {
            message: "Invalid session".to_string(),
        })?;

        if session.is_expired() {
            // Delete expired session
            self.delete_session(session_id).await?;
            return Err(ServerError::AuthenticationError {
                message: "Session expired".to_string(),
            });
        }

        // Update last accessed
        let session = sqlx::query_as!(
            UserSession,
            r#"
            UPDATE user_sessions 
            SET last_accessed = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING id, user_id, created_at, expires_at, last_accessed
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to update session: {e}"),
        })?;

        // Get user
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, email, password_hash, created_at, updated_at, is_active, last_login 
             FROM users WHERE id = $1 AND is_active = true",
            session.user_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find user: {e}"),
        })?
        .ok_or_else(|| ServerError::AuthenticationError {
            message: "User not found".to_string(),
        })?;

        Ok((user, session))
    }

    /// Delete a session (logout)
    pub async fn delete_session(&self, session_id: Uuid) -> ServerResult<()> {
        sqlx::query!("DELETE FROM user_sessions WHERE id = $1", session_id)
            .execute(&self.db)
            .await
            .map_err(|e| ServerError::DatabaseError {
                message: format!("Failed to delete session: {e}"),
            })?;

        Ok(())
    }

    /// Build auth context for a user (load all their permissions)
    pub async fn build_auth_context(&self, user_id: i64) -> ServerResult<AuthContext> {
        let user = sqlx::query!(
            "SELECT username FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find user: {e}"),
        })?
        .ok_or_else(|| ServerError::AuthenticationError {
            message: "User not found".to_string(),
        })?;

        let mut context = AuthContext::new(user_id, user.username);

        // Load user's relationships
        let relationships = sqlx::query!(
            r#"
            SELECT r.resource_type, r.resource_id, urr.relationship as "relationship: RelationshipType"
            FROM user_resource_relationships urr
            JOIN resources r ON urr.resource_id = r.id
            WHERE urr.user_id = $1 
            AND (urr.expires_at IS NULL OR urr.expires_at > CURRENT_TIMESTAMP)
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to load user relationships: {e}"),
        })?;

        for rel in relationships {
            context.add_permission(rel.resource_type, rel.resource_id, rel.relationship);
        }

        Ok(context)
    }

    /// Grant permission to a user on a resource
    pub async fn grant_permission(&self, granter_id: i64, request: GrantPermissionRequest) -> ServerResult<()> {
        // Verify granter has permission to grant this
        let granter_context = self.build_auth_context(granter_id).await?;
        
        if !granter_context.has_permission(&request.resource_type, request.resource_id.map(|id| id as i64), Permission::ChangePermissions) {
            return Err(ServerError::AuthorizationError {
                message: "Insufficient permissions to grant access".to_string(),
            });
        }

        // Find or create resource
        let resource = self.get_or_create_resource(&request.resource_type, request.resource_id.map(|id| id as i64)).await?;

        // Check if relationship already exists
        let existing = sqlx::query!(
            "SELECT id FROM user_resource_relationships 
             WHERE user_id = $1 AND resource_id = $2 AND relationship = $3",
            request.target_user_id as i64,
            resource.id,
            request.relationship as RelationshipType
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to check existing relationship: {e}"),
        })?;

        if existing.is_some() {
            return Err(ServerError::ValidationError {
                message: "Relationship already exists".to_string(),
            });
        }

        // Grant permission
        sqlx::query!(
            r#"
            INSERT INTO user_resource_relationships (user_id, resource_id, relationship, granted_by, granted_at, expires_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, $5)
            "#,
            request.target_user_id as i64,
            resource.id,
            request.relationship as RelationshipType,
            granter_id,
            request.expires_at.map(|dt| dt.naive_utc())
        )
        .execute(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to grant permission: {e}"),
        })?;

        Ok(())
    }

    /// Revoke permission from a user on a resource
    pub async fn revoke_permission(&self, revoker_id: i64, request: RevokePermissionRequest) -> ServerResult<()> {
        // Verify revoker has permission to revoke this
        let revoker_context = self.build_auth_context(revoker_id).await?;
        
        if !revoker_context.has_permission(&request.resource_type, request.resource_id.map(|id| id as i64), Permission::ChangePermissions) {
            return Err(ServerError::AuthorizationError {
                message: "Insufficient permissions to revoke access".to_string(),
            });
        }

        // Find resource
        let resource = sqlx::query_as!(
            Resource,
            "SELECT id, resource_type, resource_id, created_at FROM resources 
             WHERE resource_type = $1 AND resource_id = $2",
            request.resource_type,
            request.resource_id.map(|id| id as i64)
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find resource: {e}"),
        })?
        .ok_or_else(|| ServerError::ValidationError {
            message: "Resource not found".to_string(),
        })?;

        // Revoke permission
        let rows_affected = sqlx::query!(
            "DELETE FROM user_resource_relationships 
             WHERE user_id = $1 AND resource_id = $2 AND relationship = $3",
            request.target_user_id as i64,
            resource.id,
            request.relationship as RelationshipType
        )
        .execute(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to revoke permission: {e}"),
        })?
        .rows_affected();

        if rows_affected == 0 {
            return Err(ServerError::ValidationError {
                message: "Permission not found".to_string(),
            });
        }

        Ok(())
    }

    /// Get or create a resource entry
    async fn get_or_create_resource(&self, resource_type: &str, resource_id: Option<i64>) -> ServerResult<Resource> {
        // Try to find existing resource
        if let Some(resource) = sqlx::query_as!(
            Resource,
            "SELECT id, resource_type, resource_id, created_at FROM resources 
             WHERE resource_type = $1 AND resource_id = $2",
            resource_type,
            resource_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find resource: {e}"),
        })? {
            return Ok(resource);
        }

        // Create new resource
        let resource = sqlx::query_as!(
            Resource,
            r#"
            INSERT INTO resources (resource_type, resource_id, created_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            RETURNING id, resource_type, resource_id, created_at
            "#,
            resource_type,
            resource_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to create resource: {e}"),
        })?;

        Ok(resource)
    }

    /// Get permissions for a resource
    pub async fn get_resource_permissions(&self, resource_type: &str, resource_id: Option<i64>) -> ServerResult<Vec<PermissionInfo>> {
        let resource = sqlx::query_as!(
            Resource,
            "SELECT id, resource_type, resource_id, created_at FROM resources 
             WHERE resource_type = $1 AND resource_id = $2",
            resource_type,
            resource_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to find resource: {e}"),
        })?
        .ok_or_else(|| ServerError::ValidationError {
            message: "Resource not found".to_string(),
        })?;

        let permissions = sqlx::query!(
            r#"
            SELECT 
                u.id as user_id, u.username, u.email, u.created_at as user_created_at, u.last_login,
                urr.relationship as "relationship: RelationshipType",
                urr.granted_by, urr.granted_at, urr.expires_at
            FROM user_resource_relationships urr
            JOIN users u ON urr.user_id = u.id
            WHERE urr.resource_id = $1 
            AND (urr.expires_at IS NULL OR urr.expires_at > CURRENT_TIMESTAMP)
            ORDER BY urr.granted_at DESC
            "#,
            resource.id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| ServerError::DatabaseError {
            message: format!("Failed to load resource permissions: {e}"),
        })?;

        let permission_infos = permissions
            .into_iter()
            .map(|row| PermissionInfo {
                user: UserInfo {
                    id: row.user_id as u64,
                    username: row.username,
                    email: row.email,
                    created_at: row.user_created_at.and_utc(),
                    last_login: row.last_login.map(|dt| dt.and_utc()),
                },
                relationship: row.relationship,
                granted_by: row.granted_by.map(|id| id as u64),
                granted_at: row.granted_at.and_utc(),
                expires_at: row.expires_at.map(|dt| dt.and_utc()),
            })
            .collect();

        Ok(permission_infos)
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> ServerResult<u64> {
        let result = sqlx::query!("DELETE FROM user_sessions WHERE expires_at < CURRENT_TIMESTAMP")
            .execute(&self.db)
            .await
            .map_err(|e| ServerError::DatabaseError {
                message: format!("Failed to cleanup sessions: {e}"),
            })?;

        Ok(result.rows_affected())
    }
}