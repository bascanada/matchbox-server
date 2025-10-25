<script>
    import { onMount, onDestroy } from 'svelte';
    import { lobbies, getLobbies, joinLobby, deleteLobby, friendsList, currentUser } from '../matchbox-service.js';
    import { toast } from '@zerodevx/svelte-toast';
    import PubKeyDisplay from './PubKeyDisplay.svelte';

    let autoRefresh = false;
    let refreshInterval;
    let isLoading = false;

    // A map for quick friend lookups
    let friendMap = {};
    friendsList.subscribe(friends => {
        friendMap = friends.reduce((acc, friend) => {
            acc[friend.publicKey] = friend.username;
            return acc;
        }, {});
    });

    async function fetchLobbies() {
        isLoading = true;
        try {
            await getLobbies();
        } catch (error) {
            toast.error(error.message);
        } finally {
            isLoading = false;
        }
    }

    function handleAutoRefreshChange() {
        if (autoRefresh) {
            fetchLobbies(); // Fetch immediately
            refreshInterval = setInterval(fetchLobbies, 3000);
        } else {
            clearInterval(refreshInterval);
        }
    }

    async function handleJoin(lobbyId) {
        try {
            await joinLobby(lobbyId);
            toast.success('Joined lobby successfully!');
        } catch (error) {
            toast.error(error.message);
        }
    }

    async function handleDelete(lobbyId) {
        try {
            await deleteLobby(lobbyId);
            toast.success('Lobby deleted (locally).');
        } catch (error) {
            toast.error(error.message);
        }
    }

    // Fetch lobbies when the component mounts
    onMount(fetchLobbies);

    // Clear the interval when the component is destroyed
    onDestroy(() => {
        if (refreshInterval) {
            clearInterval(refreshInterval);
        }
    });
</script>

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
                    <th>ID</th>
                    <th>Players</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {#each $lobbies as lobby (lobby.id)}
                    <tr>
                        <td>{lobby.is_private ? 'Private' : 'Public'}</td>
                        <td><PubKeyDisplay pubKey={lobby.id} /></td>
                        <td>
                            <ul>
                                {#each lobby.players as player}
                                    <li>
                                        {friendMap[player] || player}
                                    </li>
                                {/each}
                            </ul>
                        </td>
                        <td>
                            <button on:click={() => handleJoin(lobby.id)}>Join</button>
                            <!-- Only show delete button if the user is in the lobby -->
                            {#if lobby.players.includes($currentUser?.publicKey)}
                                <button class="delete" on:click={() => handleDelete(lobby.id)}>Delete</button>
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

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
    }
    th, td {
        border: 1px solid #ddd;
        padding: 8px;
        text-align: left;
    }
    th {
        background-color: #f2f2f2;
    }
    ul {
        list-style-type: none;
        padding: 0;
        margin: 0;
    }
    .delete {
        background-color: #f44336;
        color: white;
    }
</style>
