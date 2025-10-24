use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Clone)]
pub struct AuthSecret(pub String);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // public key
    pub username: String,
    pub exp: usize,
}

pub fn verify_signature(
    public_key_b64: &str,
    message: &str,
    signature_b64: &str,
) -> Result<bool, anyhow::Error> {
    let public_key_bytes = general_purpose::STANDARD.decode(public_key_b64)?;
    let signature_bytes = general_purpose::STANDARD.decode(signature_b64)?;

    let public_key = VerifyingKey::from_bytes(
        &public_key_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?,
    )?;
    let signature = Signature::from_bytes(
        &signature_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid signature length"))?,
    );

    Ok(public_key
        .verify_strict(message.as_bytes(), &signature)
        .is_ok())
}

pub fn issue_jwt(
    public_key_b64: String,
    username: String,
    secret: &AuthSecret,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: public_key_b64,
        username,
        exp: expiration as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.0.as_ref()),
    )
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    AuthSecret: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let secret = AuthSecret::from_ref(state);
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AuthError::InvalidToken)?;

        let bearer_token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        let token_data = decode::<Claims>(
            bearer_token,
            &DecodingKey::from_secret(secret.0.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid token")]
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
