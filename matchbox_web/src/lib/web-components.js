// Web Components entry point
import MatchboxAuthComponent from './components/MatchboxAuth.svelte';

// Register as custom element
customElements.define('matchbox-auth', MatchboxAuthComponent);

// Export the component for direct use if needed
export { MatchboxAuthComponent };

// Also export the service functions for programmatic use
export * from './matchbox-service.js';
