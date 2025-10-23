#!/bin/bash

# --- Configuration ---
# 1. PASTE YOUR GITHUB PERSONAL ACCESS TOKEN (PAT) HERE
#    You can generate one at: https://github.com/settings/tokens
#    It needs the 'repo' (or 'public_repo') scope.

# 2. PR Details (from https://github.com/bascanada/matchbox-server/pull/5)
OWNER="bascanada"
REPO="matchbox-server"
PR_NUMBER=5
# ---------------------

# Check if jq is installed
if ! command -v jq &> /dev/null
then
    echo "Error: 'jq' is not installed."
    echo "Please install it to run this script (e.g., 'brew install jq' or 'sudo apt-get install jq')"
    exit 1
fi

# Check if token is set
if [ "$GH_TOKEN" == "YOUR_GITHUB_TOKEN_HERE" ]; then
    echo "Error: Please edit the script and add your GitHub Personal Access Token to the GH_TOKEN variable."
    exit 1
fi

echo "Fetching unresolved comments for $OWNER/$REPO PR #$PR_NUMBER..."
echo ""

# Build the GraphQL query payload using jq
# This is safer than manually escaping newlines and quotes
QUERY_PAYLOAD=$(jq -n \
  --arg owner "$OWNER" \
  --arg repo "$REPO" \
  --arg pr_num "$PR_NUMBER" \
  '{
    "query": "query($owner: String!, $repo: String!, $pr_num: Int!) { repository(owner: $owner, name: $repo) { pullRequest(number: $pr_num) { reviewThreads(first: 100) { nodes { isResolved path comments(first: 50) { nodes { author { login } body url } } } } } } }",
    "variables": {
      "owner": $owner,
      "repo": $repo,
      "pr_num": $pr_num | tonumber
    }
  }')

# Make the API call with curl
RESPONSE=$(curl -s -X POST \
  -H "Authorization: bearer $GH_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$QUERY_PAYLOAD" \
  "https://api.github.com/graphql")

# Check for API errors
if echo "$RESPONSE" | jq -e '.errors' > /dev/null; then
    echo "Error from GitHub API:"
    echo "$RESPONSE" | jq '.errors'
    exit 1
fi

# Parse the response with jq to display *only* unresolved threads
UNRESOLVED_COMMENTS=$(echo "$RESPONSE" | jq -r '
  .data.repository.pullRequest.reviewThreads.nodes[] |
  select(.isResolved == false) |
  (
    "--- UNRESOLVED THREAD on " + .path + " ---",
    (.comments.nodes[] |
      "  Author:  " + .author.login,
      "  Comment: " + .body,
      "  Link:    " + .url,
      ""
    ),
    "--------------------------------------------------\n"
  )
')

if [ -z "$UNRESOLVED_COMMENTS" ]; then
    echo "All clear! No unresolved comments found."
else
    echo -e "$UNRESOLVED_COMMENTS"
fi
