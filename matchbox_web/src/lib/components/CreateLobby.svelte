<script>
  import { createLobby, friendsList } from '../matchbox-service.js';
  import { toast } from '@zerodevx/svelte-toast';

  let isPrivate = false;
  let selectedFriends = [];
  let isLoading = false;

  async function handleCreateLobby() {
    isLoading = true;
    try {
      const whitelist = isPrivate ? selectedFriends : [];
      await createLobby(isPrivate, whitelist);
      toast.success('Lobby created successfully!');
    } catch (error) {
      toast.error(error.message);
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="create-lobby-container">
  <h2>Create a New Lobby</h2>
  <form on:submit|preventDefault={handleCreateLobby}>
    <div class="form-group">
      <label>
        <input type="checkbox" bind:checked={isPrivate} />
        Private Lobby
      </label>
    </div>

    {#if isPrivate}
      <div class="form-group">
        <label for="whitelist">Whitelist Friends:</label>
        <div class="friends-checkbox-list">
          {#each $friendsList as friend}
            <label>
              <input type="checkbox" value={friend.publicKey} bind:group={selectedFriends} />
              {friend.username}
            </label>
          {/each}
        </div>
      </div>
    {/if}

    <button type="submit" disabled={isLoading}>
      {#if isLoading}
        Creating...
      {:else}
        Create Lobby
      {/if}
    </button>
  </form>
</div>

<style>
  .create-lobby-container {
    padding: 1em;
    border: 1px solid #ccc;
    border-radius: 5px;
    margin-bottom: 1em;
  }
  .form-group {
    margin-bottom: 0.5em;
  }
  .friends-checkbox-list {
    display: flex;
    flex-direction: column;
  }
</style>
