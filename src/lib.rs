pub mod args;
pub mod auth;
pub mod helpers;
pub mod lobby;
pub mod state;
pub mod topology;

use crate::{auth::AuthSecret, state::ServerState, topology::MatchmakingDemoTopology};
use axum::http::HeaderMap;
use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, delete},
    Router,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use matchbox_signaling::SignalingServerBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::prelude::*;

pub fn setup_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "matchbox_server=info,tower_http=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_file(false)
                .with_target(false),
        )
        .init();
}

#[derive(Clone)]
pub struct AppState {
    pub state: ServerState,
    pub secret: AuthSecret,
}

impl FromRef<AppState> for AuthSecret {
    fn from_ref(input: &AppState) -> Self {
        input.secret.clone()
    }
}

pub async fn run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "test-secret-key-for-development-only".to_string());
    let state = ServerState::default();
    let app_state = AppState {
        state: state.clone(),
        secret: AuthSecret(secret.clone()),
    };
    let app_router = app(app_state);

    let challenge_manager = state.challenge_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            challenge_manager.cleanup_expired();
        }
    });

    let server = SignalingServerBuilder::new(addr, MatchmakingDemoTopology, state.clone())
        .on_connection_request({
            let state = state.clone();
            let secret = AuthSecret(secret);
            move |connection| {
                tracing::info!(origin = ?connection.origin, path = ?connection.path, "WebSocket connection attempt");
                // Extract token from path (matchbox stores path without leading /)
                let token = connection
                    .path
                    .as_ref()
                    .map(|p| p.as_str())
                    .ok_or_else(|| {
                        tracing::warn!(origin = ?connection.origin, path = ?connection.path, "Missing token in path");
                        (StatusCode::UNAUTHORIZED, "Missing token in path").into_response()
                    })?;

                let claims = decode::<auth::Claims>(
                    token,
                    &DecodingKey::from_secret(secret.0.as_ref()),
                    &Validation::default(),
                )
                .map_err(|e| {
                    tracing::warn!(origin = ?connection.origin, error = ?e, "Invalid token");
                    (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
                })?
                .claims;

                tracing::info!(origin = ?connection.origin, pubkey = %&claims.sub[..8], "WebSocket connection request: player connected");

                let mut waiting_players = state.waiting_players.write().unwrap();
                waiting_players.insert(connection.origin, claims.sub);

                // Log current waiting_players size for debugging
                tracing::debug!(waiting_players_count = waiting_players.len(), "Current waiting_players map size");

                Ok(true)
            }
        })
        .on_id_assignment({
            let state = state.clone();
            move |(origin, peer_id)| {
                let mut waiting_players = state.waiting_players.write().unwrap();
                if let Some(player_id) = waiting_players.remove(&origin) {
                    let mut players_to_peers = state.players_to_peers.write().unwrap();
                    players_to_peers.insert(player_id.clone(), peer_id);
                    tracing::info!(origin = ?origin, pubkey = %&player_id[..8], peer_id = ?peer_id, "Assigned peer_id to player");
                } else {
                    tracing::error!(origin = ?origin, "No player_id found in waiting_players during id assignment");
                }
            }
        })
        .cors()
        .trace()
        .mutate_router(|router| router.merge(app_router))
        .build();

    info!("listening on {}", addr);
    server.serve().await?;
    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/auth/challenge", post(challenge_handler))
        .route("/auth/login", post(login_handler))
        .route(
            "/lobbies",
            post(create_lobby_handler).get(list_lobbies_handler),
        )
        .route("/lobbies/:lobby_id/join", post(join_lobby_handler))
        .route("/lobbies/:lobby_id", delete(delete_lobby_handler))
        .route("/lobbies/:lobby_id/invite", post(invite_to_lobby_handler))
        // TODO: Restrict CORS for production environments
        .layer(CorsLayer::very_permissive())
        .with_state(state)
}

pub async fn health_handler() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Serialize)]
struct ChallengeResponse {
    challenge: String,
}

async fn challenge_handler(State(state): State<AppState>) -> Json<ChallengeResponse> {
    let challenge = state.state.challenge_manager.generate_challenge();
    Json(ChallengeResponse { challenge })
}

#[derive(Deserialize)]
pub struct LoginRequest {
    public_key_b64: String,
    username: String,
    challenge: String,
    signature_b64: String,
}

async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    tracing::info!(
        pubkey = %payload.public_key_b64,
        "Login attempt"
    );

    if !state
        .state
        .challenge_manager
        .verify_challenge(&payload.challenge)
    {
        tracing::warn!(pubkey = %payload.public_key_b64, "Challenge verification failed");
        return Err((StatusCode::UNAUTHORIZED, "Invalid challenge"));
    }

    let signature_valid = match auth::verify_signature(
        &payload.public_key_b64,
        &payload.challenge,
        &payload.signature_b64,
    ) {
        Ok(valid) => {
            tracing::debug!(pubkey = %payload.public_key_b64, signature_valid = valid, "Signature verification result");
            valid
        }
        Err(e) => {
            tracing::warn!(pubkey = %payload.public_key_b64, error = ?e, "Signature verification error");
            return Err((StatusCode::UNAUTHORIZED, "Invalid signature"));
        }
    };

    if !signature_valid {
        tracing::warn!(pubkey = %payload.public_key_b64, "Signature validation failed");
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature"));
    }

    match auth::issue_jwt(
        payload.public_key_b64.clone(),
        payload.username.clone(),
        &state.secret,
    ) {
        Ok(token) => {
            tracing::info!(pubkey = %payload.public_key_b64, username = %payload.username, "Login successful");
            Ok(Json(json!({ "token": token })))
        }
        Err(_) => {
            tracing::error!(pubkey = %payload.public_key_b64, "Failed to issue JWT");
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to issue token"))
        }
    }
}

#[derive(Deserialize)]
pub struct CreateLobbyRequest {
    is_private: bool,
    #[serde(default)]
    whitelist: Option<Vec<String>>,
}

async fn create_lobby_handler(
    State(state): State<AppState>,
    claims: auth::Claims,
    Json(payload): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    // Check if player is already in a lobby
    let players_in_lobbies = state.state.players_in_lobbies.read().unwrap();
    if let Some(existing_lobby_id) = players_in_lobbies.get(&claims.sub) {
        tracing::warn!(
            existing_lobby_id = %existing_lobby_id,
            pubkey = %&claims.sub[..8],
            "Player attempted to create lobby while already in one"
        );
        return (StatusCode::CONFLICT, "Already in a lobby").into_response();
    }
    drop(players_in_lobbies);

    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    // Create lobby and ensure the owner is present atomically
    let lobby = lobby_manager.create_lobby_with_owner(payload.is_private, claims.sub.clone(), payload.whitelist);
    let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
    players_in_lobbies.insert(claims.sub.clone(), lobby.id);
    tracing::info!(lobby_id = %lobby.id, pubkey = %&claims.sub[..8], "Lobby created and player added");
    Json(lobby).into_response()
}

async fn list_lobbies_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Try to extract bearer token from Authorization header and decode claims
    let player_pubkey = headers
        .get("authorization")
        .and_then(|hv| hv.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .and_then(|token| {
            decode::<auth::Claims>(
                token,
                &DecodingKey::from_secret(state.secret.0.as_ref()),
                &Validation::default(),
            )
            .ok()
            .map(|data| data.claims.sub)
        });

    let lobby_manager = state.state.lobby_manager.read().unwrap();
    let lobbies = lobby_manager.get_lobbies_for_player(player_pubkey);
    Json(lobbies)
}

async fn join_lobby_handler(
    State(state): State<AppState>,
    Path(lobby_id): Path<uuid::Uuid>,
    claims: auth::Claims,
) -> impl IntoResponse {
    // Check if player is already in a lobby
    let players_in_lobbies = state.state.players_in_lobbies.read().unwrap();
    if let Some(existing_lobby_id) = players_in_lobbies.get(&claims.sub) {
        // Allow rejoining the same lobby (idempotent operation)
        if *existing_lobby_id == lobby_id {
            tracing::debug!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player already in this lobby");
            return StatusCode::OK.into_response();
        }
        tracing::warn!(
            existing_lobby_id = %existing_lobby_id,
            attempted_lobby_id = %lobby_id,
            pubkey = %&claims.sub[..8],
            "Player attempted to join lobby while already in another"
        );
        return (StatusCode::CONFLICT, "Already in a lobby").into_response();
    }
    drop(players_in_lobbies);

    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    let result = lobby_manager.add_player_to_lobby(&lobby_id, claims.sub.clone());

    if result.is_err() {
        // Check if it's a whitelist rejection
        let lobby = lobby_manager.get_lobby(&lobby_id);
        if let Some(lobby) = lobby {
            if let Some(whitelist) = &lobby.whitelist {
                if !whitelist.contains(&claims.sub) {
                    tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player not in whitelist");
                    return (StatusCode::FORBIDDEN, "Not in whitelist").into_response();
                }
            }
        }
        tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player failed to join lobby: not found");
        return (StatusCode::NOT_FOUND, "Lobby not found").into_response();
    }

    // Insert the joining player's public key into players_in_lobbies
    let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
    players_in_lobbies.insert(claims.sub.clone(), lobby_id);
    tracing::debug!(full_pubkey = %claims.sub, "Full public key for join");
    tracing::debug!(players_in_lobbies = ?*players_in_lobbies, "Current players_in_lobbies map");
    tracing::info!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player joined lobby");
    StatusCode::OK.into_response()
}

async fn delete_lobby_handler(
    State(state): State<AppState>,
    Path(lobby_id): Path<uuid::Uuid>,
    claims: auth::Claims,
) -> impl IntoResponse {
    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    
    // Check if lobby exists
    let lobby = lobby_manager.get_lobby(&lobby_id);
    let lobby = match lobby {
        Some(l) => l,
        None => {
            tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Attempted to delete/leave non-existent lobby");
            return (StatusCode::NOT_FOUND, "Lobby not found").into_response();
        }
    };
    
    let is_owner = lobby.owner == claims.sub;
    
    if is_owner {
        // Owner is deleting the lobby - remove it completely
        match lobby_manager.delete_lobby(&lobby_id) {
            Ok(_) => {
                // Remove all players from players_in_lobbies that were in this lobby
                let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
                players_in_lobbies.retain(|_, lid| *lid != lobby_id);
                
                tracing::info!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Lobby deleted by owner");
                
                // TODO: Close all WebSocket connections for players in this lobby
                // This would require tracking peer connections by lobby
                
                StatusCode::OK.into_response()
            }
            Err(_) => {
                tracing::error!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Failed to delete lobby");
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete lobby").into_response()
            }
        }
    } else {
        // Non-owner is leaving the lobby - just remove them
        lobby_manager.remove_player_from_lobby(&lobby_id, &claims.sub);
        
        // Remove player from players_in_lobbies
        let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
        players_in_lobbies.remove(&claims.sub);
        
        tracing::info!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player left lobby");
        StatusCode::OK.into_response()
    }
}

#[derive(Deserialize)]
pub struct InviteToLobbyRequest {
    player_public_keys: Vec<String>,
}

async fn invite_to_lobby_handler(
    State(state): State<AppState>,
    Path(lobby_id): Path<uuid::Uuid>,
    claims: auth::Claims,
    Json(payload): Json<InviteToLobbyRequest>,
) -> impl IntoResponse {
    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    
    // Check if lobby exists
    let lobby = lobby_manager.get_lobby(&lobby_id);
    let lobby = match lobby {
        Some(l) => l,
        None => {
            tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Attempted to invite to non-existent lobby");
            return (StatusCode::NOT_FOUND, "Lobby not found").into_response();
        }
    };
    
    // Only owner can invite
    if lobby.owner != claims.sub {
        tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Non-owner attempted to invite players");
        return (StatusCode::FORBIDDEN, "Only lobby owner can invite players").into_response();
    }
    
    // Add players to whitelist
    match lobby_manager.add_to_whitelist(&lobby_id, payload.player_public_keys.clone()) {
        Ok(_) => {
            tracing::info!(
                lobby_id = %lobby_id,
                pubkey = %&claims.sub[..8],
                invited_count = payload.player_public_keys.len(),
                "Players invited to lobby"
            );
            Json(json!({ "success": true, "invited": payload.player_public_keys })).into_response()
        }
        Err(_) => {
            tracing::error!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Failed to invite players");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to invite players").into_response()
        }
    }
}
