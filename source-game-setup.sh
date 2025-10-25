#!/bin/bash
# Usage: source ./source-game-setup.sh <num_players> [server_url]
# Example: source ./source-game-setup.sh 4
# Example: source ./source-game-setup.sh 4 https://matchbox.example.com
#
# This script exports TOKEN_1, TOKEN_2, etc. directly to your current shell

# Parse arguments
NUM_PLAYERS="${1:-2}"
SERVER_URL="${2:-http://localhost:3536}"

if [ "$NUM_PLAYERS" -lt 2 ]; then
  echo "Error: Need at least 2 players" >&2
  return 1
fi

echo "Setting up multiplayer game with $NUM_PLAYERS players..."

# Get the directory where this script is located
# When sourced, BASH_SOURCE[0] contains the path to the script
SCRIPT_DIR=$HOME/Project/bascanada/matchbox-server


# Function to authenticate a player and get token
authenticate_player() {
  local username="$1"
  local password="$2"

  
  local challenge=$(curl -s -X POST "$SERVER_URL/auth/challenge" | jq -r '.challenge')
  local auth_json=$(cargo run --quiet --manifest-path "$SCRIPT_DIR/Cargo.toml" --example client-auth-demo -- -u "$username" -p "$password" -c "$challenge")
  local token=$(curl -s -X POST "$SERVER_URL/auth/login" \
    -H 'Content-Type: application/json' \
    -d "$auth_json" | jq -r '.token')
  
  echo "$token"
}

# Authenticate all players
echo "Authenticating players..."
declare -a TOKENS
for i in $(seq 1 "$NUM_PLAYERS"); do
  username="player$i"
  password="pass$i"
  token=$(authenticate_player "$username" "$password")
  TOKENS[$i]="$token"
done

# Player 1 creates the lobby
echo "Player 1 creating lobby..."
LOBBY_ID=$(curl -s -X POST "$SERVER_URL/lobbies" \
  -H "Authorization: Bearer ${TOKENS[1]}" \
  -H 'Content-Type: application/json' \
  -d '{"is_private":true}' | jq -r '.id')

if [ -z "$LOBBY_ID" ] || [ "$LOBBY_ID" = "null" ]; then
  echo "Error: Failed to create lobby" >&2
  return 1
fi

# Other players join the lobby
if [ "$NUM_PLAYERS" -gt 1 ]; then
  echo "Other players joining lobby..."
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
  eval "export WS_URL_$i='$WS_SERVER_URL/${TOKENS[$i]}'"
done

echo
echo "✓ Setup complete!"
echo
echo "Exported variables:"
echo "  LOBBY_ID=$LOBBY_ID"
echo "  NUM_PLAYERS=$NUM_PLAYERS"
for i in $(seq 1 "$NUM_PLAYERS"); do
  echo "  TOKEN_$i=<exported>"
  echo "  WS_URL_$i=<exported>"
done
echo
echo "To connect player 1: websocat \"\$WS_URL_1\""
echo "To connect player 2: websocat \"\$WS_URL_2\""
