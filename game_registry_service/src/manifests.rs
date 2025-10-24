use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::{json, Value};
use tracing::debug;

/// Axum handler to serve a placeholder game manifest.
pub async fn serve_manifest(
    Path((game_slug, version)): Path<(String, String)>,
) -> impl IntoResponse {
    debug!(%game_slug, %version, "Request for placeholder manifest");

    // For now, return a hardcoded manifest.
    // In the future, this will be dynamically generated from the database.
    let manifest: Value = json!({
        "game_slug": game_slug,
        "version": version,
        "publisher_pubkey_b64": "PLACEHOLDER_PUBKEY",
        "manifest_content": {
            "files": {
                "game.exe": "sha256:placeholder_hash_1",
                "assets/textures.pak": "sha256:placeholder_hash_2"
            },
            "launch_command": "game.exe"
        },
        "manifest_signature_b64": "PLACEHOLDER_SIGNATURE"
    });

    (StatusCode::OK, Json(manifest))
}
