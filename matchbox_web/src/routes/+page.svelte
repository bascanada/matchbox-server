<script>
  import MatchboxAuth from '$lib/components/MatchboxAuth.svelte';
</script>

<h1>Matchbox Auth Component Test</h1>

<MatchboxAuth />

<style>
  :global(body) {
    font-family: Arial, sans-serif;
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    margin: 0;
    background-color: #f0f2f5;
  }
  h1 {
    position: absolute;
    top: 20px;
    color: #333;
  }
</style>
