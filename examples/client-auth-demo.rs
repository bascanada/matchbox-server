//! A command-line tool to demonstrate the client-side authentication flow.
use anyhow::Result;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use std::convert::TryInto;

#[derive(Parser, Debug)]
#[clap(name = "client-auth-demo")]
struct Args {
    #[clap(short, long)]
    username: String,
    #[clap(short, long)]
    password: String,
    #[clap(short, long)]
    challenge: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut salt_bytes = [0u8; 16];
    let username_bytes = args.username.as_bytes();
    let len = std::cmp::min(username_bytes.len(), 16);
    salt_bytes[..len].copy_from_slice(&username_bytes[..len]);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to encode salt: {}", e))?;
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(args.password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    let hash_value = hash.hash.ok_or("Failed to extract hash")?;
    let hash_bytes = hash_value.as_bytes();

    let seed: [u8; 32] = hash_bytes[..32].try_into()?;
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(args.challenge.as_bytes());

    let public_key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());
    let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());

    let login_payload = json!({
        "public_key_b64": public_key_b64,
        "challenge": args.challenge,
        "signature_b64": signature_b64
    });

    println!("{}", serde_json::to_string_pretty(&login_payload)?);
    Ok(())
}
