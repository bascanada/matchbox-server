<script>
  import MatchboxAuth from '$lib/components/MatchboxAuth.svelte';
  import MatchboxFriendsList from '$lib/components/MatchboxFriendsList.svelte';
</script>

<div class="page-container">
  <h1>Matchbox Auth Component Test</h1>

  <div class="info">
    <h3>📦 Svelte Integration Demo</h3>
    <p>This page demonstrates the Matchbox authentication component and friends list as Svelte components.</p>
    <p>Use the auth component to create/login; the friends list will appear when you're logged in.</p>
  </div>

  <div class="components">
    <MatchboxAuth />
    <MatchboxFriendsList />
  </div>
</div>

<style>
  :global(body) {
    font-family: Arial, sans-serif;
    margin: 0;
    background-color: #f0f2f5;
  }

  .page-container {
    max-width: 900px;
    margin: 40px auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    align-items: center;
  }

  h1 {
    color: #333;
    margin: 0;
  }

  .info {
    background-color: #e3f2fd;
    border-left: 4px solid #2196f3;
    padding: 15px;
    width: 100%;
    border-radius: 4px;
  }

  .components {
    display: flex;
    gap: 20px;
    width: 100%;
    justify-content: center;
    align-items: flex-start;
  }

  /* Ensure components stack on small screens */
  @media (max-width: 800px) {
    .components {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
