#!/bin/bash
# Usage: ./host_and_get_url.sh <username> <password>
# This script logs in, creates a lobby, and prints the WSS URL to join the game.

set -e
USERNAME="$1"
PASSWORD="$2"

if [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
  echo "Usage: $0 <username> <password>" >&2
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

# Step 4: Create lobby and extract lobby id
LOBBY_RESPONSE=$(curl -s -X POST http://localhost:3536/lobbies \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"is_private":false}')
LOBBY_ID=$(echo "$LOBBY_RESPONSE" | jq -r '.id')

# Step 5: Print the WSS URL to join the game (token in path)
WSS_URL="ws://localhost:3536/$TOKEN"
echo "$LOBBY_ID"
echo "$WSS_URL"
