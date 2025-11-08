<script>
    import { onMount, onDestroy } from 'svelte';
    import { lobbies, getLobbies, joinLobby, deleteLobby, inviteToLobby, friendsList, currentUser, isLoggedIn, jwt } from '../matchbox-service.js';
    import { toast } from '@zerodevx/svelte-toast';

    // Callback function that will be called when joining a lobby
    // Provides: { lobbyId, token, players, isPrivate }
    export let onJoinLobby = null;

    // Auto-refresh is enabled by default
    let autoRefresh = true;
    let refreshInterval;
    let isLoading = false;
    let showInviteModal = false;
    let selectedLobbyForInvite = null;
    let selectedFriendsToInvite = [];

    // A map for quick friend lookups
    let friendMap = {};
    friendsList.subscribe(friends => {
        friendMap = friends.reduce((acc, friend) => {
            acc[friend.publicKey] = friend.username;
            return acc;
        }, {});
    });

    // Helper function to get display name for a player
    // Accepts either a publicKey string or a player object { publicKey }
    function getPlayerDisplayName(player) {
        const publicKey = typeof player === 'string' ? player : player?.publicKey;
        if (!publicKey) return 'Unknown Player';
        // Check if it's the current user
        if ($currentUser?.publicKey === publicKey) {
            return `${$currentUser.username} (You)`;
        }
        // Check if it's a friend
        return friendMap[publicKey] || `Player ${publicKey.substring(0, 8)}...`;
    }

    async function fetchLobbies() {
        isLoading = true;
        try {
            await getLobbies();
        } catch (error) {
            toast.push(error.message || 'Failed to fetch lobbies');
        } finally {
            isLoading = false;
        }
    }

    function handleAutoRefreshChange() {
        // Always clear any existing interval to avoid duplicates
        if (refreshInterval) {
            clearInterval(refreshInterval);
            refreshInterval = null;
        }

        if (autoRefresh) {
            fetchLobbies(); // Fetch immediately
            refreshInterval = setInterval(fetchLobbies, 3000);
        }
    }

    async function handleJoin(lobby) {
        const inLobby = isUserInLobby(lobby);

        // If user is already in the lobby, treat this as "Start Game" and call the callback
        if (inLobby) {
            if (onJoinLobby && typeof onJoinLobby === 'function') {
                try {
                    const token = $jwt;
                    await onJoinLobby({
                        lobbyId: lobby.id,
                        token: token,
                        players: lobby.players,
                        isPrivate: lobby.is_private,
                    });
                } catch (error) {
                    toast.push(error.message || 'Failed to start game');
                }
            } else {
                toast.push('No start-game callback provided');
            }
            return;
        }

        // Not in lobby -> join via API endpoint
        try {
            await joinLobby(lobby.id);
            toast.push('Joined lobby successfully!');
        } catch (error) {
            toast.push(error.message || 'Failed to join lobby');
        }
    }

    async function handleDelete(lobbyId) {
        // Check if user is owner to show appropriate message
        const lobby = $lobbies.find(l => l.id === lobbyId);
        const isOwner = lobby?.owner === $currentUser?.publicKey;
        
        try {
            await deleteLobby(lobbyId);
            toast.push(isOwner ? 'Lobby deleted successfully!' : 'Left lobby successfully!');
        } catch (error) {
            toast.push(error.message || (isOwner ? 'Failed to delete lobby' : 'Failed to leave lobby'));
        }
    }

    // Check if current user is in a lobby
    function isUserInLobby(lobby) {
        const userKey = $currentUser?.publicKey;
        if (!userKey || !lobby?.players) return false;
        // Players may be an array of publicKey strings or player objects { publicKey }
        return lobby.players.some(p => (typeof p === 'string' ? p : p?.publicKey) === userKey);
    }

    function openInviteModal(lobby) {
        selectedLobbyForInvite = lobby;
        selectedFriendsToInvite = [];
        showInviteModal = true;
    }

    function closeInviteModal() {
        showInviteModal = false;
        selectedLobbyForInvite = null;
        selectedFriendsToInvite = [];
    }

    async function handleInvite() {
        if (!selectedLobbyForInvite || selectedFriendsToInvite.length === 0) {
            toast.push('Please select at least one friend to invite');
            return;
        }

        try {
            await inviteToLobby(selectedLobbyForInvite.id, selectedFriendsToInvite);
            toast.push(`Invited ${selectedFriendsToInvite.length} friend(s) successfully!`);
            closeInviteModal();
        } catch (error) {
            toast.push(error.message || 'Failed to invite friends');
        }
    }

    // Get friends that are not already in the lobby and not in whitelist
    function getInvitableFriends(lobby) {
        const players = lobby?.players || [];
        return $friendsList.filter(friend => {
            // Don't show if already in lobby (players may be strings or objects)
            if (players.some(p => (typeof p === 'string' ? p : p?.publicKey) === friend.publicKey)) return false;
            // Don't show if already whitelisted
            if (lobby.whitelist && lobby.whitelist.includes(friend.publicKey)) return false;
            return true;
        });
    }

    // Fetch lobbies / start auto-refresh when the component mounts (only if logged in)
    onMount(() => {
        if ($isLoggedIn) {
            // If autoRefresh is enabled (default), this will immediately fetch and start the interval.
            handleAutoRefreshChange();
        }
    });

    // Clear the interval when the component is destroyed
    onDestroy(() => {
        if (refreshInterval) {
            clearInterval(refreshInterval);
        }
    });
</script>

{#if $isLoggedIn}
<div class="lobby-list-container">
    <div class="header">
        <h2>Lobbies</h2>
        <div class="controls">
            <button on:click={fetchLobbies} disabled={isLoading || autoRefresh}>
                {#if isLoading}Refreshing...{:else}Refresh{/if}
            </button>
            <label>
                <input type="checkbox" bind:checked={autoRefresh} on:change={handleAutoRefreshChange} />
                Auto-Refresh (3s)
            </label>
        </div>
    </div>

    {#if $lobbies.length === 0}
        <p>No lobbies found.</p>
    {:else}
        <table>
            <thead>
                <tr>
                    <th>Privacy</th>
                    <th>Lobby ID</th>
                    <th>Players ({$lobbies.reduce((sum, l) => sum + l.players.length, 0)})</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {#each $lobbies as lobby (lobby.id)}
                    <tr class:user-in-lobby={isUserInLobby(lobby)}>
                        <td>
                            <span class="privacy-badge" class:private={lobby.is_private}>
                                {lobby.is_private ? '🔒 Private' : '🌐 Public'}
                            </span>
                        </td>
                        <td>
                            <code class="lobby-id" title={lobby.id}>
                                {lobby.id.substring(0, 8)}...
                            </code>
                        </td>
                        <td>
                            <ul class="player-list">
                                {#each lobby.players as player}
                                    <li class:current-user={(typeof player === 'string' ? player : player?.publicKey) === $currentUser?.publicKey}>
                                        {getPlayerDisplayName(player)}
                                    </li>
                                {/each}
                            </ul>
                        </td>
                        <td class="actions">
                            {#if !isUserInLobby(lobby)}
                                <button class="join-btn" on:click={() => handleJoin(lobby)}>
                                    Join
                                </button>
                            {:else}
                                <div class="action-buttons">
                                    {#if lobby.owner === $currentUser?.publicKey}
                                            {#if onJoinLobby && lobby.owner === $currentUser?.publicKey}
                                                <button class="join-btn" on:click={() => handleJoin(lobby)}>
                                                    Start Game
                                                </button>
                                            {/if}
                                            {#if lobby.is_private && getInvitableFriends(lobby).length > 0}
                                                <button class="invite-btn" on:click={() => openInviteModal(lobby)}>
                                                    ➕ Invite
                                                </button>
                                            {/if}
                                            <button class="delete-btn" on:click={() => handleDelete(lobby.id)}>
                                                🗑️ Delete
                                            </button>
                                    {:else}
                                        <button class="leave-btn" on:click={() => handleDelete(lobby.id)}>
                                            🚪 Leave
                                        </button>
                                    {/if}
                                </div>
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
{:else}
<div class="lobby-list-container">
    <p>Please log in to view and manage lobbies.</p>
    <!-- Optionally the MatchboxAuth component could be shown here in the future -->
</div>
{/if}

<!-- Invite Modal -->
{#if showInviteModal && selectedLobbyForInvite}
<div class="modal-overlay" on:click={closeInviteModal} on:keydown={(e) => e.key === 'Escape' && closeInviteModal()} role="button" tabindex="0">
    <div class="modal-content" on:click|stopPropagation on:keydown role="dialog" aria-modal="true" tabindex="-1">
        <h3>Invite Friends to Lobby</h3>
        <p class="modal-subtitle">Lobby ID: <code>{selectedLobbyForInvite.id.substring(0, 8)}...</code></p>
        
        {#if getInvitableFriends(selectedLobbyForInvite).length === 0}
            <p>All your friends are already invited or in this lobby.</p>
        {:else}
            <div class="friend-select-list">
                {#each getInvitableFriends(selectedLobbyForInvite) as friend}
                    <label class="friend-item">
                        <input 
                            type="checkbox" 
                            value={friend.publicKey} 
                            bind:group={selectedFriendsToInvite}
                        />
                        <span>{friend.username}</span>
                    </label>
                {/each}
            </div>
        {/if}
        
        <div class="modal-actions">
            <button class="cancel-btn" on:click={closeInviteModal}>Cancel</button>
            <button 
                class="invite-confirm-btn" 
                on:click={handleInvite}
                disabled={selectedFriendsToInvite.length === 0}
            >
                Invite ({selectedFriendsToInvite.length})
            </button>
        </div>
    </div>
</div>
{/if}

<style>
    .lobby-list-container {
        margin-top: 1em;
    }
    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1em;
    }
    .controls {
        display: flex;
        gap: 1em;
        align-items: center;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    th, td {
        border: 1px solid #ddd;
        padding: 12px 8px;
        text-align: left;
    }
    th {
        background-color: #f8f9fa;
        font-weight: 600;
        color: #333;
    }
    .user-in-lobby {
        background-color: #e8f5e9;
    }
    .privacy-badge {
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 0.875em;
        font-weight: 500;
    }
    .privacy-badge.private {
        background-color: #fff3cd;
        color: #856404;
    }
    .privacy-badge:not(.private) {
        background-color: #d1ecf1;
        color: #0c5460;
    }
    .lobby-id {
        font-family: monospace;
        background-color: #f5f5f5;
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 0.875em;
        cursor: pointer;
    }
    .lobby-id:hover {
        background-color: #e0e0e0;
    }
    .player-list {
        list-style-type: none;
        padding: 0;
        margin: 0;
    }
    .player-list li {
        padding: 2px 0;
    }
    .player-list li.current-user {
        font-weight: 600;
        color: #1976d2;
    }
    .actions {
        white-space: nowrap;
    }
    .join-btn {
        background-color: #4caf50;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
    }
    .join-btn:hover {
        background-color: #45a049;
    }
    .join-btn:disabled {
        background-color: #cccccc;
        cursor: not-allowed;
    }
    .delete-btn {
        background-color: #f44336;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
    }
    .delete-btn:hover {
        background-color: #da190b;
    }
    .leave-btn {
        background-color: #ff9800;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
    }
    .leave-btn:hover {
        background-color: #e68900;
    }
    .action-buttons {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }
    .invite-btn {
        background-color: #2196f3;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
    }
    .invite-btn:hover {
        background-color: #0b7dda;
    }
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }
    .modal-content {
        background: white;
        padding: 24px;
        border-radius: 8px;
        max-width: 500px;
        width: 90%;
        max-height: 80vh;
        overflow-y: auto;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    }
    .modal-content h3 {
        margin-top: 0;
        margin-bottom: 8px;
    }
    .modal-subtitle {
        color: #666;
        font-size: 0.875em;
        margin-bottom: 16px;
    }
    .modal-subtitle code {
        background-color: #f5f5f5;
        padding: 2px 6px;
        border-radius: 3px;
        font-family: monospace;
    }
    .friend-select-list {
        max-height: 300px;
        overflow-y: auto;
        border: 1px solid #ddd;
        border-radius: 4px;
        padding: 8px;
        margin-bottom: 16px;
    }
    .friend-item {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px;
        cursor: pointer;
        border-radius: 4px;
    }
    .friend-item:hover {
        background-color: #f5f5f5;
    }
    .friend-item input[type="checkbox"] {
        cursor: pointer;
    }
    .modal-actions {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
    }
    .cancel-btn {
        background-color: #999;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
    }
    .cancel-btn:hover {
        background-color: #777;
    }
    .invite-confirm-btn {
        background-color: #4caf50;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
    }
    .invite-confirm-btn:hover:not(:disabled) {
        background-color: #45a049;
    }
    .invite-confirm-btn:disabled {
        background-color: #cccccc;
        cursor: not-allowed;
    }
</style>
