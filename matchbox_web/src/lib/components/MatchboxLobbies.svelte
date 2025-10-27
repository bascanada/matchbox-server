<script>
    import CreateLobby from './CreateLobby.svelte';
    import LobbyList from './LobbyList.svelte';
    import { SvelteToast } from '@zerodevx/svelte-toast';
</script>

<div class="matchbox-lobbies-container">
    <SvelteToast />
    <h1>Matchbox Lobbies</h1>
    <CreateLobby />
    <LobbyList />
</div>

<style>
    .matchbox-lobbies-container {
        font-family: sans-serif;
    }
</style>
