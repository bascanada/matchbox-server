#!/bin/bash
# Usage: ./extract-pubkey-from-token.sh <jwt-token>
# Prints the public key (base64) from the JWT token's 'sub' claim

if [ -z "$1" ]; then
  echo "Usage: $0 <jwt-token>" >&2
  exit 1
fi

TOKEN="$1"

# Extract the payload (second part), decode from base64, and get the 'sub' field

# Extract the payload (second part), convert base64url to base64, add padding, decode, and get the 'sub' field
PAYLOAD=$(echo "$TOKEN" | awk -F. '{print $2}')
PAYLOAD_B64=$(echo "$PAYLOAD" | tr '_-' '/+'; printf '%0.s=' $(seq 1 $(( (4 - ${#PAYLOAD} % 4) % 4 ))))
PUBKEY=$(echo "$PAYLOAD_B64" | base64 -d 2>/dev/null | jq -r '.sub')

echo "$PUBKEY"
