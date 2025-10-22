pub mod args;
pub mod auth;
pub mod lobby;
pub mod state;
pub mod topology;

use crate::{
    auth::AuthSecret,
    state::ServerState,
    topology::MatchmakingDemoTopology,
};
use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use matchbox_signaling::SignalingServerBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
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
    let secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let state = ServerState::default();
    let app_state = AppState {
        state: state.clone(),
        secret: AuthSecret(secret.clone()),
    };
    let app_router = app(app_state);

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
    challenge: String,
    signature_b64: String,
}

async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    if !state
        .state
        .challenge_manager
        .verify_challenge(&payload.challenge)
    {
        return Err((StatusCode::UNAUTHORIZED, "Invalid challenge"));
    }

    let signature_valid =
        match auth::verify_signature(&payload.public_key_b64, &payload.challenge, &payload.signature_b64) {
            Ok(valid) => valid,
            Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid signature")),
        };

    if !signature_valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature"));
    }

    match auth::issue_jwt(payload.public_key_b64, &state.secret) {
        Ok(token) => Ok(Json(json!({ "token": token }))),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to issue token",
        )),
    }
}

#[derive(Deserialize)]
pub struct CreateLobbyRequest {
    is_private: bool,
}

async fn create_lobby_handler(
    State(state): State<AppState>,
    claims: auth::Claims,
    Json(payload): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    let mut lobby = lobby_manager.create_lobby(payload.is_private);
    let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
    players_in_lobbies.insert(claims.sub.clone(), lobby.id);
    lobby.players.insert(claims.sub.clone());
    tracing::info!(lobby_id = %lobby.id, pubkey = %&claims.sub[..8], "Lobby created and player added");
    Json(lobby)
}

async fn list_lobbies_handler(State(state): State<AppState>) -> impl IntoResponse {
    let lobby_manager = state.state.lobby_manager.read().unwrap();
    let lobbies = lobby_manager.get_public_lobbies();
    Json(lobbies)
}

async fn join_lobby_handler(
    State(state): State<AppState>,
    Path(lobby_id): Path<uuid::Uuid>,
    claims: auth::Claims,
) -> impl IntoResponse {
    let mut lobby_manager = state.state.lobby_manager.write().unwrap();
    let result = lobby_manager.add_player_to_lobby(&lobby_id, claims.sub.clone());
    // Insert the joining player's public key into players_in_lobbies
    let mut players_in_lobbies = state.state.players_in_lobbies.write().unwrap();
    players_in_lobbies.insert(claims.sub.clone(), lobby_id);
    tracing::debug!(full_pubkey = %claims.sub, "Full public key for join");
    tracing::debug!(players_in_lobbies = ?*players_in_lobbies, "Current players_in_lobbies map");
    if result.is_ok() {
        tracing::info!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player joined lobby");
        StatusCode::OK
    } else {
        tracing::warn!(lobby_id = %lobby_id, pubkey = %&claims.sub[..8], "Player failed to join lobby: not found");
        StatusCode::NOT_FOUND
    }
}
