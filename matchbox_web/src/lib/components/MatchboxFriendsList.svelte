<script>
    import { friendsList, generateMyFriendCode, addFriendFromCode, removeFriend, isLoggedIn } from '../matchbox-service.js';
    import PubKeyDisplay from './PubKeyDisplay.svelte';

    let friendCodeToAdd = '';
    let myFriendCode = '';
    let errorMessage = '';
    let successMessage = '';

    function handleAddFriend() {
        if (!friendCodeToAdd) return;
        try {
            addFriendFromCode(friendCodeToAdd);
            friendCodeToAdd = '';
            errorMessage = '';
            successMessage = 'Friend added successfully!';
            setTimeout(() => successMessage = '', 3000);
        } catch (error) {
            errorMessage = error.message;
            successMessage = '';
        }
    }

    async function handleCopyFriendCode() {
        try {
            myFriendCode = generateMyFriendCode();
            await navigator.clipboard.writeText(myFriendCode);
            successMessage = 'Your Friend Code has been copied to the clipboard!';
            errorMessage = '';
            setTimeout(() => successMessage = '', 3000);
        } catch (error) {
            errorMessage = 'Failed to copy Friend Code.';
            successMessage = '';
        }
    }
</script>

<div class="friends-list-container">
    {#if $isLoggedIn}
        <h2>Friends</h2>

        {#if $friendsList.length === 0}
            <p>Your friends list is empty. Add a friend using their Friend Code!</p>
        {/if}

            <ul>
                {#each $friendsList as friend (friend.publicKey)}
                    <li>
                        <div class="friend-info">
                            <strong>{friend.username}</strong>
                            <PubKeyDisplay pubkey={friend.publicKey} />
                        </div>
                        <button on:click={() => removeFriend(friend.publicKey)}>Remove</button>
                    </li>
                {/each}
            </ul>

        <div class="add-friend-section">
            <h3>Add Friend</h3>
            <input type="text" bind:value={friendCodeToAdd} placeholder="Enter Friend Code" />
            <button on:click={handleAddFriend}>Add</button>
        </div>

        <div class="my-friend-code-section">
            <button on:click={handleCopyFriendCode}>Share my Friend Code</button>
        </div>

        {#if errorMessage}
            <p class="error">{errorMessage}</p>
        {/if}

        {#if successMessage}
            <p class="success">{successMessage}</p>
        {/if}
    {:else}
        <p>Please log in to see your friends list.</p>
    {/if}
</div>

<style>
    .friends-list-container {
        font-family: sans-serif;
        border: 1px solid #ccc;
        padding: 1em;
        border-radius: 5px;
        max-width: 400px;
    }
    ul {
        list-style: none;
        padding: 0;
    }
    li {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.5em 0;
        border-bottom: 1px solid #eee;
    }
    .add-friend-section, .my-friend-code-section {
        margin-top: 1em;
    }
    input {
        width: calc(100% - 60px);
        padding: 0.5em;
    }
    button {
        cursor: pointer;
    }
    .error {
        color: red;
    }
    .success {
        color: green;
    }
</style>
