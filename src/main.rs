mod args;
mod auth;
mod lobby;
mod state;
mod topology;

use crate::{
    auth::{self, ChallengeManager},
    state::ServerState,
    topology::MatchmakingDemoTopology,
};
use args::Args;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use matchbox_signaling::SignalingServerBuilder;
use tracing::info;
use tracing_subscriber::prelude::*;

fn setup_logging() {
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

#[tokio::main]
async fn main() {
    setup_logging();
    let args = Args::parse();
    let state = ServerState::default();

    // Create a new ChallengeManager and add it to the server state
    let challenge_manager = ChallengeManager::new();
    state.challenge_manager.lock().unwrap().clone_from(&challenge_manager);

    let server = SignalingServerBuilder::new(args.host, MatchmakingDemoTopology, state.clone())
        .on_connection_request({
            let state = state.clone();
            move |connection| {
                let lobby_id = connection
                    .path
                    .clone()
                    .and_then(|p| p.strip_prefix("/ws/").map(String::from))
                    .and_then(|s| uuid::Uuid::parse_str(&s).ok());

                let token = connection
                    .query_params
                    .get("token")
                    .cloned();

                // Here, you would typically validate the token and check if the player is in the lobby.
                // For now, we'll just check if a lobby ID and token are present.
                if lobby_id.is_some() && token.is_some() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        })
        .cors()
        .trace()
        .mutate_router(|router| {
            router
                .route("/health", get(health_handler))
                .route("/auth/challenge", post(challenge_handler))
                .route("/auth/login", post(login_handler))
                .route("/lobbies", post(create_lobby_handler).get(list_lobbies_handler))
                .route("/lobbies/:lobby_id/join", post(join_lobby_handler))
                .with_state(state)
        })
        .build();
    server
        .serve()
        .await
        .expect("Unable to run signaling server, is it already running?")
}

pub async fn health_handler() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Serialize)]
struct ChallengeResponse {
    challenge: String,
}

async fn challenge_handler(State(state): State<ServerState>) -> Json<ChallengeResponse> {
    let challenge = state.challenge_manager.lock().unwrap().generate_challenge();
    Json(ChallengeResponse { challenge })
}

#[derive(Deserialize)]
pub struct LoginRequest {
    public_key_b64: String,
    challenge: String,
    signature_b64: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

async fn login_handler(
    State(state): State<ServerState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    if !state
        .challenge_manager
        .lock()
        .unwrap()
        .verify_challenge(&payload.challenge)
    {
        return Err((StatusCode::UNAUTHORIZED, "Invalid challenge"));
    }

    let signature_valid =
        auth::verify_signature(&payload.public_key_b64, &payload.challenge, &payload.signature_b64)
            .unwrap_or(false);

    if !signature_valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature"));
    }

    match auth::issue_jwt(payload.public_key_b64) {
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
    State(state): State<ServerState>,
    claims: auth.Claims,
    Json(payload): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    let mut lobby_manager = state.lobby_manager.lock().unwrap();
    let mut lobby = lobby_manager.create_lobby(payload.is_private);
    lobby.players.insert(claims.sub);
    Json(lobby)
}

async fn list_lobbies_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let lobby_manager = state.lobby_manager.lock().unwrap();
    let lobbies = lobby_manager.get_public_lobbies();
    Json(lobbies)
}

async fn join_lobby_handler(
    State(state): State<ServerState>,
    Path(lobby_id): Path<uuid::Uuid>,
    claims: auth::Claims,
) -> impl IntoResponse {
    let mut lobby_manager = state.lobby_manager.lock().unwrap();
    if lobby_manager
        .add_player_to_lobby(&lobby_id, claims.sub)
        .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
