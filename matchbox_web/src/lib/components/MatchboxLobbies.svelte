<script>
    import CreateLobby from './CreateLobby.svelte';
    import LobbyList from './LobbyList.svelte';
    import { SvelteToast } from '@zerodevx/svelte-toast';

    // Export a callback prop that parent components can provide
    // This will be called when the user wants to start/join a game
    export let onJoinLobby = null;
</script>

<div class="matchbox-lobbies-container">
    <SvelteToast />
    <h1>Matchbox Lobbies</h1>
    <CreateLobby />
    <LobbyList {onJoinLobby} />
</div>

<style>
    .matchbox-lobbies-container {
        font-family: sans-serif;
    }
</style>
