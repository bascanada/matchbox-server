use anyhow::Result;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use std::convert::TryInto;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HelperError {
    #[error("argon2 error: {0}")]
    Argon2(String),
    #[error("failed to extract hash")]
    HashExtraction,
    #[error("b64 encode error")]
    Base64,
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("try from slice error")]
    TryFromSlice,
}

pub fn generate_login_payload(
    username: &str,
    password: &str,
    challenge: &str,
) -> Result<String, HelperError> {
    let mut salt_bytes = [0u8; 16];
    let username_bytes = username.as_bytes();
    let len = std::cmp::min(username_bytes.len(), 16);
    salt_bytes[..len].copy_from_slice(&username_bytes[..len]);
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| HelperError::Base64)?;
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt).map_err(|e| HelperError::Argon2(e.to_string()))?;
    let hash_value = hash.hash.ok_or(HelperError::HashExtraction)?;
    let hash_bytes = hash_value.as_bytes();

    let seed: [u8; 32] = hash_bytes[..32]
        .try_into()
        .map_err(|_| HelperError::TryFromSlice)?;
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(challenge.as_bytes());

    let public_key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());
    let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());

    let login_payload = json!({
        "public_key_b64": public_key_b64,
        "challenge": challenge,
        "signature_b64": signature_b64
    });

    Ok(serde_json::to_string(&login_payload)?)
}
