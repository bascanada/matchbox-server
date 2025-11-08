# Matchbox Web Components

UI components for the Matchbox matchmaking server.

## Components

### MatchboxLobbies

A component that displays a list of available lobbies and allows users to create, join, and delete lobbies.

#### Props

- `onJoinLobby` (optional): A callback function that will be called when a user clicks the join/start game button. This allows parent components to handle game launching.

#### Callback Signature

```javascript
onJoinLobby({ lobbyId, token, players, isPrivate })
```

**Parameters:**
- `lobbyId` (string): The UUID of the lobby
- `token` (string): The JWT authentication token for the current user
- `players` (array): Array of public keys of players in the lobby
- `isPrivate` (boolean): Whether the lobby is private

#### Example Usage

```svelte
<script>
  import MatchboxLobbies from '$lib/components/MatchboxLobbies.svelte';

  function handleJoinLobby({ lobbyId, token, players, isPrivate }) {
    console.log('Starting game with:', { lobbyId, token, players, isPrivate });
    
    // Launch your game here
    // For example, connect to the matchbox websocket:
    // const ws = new WebSocket(`ws://localhost:3536/${token}`);
    
    // Or navigate to your game page:
    // window.location.href = `/game?lobby=${lobbyId}&token=${token}`;
  }
</script>

<MatchboxLobbiesComponent onJoinLobby={handleJoinLobby} />
```

#### Using as Web Component

```html
<script>
  // Define the callback before the component loads
  window.handleJoinLobby = function({ lobbyId, token, players, isPrivate }) {
    console.log('Starting game with:', { lobbyId, token, players, isPrivate });
    // Your game launch logic here
  };
</script>

<matchbox-lobbies on-join-lobby="handleJoinLobby"></matchbox-lobbies>
```

## Features

### Improved UI

1. **Clear Lobby IDs**: Lobby IDs are now displayed as shortened codes instead of empty input fields
2. **Player Names**: Players are shown with their usernames (if they're friends) instead of cropped public keys
3. **Current User Highlighting**: Your own player entry is highlighted in the player list
4. **Visual Status**: Lobbies you're in are highlighted with a green background
5. **Smart Actions**: The join button only shows for lobbies you're not in; delete button only shows for lobbies you're in

### Privacy Badges

- 🔒 Private - Yellow badge for private lobbies
- 🌐 Public - Blue badge for public lobbies

### Player Display

- Current user: **Username (You)** in blue
- Friends: Username from friends list
- Others: `Player <pubkey>...`

### Leave vs Delete Lobby

The system now intelligently handles lobby removal based on ownership:

- **Owner**: When the lobby creator clicks "🗑️ Delete Lobby", the entire lobby is destroyed and all players are removed
- **Members**: When a non-owner player clicks "🚪 Leave", only that player is removed from the lobby (lobby continues to exist)

This provides a better user experience and prevents accidental lobby deletion by non-owners.
