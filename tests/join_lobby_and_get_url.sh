#!/bin/bash
# Usage: ./join_lobby_and_get_url.sh <username> <password> <lobby_id>
# This script logs in, joins a lobby, and prints the WSS URL to join the game.

set -e
USERNAME="$1"
PASSWORD="$2"
LOBBY_ID="$3"

if [ -z "$USERNAME" ] || [ -z "$PASSWORD" ] || [ -z "$LOBBY_ID" ]; then
  echo "Usage: $0 <username> <password> <lobby_id>" >&2
  exit 1
fi

# Step 1: Get challenge
CHALLENGE=$(curl -s -X POST http://localhost:3536/auth/challenge | jq -r '.challenge')

# Step 2: Run Rust example to get auth payload (mute cargo output)
AUTH_JSON=$(cargo run --quiet --example client-auth-demo -- -u "$USERNAME" -p "$PASSWORD" -c "$CHALLENGE" 2>/dev/null)

# Step 3: POST to login endpoint and extract token
TOKEN=$(curl -s -X POST http://localhost:3536/auth/login \
  -H 'Content-Type: application/json' \
  -d "$AUTH_JSON" | jq -r '.token')

# Step 4: Join the lobby
curl -s -X POST http://localhost:3536/lobbies/$LOBBY_ID/join \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{}'

# Step 5: Print the WSS URL to join the game (token in path)
WSS_URL="ws://localhost:3536/$TOKEN"
echo "$WSS_URL"
