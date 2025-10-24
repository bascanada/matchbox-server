use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, Response},
    response::IntoResponse,
};
use std::{fs, io, path::PathBuf};
use tracing::{debug, error};

use crate::AppState;

/// Generates a content-addressable storage path from a SHA256 hash.
/// e.g., "aabbcc..." -> "{STORAGE_ROOT}/by_hash/aa/bb/aabbcc..."
fn get_storage_path(storage_root: &std::path::Path, sha256: &str) -> io::Result<PathBuf> {
    if sha256.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SHA256 hash is too short",
        ));
    }
    let mut path = PathBuf::from(storage_root);
    path.push("by_hash");
    path.push(&sha256[0..2]);
    path.push(&sha256[2..4]);
    fs::create_dir_all(&path)?;
    path.push(sha256);
    Ok(path)
}

/// Axum handler to serve a file from the content-addressable storage.
pub async fn serve_file(
    State(state): State<AppState>,
    Path(sha256): Path<String>,
) -> impl IntoResponse {
    debug!(%sha256, "Request to serve file");
    let file_path = match get_storage_path(&state.storage_path, &sha256) {
        Ok(path) => path,
        Err(e) => {
            error!("Invalid storage path for hash {}: {}", sha256, e);
            return (StatusCode::BAD_REQUEST, "Invalid file hash").into_response();
        }
    };

    match tokio::fs::read(file_path).await {
        Ok(data) => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(data))
            .unwrap()
            .into_response(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
        Err(e) => {
            error!("Failed to read file for hash {}: {}", sha256, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error reading file").into_response()
        }
    }
}
