use serial_test::serial;
use std::net::SocketAddr;
use reqwest::Client;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

mod helpers;

async fn spawn_app() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    tokio::spawn(async move {
        matchbox_server::run(addr).await.unwrap();
    });
    sleep(Duration::from_millis(100)).await;
    addr
}

#[tokio::test]
#[serial]
async fn test_authentication_flow() {
    let addr = spawn_app().await;
    let client = Client::new();

    // 1. Get a challenge
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let challenge = body["challenge"].as_str().unwrap();

    // 2. Generate login payload
    let login_payload = helpers::generate_login_payload("testuser", "testpass", challenge).unwrap();

    // 3. Log in
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert!(body["token"].as_str().is_some());
}

#[tokio::test]
#[serial]
async fn test_authentication_flow_invalid_signature() {
    let addr = spawn_app().await;
    let client = Client::new();

    // 1. Get a challenge
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let challenge = body["challenge"].as_str().unwrap();

    // 2. Generate a valid login payload
    let login_payload_str = helpers::generate_login_payload("testuser", "testpass", challenge).unwrap();
    let mut login_payload: Value = serde_json::from_str(&login_payload_str).unwrap();

    // 3. Tamper with the signature, replacing it with a bogus value
    login_payload["signature_b64"] = Value::String("aW52YWxpZCBzaWduYXR1cmU=".to_string()); // "invalid signature" in base64

    // 4. Log in with the invalid signature
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload.to_string())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
#[serial]
async fn test_public_lobby_flow() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a = helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();

    // 2. Create a lobby
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(r#"{"is_private": false}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let lobby_id = body["id"].as_str().unwrap();

    // --- Player B ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_b = body["challenge"].as_str().unwrap();
    let login_payload_b = helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_b)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_b = body["token"].as_str().unwrap();

    // 2. Discover lobbies
    let response = client
        .get(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_b))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["id"].as_str().unwrap(), lobby_id);

    // 3. Join the lobby
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_b))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
#[serial]
async fn test_private_lobby_flow() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A ---
    // 1. Authenticate and create a private lobby
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a = helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(r#"{"is_private": true}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    // --- Player B ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_b = body["challenge"].as_str().unwrap();
    let login_payload_b = helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_b)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_b = body["token"].as_str().unwrap();

    // 2. Discover lobbies
    let response = client
        .get(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_b))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 0);
}
