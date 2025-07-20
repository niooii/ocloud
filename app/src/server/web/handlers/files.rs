use crate::{config::SETTINGS, server::models::files::SFile};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderValue},
    response::Response,
    routing::{delete, get, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::error;

use crate::server::error::{ServerError, ServerResult};
use crate::server::web::middleware::{optional_auth, require_auth};
use crate::server::{
    controllers::files::{FileController, FileVisibility},
    models::auth::{AuthContext, Permission},
    models::files::{FileUploadInfo, Media, VirtualPath},
};
use sqlx::query;

pub fn routes(controller: FileController) -> Router {
    // Public routes (no authentication required - handlers check if files are public)
    let public_routes = Router::new()
        .route("/files/*path", get(get_file_or_list_dir))
        .layer(axum::middleware::from_fn(optional_auth));

    // Protected routes (require authentication)
    let protected_routes = Router::new()
        .route(
            "/files/*path",
            delete(delete_file).post(upload_or_mk_dirs).layer(
                if let Some(s) = SETTINGS.application.max_filesize {
                    DefaultBodyLimit::max(s)
                } else {
                    DefaultBodyLimit::disable()
                },
            ),
        )
        .route("/files", put(move_files).patch(change_file_visibility))
        .layer(axum::middleware::from_fn(require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(controller)
}

#[derive(Deserialize)]
pub struct MoveInfo {
    pub from: VirtualPath,
    pub to: VirtualPath,
}

#[derive(Deserialize)]
pub struct VisibilityInfo {
    pub path: VirtualPath,
    pub visibility: String, // "public" or "private"
}

#[derive(Deserialize)]
pub struct UserQuery {
    pub u: Option<i64>, // Optional user ID to access other user's files
}

pub async fn move_files(
    Extension(auth_context): Extension<AuthContext>,
    State(files): State<FileController>,
    Json(move_info): Json<MoveInfo>,
) -> ServerResult<Json<SFile>> {
    // Check if user has permission to move the source file
    let sfile = files
        .get_sfile(&move_info.from, auth_context.user_id)
        .await?;

    // First check if user is the owner (direct ownership via sfiles.user_id)
    let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile.id as i64)
        .fetch_optional(files.db_pool())
        .await?
        .map(|row| row.user_id == Some(auth_context.user_id))
        .unwrap_or(false);

    // If not owner, check ReBAC permissions
    if !is_owner
        && !auth_context.has_permission(
            "sfile",
            Some(sfile.id as i64),
            Permission::ChangePermissions,
        )
    {
        return Err(ServerError::AuthorizationError {
            message: "Only file owners can move files".to_string(),
        });
    }

    files
        .mv(&move_info.from, &move_info.to, auth_context.user_id)
        .await
        .map(Json)
}

pub async fn upload_or_mk_dirs(
    Extension(auth_context): Extension<AuthContext>,
    State(files): State<FileController>,
    Path(mut path): Path<VirtualPath>,
    multipart: Option<Multipart>,
) -> ServerResult<Json<Vec<SFile>>> {
    path.err_if_file()?;
    // If it was multipart
    if let Some(mut multipart) = multipart {
        if let Some(mut field) =
            multipart
                .next_field()
                .await
                .map_err(|e| ServerError::AxumError {
                    message: format!("Multipart error: {}", e.body_text()),
                })?
        {
            let save_dir = &SETTINGS.directories.files_dir;

            // Name should be the name of the file, including the extension.
            let name: String = field.name().expect("File has no name??").to_string();
            // trace!("Got file: {name}");
            // trace!("for path: {}", path.to_string());

            if path.to_string_with_trailing().is_empty() {
                path = VirtualPath::root();
            }

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let mut hasher = Sha256::new();

            let temp_path: PathBuf = save_dir.join(format!("./tmp_{now}_{name}"));

            let mut file: File =
                File::create(&temp_path)
                    .await
                    .map_err(|e| ServerError::IOError {
                        message: e.to_string(),
                    })?;
            // i64 type because postgres doesnt support unsigned gg

            let mut file_size: i64 = 0;
            const PROGRESS_THRESHOLD: u64 = 1024 * 1024; // 1MB
            let mut last_progress_report: u64 = 0;

            while let Some(chunk) = field.chunk().await.map_err(|e| ServerError::AxumError {
                message: format!("Chunk error: {}", e.body_text()),
            })? {
                file.write_all(&chunk)
                    .await
                    .map_err(|e| ServerError::IOError {
                        message: e.to_string(),
                    })?;
                file_size += chunk.len() as i64;
                hasher.write_all(&chunk).expect("Failed to hash shit");

                // Send progress updates every 1MB or so
                if file_size as u64 - last_progress_report >= PROGRESS_THRESHOLD {
                    // Note: We don't have total size available with multipart uploads in axum
                    // So we'll just report current bytes uploaded without percentage
                    last_progress_report = file_size as u64;
                }
            }

            file.flush().await.expect("Bluh flushing file failed");

            let hash = hasher.finalize();
            let file_hash: String = format!("{hash:X}");

            let info = FileUploadInfo {
                file_name: name,
                temp_path: temp_path.clone(),
                file_size,
                file_hash: file_hash.clone(),
                vpath: path,
                user_id: auth_context.user_id,
            };

            // Ensure the file handle is dropped before doing anything
            // ahem windows
            drop(file);

            // HEHEHEHAW fix race condition
            // just in case if two people upload the same file at the exact same time down to the millisecond...??
            let mutex = files.active_uploads.lock(file_hash.clone()).await;

            // Check-in file to database
            let sfile = match files.finish_upload(info).await {
                Err(e) => {
                    // doesnt really have to be checked
                    let _ = fs::remove_file(&temp_path).await;
                    error!(
                        "(Tried) removed {} due to error: {e:?}",
                        temp_path.to_string_lossy()
                    );
                    Err(e)
                }
                Ok(c) => {
                    // Notify upload completion via WebSocket if available
                    // This is handled in the FileController's finish_upload method
                    Ok(c)
                }
            };

            drop(mutex);

            return sfile.map(|s| Json(vec![s]));
        }
    }

    // either no content in multipart or not multipart. thats okay, just make the directory.
    files
        .make_all_dirs(&path, auth_context.user_id, None)
        .await
        .map(Json)
}

pub async fn get_file_or_list_dir(
    auth_context: Option<Extension<AuthContext>>,
    Path(path): Path<VirtualPath>,
    Query(user_query): Query<UserQuery>,
    State(files): State<FileController>,
) -> ServerResult<Response> {
    // Determine target user ID based on query parameter or authenticated user
    let target_user_id = match user_query.u {
        Some(requested_user_id) => {
            // User wants to access someone else's files
            match auth_context.as_ref() {
                Some(Extension(ctx)) => {
                    // For now, allow access if authenticated (permission checks can be added later)
                    requested_user_id
                }
                None => {
                    // Anonymous user trying to access specific user's files - only allow for public files
                    requested_user_id
                }
            }
        }
        None => {
            // No specific user requested, use authenticated user's files or reject if not authenticated
            match auth_context.as_ref() {
                Some(Extension(ctx)) => ctx.user_id,
                None => {
                    return Err(ServerError::AuthenticationError {
                        message: "Authentication required when no target user specified"
                            .to_string(),
                    });
                }
            }
        }
    };

    if !path.is_dir() {
        let sfile = files.get_sfile(&path, target_user_id).await?;

        // Check if file is public - if so, allow access regardless of authentication
        if !sfile.is_public {
            // File is private, require authentication and permission checking
            let auth_context = match auth_context {
                Some(Extension(ctx)) => ctx,
                None => {
                    return Err(ServerError::AuthenticationError {
                        message: "Authentication required to access private files".to_string(),
                    })
                }
            };

            // First check if user is the owner (direct ownership via sfiles.user_id)
            let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile.id as i64)
                .fetch_optional(files.db_pool())
                .await?
                .map(|row| row.user_id == Some(auth_context.user_id))
                .unwrap_or(false);

            // If not owner, check ReBAC permissions
            if !is_owner
                && !auth_context.has_permission("sfile", Some(sfile.id as i64), Permission::Read)
            {
                return Err(ServerError::AuthorizationError {
                    message: "You don't have permission to access this file".to_string(),
                });
            }
        }

        let media: Media = files.get_media(&path, target_user_id).await?;

        let stream = media.reader_stream().await?;
        let body = Body::from_stream(stream);
        let mut res = Response::new(body);

        // error should be propogated from the storage.get_media call,
        // since there it has a directory or not check.
        let file_name = path.file_name().expect("Should not have gotten here.");

        let mime_type = mime_guess::from_path(&file_name)
            .first_raw()
            .unwrap_or("application/octet-stream");
        res.headers_mut()
            .append(header::CONTENT_TYPE, HeaderValue::from_static(mime_type));

        res.headers_mut().append(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("inline; filename=\"{file_name}\"")).map_err(|_e| {
                ServerError::InternalError {
                    message: "Parse error".to_string(),
                }
            })?,
        );

        res.headers_mut()
            .append(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));

        Ok(res)
    } else {
        // For directory listing, check if user has read permission for the directory
        let sfile = files.get_sfile(&path, target_user_id).await?;

        // Extract user_id first before any borrowing issues
        let user_id = auth_context.as_ref().map(|Extension(ctx)| ctx.user_id);

        // Special handling for root directory - allow access for any authenticated user
        if path.is_root() {
            // For root directory, just require authentication
            if auth_context.is_none() {
                return Err(ServerError::AuthenticationError {
                    message: "Authentication required to access directories".to_string(),
                });
            }
        } else {
            // Check if directory is public - if so, allow access regardless of authentication
            if !sfile.is_public {
                // Directory is private, require authentication and permission checking
                let auth_context = match auth_context {
                    Some(Extension(ctx)) => ctx,
                    None => {
                        return Err(ServerError::AuthenticationError {
                            message: "Authentication required to access private directories"
                                .to_string(),
                        })
                    }
                };

                // First check if user is the owner (direct ownership via sfiles.user_id)
                let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile.id as i64)
                    .fetch_optional(files.db_pool())
                    .await?
                    .map(|row| row.user_id == Some(auth_context.user_id))
                    .unwrap_or(false);

                // If not owner, check ReBAC permissions
                if !is_owner
                    && !auth_context.has_permission(
                        "sfile",
                        Some(sfile.id as i64),
                        Permission::Read,
                    )
                {
                    return Err(ServerError::AuthorizationError {
                        message: "You don't have permission to access this directory".to_string(),
                    });
                }
            }
        }
        let list = files
            .list_dir(&path, user_id, target_user_id)
            .await?
            .ok_or(ServerError::PathDoesntExist)?;
        Ok(Response::new(Body::from(serde_json::to_string(&list)?)))
    }
}

pub async fn delete_file(
    Extension(auth_context): Extension<AuthContext>,
    State(files): State<FileController>,
    Path(path): Path<VirtualPath>,
) -> ServerResult<()> {
    // Check if user has delete permission for this file
    let sfile = files.get_sfile(&path, auth_context.user_id).await?;

    // Delete requires authentication regardless of public status - only owners can delete
    // First check if user is the owner (direct ownership via sfiles.user_id)
    let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile.id as i64)
        .fetch_optional(files.db_pool())
        .await?
        .map(|row| row.user_id == Some(auth_context.user_id))
        .unwrap_or(false);

    // If not owner, check ReBAC permissions (only Owner relationship has Delete permission)
    if !is_owner && !auth_context.has_permission("sfile", Some(sfile.id as i64), Permission::Delete)
    {
        return Err(ServerError::AuthorizationError {
            message: "Only file owners can delete files".to_string(),
        });
    }

    files.delete_sfile(&path, auth_context.user_id).await?;

    Ok(())
}

pub async fn change_file_visibility(
    Extension(auth_context): Extension<AuthContext>,
    State(files): State<FileController>,
    Json(visibility_info): Json<VisibilityInfo>,
) -> ServerResult<Json<SFile>> {
    // Check if user has permission to change visibility
    let sfile = files
        .get_sfile(&visibility_info.path, auth_context.user_id)
        .await?;

    // First check if user is the owner (direct ownership via sfiles.user_id)
    let is_owner = query!("SELECT user_id FROM sfiles WHERE id = $1", sfile.id as i64)
        .fetch_optional(files.db_pool())
        .await?
        .map(|row| row.user_id == Some(auth_context.user_id))
        .unwrap_or(false);

    // If not owner, check ReBAC permissions
    if !is_owner
        && !auth_context.has_permission(
            "sfile",
            Some(sfile.id as i64),
            Permission::ChangePermissions,
        )
    {
        return Err(ServerError::AuthorizationError {
            message: "Only file owners can change file visibility".to_string(),
        });
    }

    // Parse visibility string
    // TODO! unnecessary and unproper
    let visibility = match visibility_info.visibility.as_str() {
        "public" => FileVisibility::Public,
        "private" => FileVisibility::Private,
        _ => {
            return Err(ServerError::ValidationError {
                message: "Visibility must be 'public' or 'private'".to_string(),
            })
        }
    };

    let updated = files
        .set_file_visibility(&visibility_info.path, visibility, auth_context.user_id)
        .await?;

    Ok(Json(updated))
}
