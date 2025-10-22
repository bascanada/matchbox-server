#!/bin/bash
# Usage: source ./source-game-setup-whitelist.sh <num_players> [server_url]
# Example: source ./source-game-setup-whitelist.sh 3
#
# This script creates a private lobby with a whitelist and allows only specified players to join

set -e

# Parse arguments
NUM_PLAYERS="${1:-2}"
SERVER_URL="${2:-http://localhost:3536}"

if [ "$NUM_PLAYERS" -lt 2 ]; then
  echo "Error: Need at least 2 players" >&2
  return 1
fi

echo "Setting up WHITELISTED multiplayer game with $NUM_PLAYERS players..."

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Function to authenticate a player and get token + public key
authenticate_player() {
  local username="$1"
  local password="$2"
  
  local challenge=$(curl -s -X POST "$SERVER_URL/auth/challenge" | jq -r '.challenge')
  local auth_json=$(cd "$SCRIPT_DIR" && cargo run --quiet --example client-auth-demo -- -u "$username" -p "$password" -c "$challenge" 2>/dev/null)
  local token=$(curl -s -X POST "$SERVER_URL/auth/login" \
    -H 'Content-Type: application/json' \
    -d "$auth_json" | jq -r '.token')
  
  # Extract public key from the auth JSON
  local pubkey=$(echo "$auth_json" | jq -r '.public_key_b64')
  
  echo "$token|$pubkey"
}

# Authenticate all players
echo "Authenticating players..."
declare -a TOKENS
declare -a PUBKEYS
for i in $(seq 1 "$NUM_PLAYERS"); do
  username="player$i"
  password="pass$i"
  result=$(authenticate_player "$username" "$password")
  TOKENS[$i]=$(echo "$result" | cut -d'|' -f1)
  PUBKEYS[$i]=$(echo "$result" | cut -d'|' -f2)
done

# Build whitelist array (all players)
WHITELIST_JSON="["
for i in $(seq 1 "$NUM_PLAYERS"); do
  if [ $i -gt 1 ]; then
    WHITELIST_JSON="$WHITELIST_JSON,"
  fi
  WHITELIST_JSON="$WHITELIST_JSON\"${PUBKEYS[$i]}\""
done
WHITELIST_JSON="$WHITELIST_JSON]"

# Player 1 creates the private lobby with whitelist
echo "Player 1 creating private lobby with whitelist..."
LOBBY_ID=$(curl -s -X POST "$SERVER_URL/lobbies" \
  -H "Authorization: Bearer ${TOKENS[1]}" \
  -H 'Content-Type: application/json' \
  -d "{\"is_private\":true,\"whitelist\":$WHITELIST_JSON}" | jq -r '.id')

if [ -z "$LOBBY_ID" ] || [ "$LOBBY_ID" = "null" ]; then
  echo "Error: Failed to create lobby" >&2
  return 1
fi

# Other players join the lobby
if [ "$NUM_PLAYERS" -gt 1 ]; then
  echo "Other players joining whitelisted lobby..."
  for i in $(seq 2 "$NUM_PLAYERS"); do
    curl -s -X POST "$SERVER_URL/lobbies/$LOBBY_ID/join" \
      -H "Authorization: Bearer ${TOKENS[$i]}" \
      -H 'Content-Type: application/json' \
      -d '{}' > /dev/null
  done
fi

# Generate WebSocket URLs
WS_SERVER_URL=$(echo "$SERVER_URL" | sed 's/^http/ws/')

# Export all variables to the current shell
export LOBBY_ID
export NUM_PLAYERS
export SERVER_URL
export WS_SERVER_URL

for i in $(seq 1 "$NUM_PLAYERS"); do
  eval "export TOKEN_$i='${TOKENS[$i]}'"
  eval "export PUBKEY_$i='${PUBKEYS[$i]}'"
  eval "export WS_URL_$i='$WS_SERVER_URL/${TOKENS[$i]}'"
done

echo
echo "✓ Setup complete!"
echo
echo "Exported variables:"
echo "  LOBBY_ID=$LOBBY_ID (PRIVATE with WHITELIST)"
echo "  NUM_PLAYERS=$NUM_PLAYERS"
for i in $(seq 1 "$NUM_PLAYERS"); do
  echo "  TOKEN_$i=<exported>"
  echo "  PUBKEY_$i=${PUBKEYS[$i]}"
  echo "  WS_URL_$i=<exported>"
done
echo
echo "Whitelisted public keys:"
for i in $(seq 1 "$NUM_PLAYERS"); do
  echo "  Player $i: ${PUBKEYS[$i]}"
done
echo
echo "To connect player 1: websocat \"\$WS_URL_1\""
echo "To connect player 2: websocat \"\$WS_URL_2\""
