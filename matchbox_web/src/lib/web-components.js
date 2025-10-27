// Web Components entry point
import MatchboxAuthComponent from './components/MatchboxAuth.svelte';
import MatchboxFriendsListComponent from './components/MatchboxFriendsList.svelte';
import MatchboxLobbiesComponent from './components/MatchboxLobbies.svelte';


// Register as custom element
if (!customElements.get('matchbox-auth')) {
    customElements.define('matchbox-auth', MatchboxAuthComponent);
}
if (!customElements.get('matchbox-friends-list')) {
    customElements.define('matchbox-friends-list', MatchboxFriendsListComponent);
}
if (!customElements.get('matchbox-lobbies')) {
    customElements.define('matchbox-lobbies', MatchboxLobbiesComponent);
}


// Export the component for direct use if needed
export { MatchboxAuthComponent, MatchboxFriendsListComponent, MatchboxLobbiesComponent };

// Also export the service functions for programmatic use
export * from './matchbox-service.js';
