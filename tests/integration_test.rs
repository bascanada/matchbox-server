use matchbox_server::helpers;
use reqwest::Client;
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

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
    let login_payload_str =
        helpers::generate_login_payload("testuser", "testpass", challenge).unwrap();
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
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
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
    let login_payload_b =
        helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
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
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
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
    let login_payload_b =
        helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
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

#[tokio::test]
#[serial]
async fn test_whitelist_allows_whitelisted_player() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A (Host) ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();

    // --- Player B (Guest) ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_b = body["challenge"].as_str().unwrap();
    let login_payload_b =
        helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_b)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_b = body["token"].as_str().unwrap();

    // Get player B's public key
    let pubkey_b = helpers::get_public_key("player_b", "pass_b").unwrap();

    // 2. Create a lobby with player B whitelisted
    let create_lobby_body = json!({
        "is_private": true,
        "whitelist": [pubkey_b]
    });
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(create_lobby_body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let lobby_id = body["id"].as_str().unwrap();

    // 3. Player B should be able to join (they're whitelisted)
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
async fn test_whitelist_blocks_non_whitelisted_player() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A (Host) ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();

    // --- Player B (Whitelisted) ---
    let pubkey_b = helpers::get_public_key("player_b", "pass_b").unwrap();

    // --- Player C (Not whitelisted) ---
    // 1. Authenticate
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_c = body["challenge"].as_str().unwrap();
    let login_payload_c =
        helpers::generate_login_payload("player_c", "pass_c", challenge_c).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_c)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_c = body["token"].as_str().unwrap();

    // 2. Create a lobby with only player B whitelisted
    let create_lobby_body = json!({
        "is_private": true,
        "whitelist": [pubkey_b]
    });
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(create_lobby_body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let lobby_id = body["id"].as_str().unwrap();

    // 3. Player C should NOT be able to join (not whitelisted)
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_c))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403); // Forbidden
}

#[tokio::test]
#[serial]
async fn test_whitelist_multiple_players() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A (Host) ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();

    // --- Player B ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_b = body["challenge"].as_str().unwrap();
    let login_payload_b =
        helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_b)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_b = body["token"].as_str().unwrap();
    let pubkey_b = helpers::get_public_key("player_b", "pass_b").unwrap();

    // --- Player C ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_c = body["challenge"].as_str().unwrap();
    let login_payload_c =
        helpers::generate_login_payload("player_c", "pass_c", challenge_c).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_c)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_c = body["token"].as_str().unwrap();
    let pubkey_c = helpers::get_public_key("player_c", "pass_c").unwrap();

    // --- Player D (not whitelisted) ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_d = body["challenge"].as_str().unwrap();
    let login_payload_d =
        helpers::generate_login_payload("player_d", "pass_d", challenge_d).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_d)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_d = body["token"].as_str().unwrap();

    // Create a lobby with players B and C whitelisted
    let create_lobby_body = json!({
        "is_private": true,
        "whitelist": [pubkey_b, pubkey_c]
    });
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(create_lobby_body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let lobby_id = body["id"].as_str().unwrap();

    // Player B should be able to join
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_b))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    // Player C should be able to join
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_c))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    // Player D should NOT be able to join
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_d))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403); // Forbidden
}

#[tokio::test]
#[serial]
async fn test_lobby_without_whitelist_allows_all_players() {
    let addr = spawn_app().await;
    let client = Client::new();

    // --- Player A (Host) ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_a = body["challenge"].as_str().unwrap();
    let login_payload_a =
        helpers::generate_login_payload("player_a", "pass_a", challenge_a).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_a)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_a = body["token"].as_str().unwrap();

    // --- Player B ---
    let response = client
        .post(format!("http://{}/auth/challenge", addr))
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let challenge_b = body["challenge"].as_str().unwrap();
    let login_payload_b =
        helpers::generate_login_payload("player_b", "pass_b", challenge_b).unwrap();
    let response = client
        .post(format!("http://{}/auth/login", addr))
        .header("Content-Type", "application/json")
        .body(login_payload_b)
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    let token_b = body["token"].as_str().unwrap();

    // Create a lobby WITHOUT whitelist
    let create_lobby_body = json!({
        "is_private": true
    });
    let response = client
        .post(format!("http://{}/lobbies", addr))
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(create_lobby_body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let lobby_id = body["id"].as_str().unwrap();

    // Player B should be able to join (no whitelist = all allowed)
    let response = client
        .post(format!("http://{}/lobbies/{}/join", addr, lobby_id))
        .header("Authorization", format!("Bearer {}", token_b))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}
