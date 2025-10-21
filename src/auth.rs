use axum::{
    async_trait,
    extract::{FromRequestParts, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::request::Parts,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;

pub const JWT_SECRET: &[u8] = b"secret"; // In a real app, this should be a secure, configurable secret
pub const CHALLENGE_EXPIRATION: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (public key)
    pub exp: usize,  // Expiration time
}

#[derive(Debug, Clone, Default)]
pub struct ChallengeManager {
    challenges: Arc<Mutex<HashMap<String, Instant>>>,
}

impl ChallengeManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn generate_challenge(&self) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        let challenge: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let mut challenges = self.challenges.lock().unwrap();
        challenges.insert(challenge.clone(), Instant::now());
        challenge
    }

    pub fn verify_challenge(&self, challenge: &str) -> bool {
        let mut challenges = self.challenges.lock().unwrap();
        if let Some(timestamp) = challenges.get(challenge) {
            if timestamp.elapsed() < CHALLENGE_EXPIRATION {
                challenges.remove(challenge);
                return true;
            }
        }
        false
    }

    pub fn cleanup_expired_challenges(&self) {
        let mut challenges = self.challenges.lock().unwrap();
        challenges.retain(|_, timestamp| timestamp.elapsed() < CHALLENGE_EXPIRATION);
    }
}

pub fn verify_signature(
    public_key_b64: &str,
    message: &str,
    signature_b64: &str,
) -> Result<bool, anyhow::Error> {
    let public_key_bytes = general_purpose::STANDARD.decode(public_key_b64)?;
    let signature_bytes = general_purpose::STANDARD.decode(signature_b64)?;

    let public_key = VerifyingKey::from_bytes(&public_key_bytes)?;
    let signature = Signature::from_bytes(&signature_bytes)?;

    Ok(public_key
        .verify_strict(message.as_bytes(), &signature)
        .is_ok())
}

pub fn issue_jwt(public_key_b64: String) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: public_key_b64,
        exp: expiration as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, _state)
                .await
                .map_err(|_| AuthError::InvalidToken)?;

        // Decode the user data
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

pub enum AuthError {
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
