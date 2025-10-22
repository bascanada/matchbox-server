<script>
  import {
    createAccount,
    loginWithSecret,
    loginWithWallet,
    logout,
    recoverAccount,
    isLoggedIn,
    currentUser,
  } from "../matchbox-service.js";

  // Component state
  let view = "initial"; // 'initial', 'secretKeyLogin', 'secretKeySignUp', 'walletLogin', 'secretKeyRecover'
  let username = "";
  let secret = "";
  let recoveryPhrase = "";
  let error = null;
  let isLoading = false;

  const handleSecretKeySignUp = async () => {
    if (!username || !secret) {
      error = "Username and Secret Key are required.";
      return;
    }
    isLoading = true;
    error = null;
    try {
      await createAccount(username, secret);
      // Login is handled inside createAccount
      view = "loggedIn";
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  };

  const handleSecretKeyLogin = async () => {
    if (!username || !secret) {
      error = "Username and Secret Key are required.";
      return;
    }
    isLoading = true;
    error = null;
    try {
      await loginWithSecret(username, secret);
      view = "loggedIn";
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  };

  const handleWalletLogin = async () => {
    isLoading = true;
    error = null;
    try {
      await loginWithWallet();
      view = "loggedIn";
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  };

  const handleRecovery = async () => {
    if (!username || !recoveryPhrase) {
      error = "Username and Recovery Phrase are required.";
      return;
    }
    isLoading = true;
    error = null;
    try {
      // This will currently fail, as the service function is a placeholder
      await recoverAccount(username, recoveryPhrase);
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  };

  const handleLogout = () => {
    logout();
    view = "initial";
    username = "";
    secret = "";
    recoveryPhrase = "";
  };

  // Reset error on input change
  $: if (username || secret || recoveryPhrase) error = null;

  // Reactive view based on login state
  $: if ($isLoggedIn) {
    view = "loggedIn";
  }
</script>

<div class="matchbox-auth-container">
  {#if view === 'loggedIn'}
    <div class="welcome-view">
      <h3>Welcome, {$currentUser?.username}</h3>
      <button on:click={handleLogout}>Log Out</button>
    </div>
  {:else if view === 'initial'}
    <div class="initial-view">
      <h2>Join or Log In</h2>
      <button on:click={() => view = 'secretKeySignUp'}>
        Create Account with Secret Key
      </button>
      <button on:click={() => view = 'secretKeyLogin'}>
        Log In with Secret Key
      </button>
      <button on:click={() => view = 'secretKeyRecover'}>
        Recover Account
      </button>
      <button on:click={handleWalletLogin} disabled={isLoading}>
        {isLoading ? "Connecting..." : "Log In with Wallet"}
      </button>
    </div>
  {:else if view === 'secretKeySignUp' || view === 'secretKeyLogin'}
    <div class="form-view">
      <h2>{view === 'secretKeySignUp' ? 'Create Account' : 'Log In'}</h2>
      <input
        type="text"
        placeholder="Username"
        bind:value={username}
        disabled={isLoading}
      />
      <input
        type="password"
        placeholder="Secret Key (like a password)"
        bind:value={secret}
        disabled={isLoading}
      />
      {#if view === 'secretKeySignUp'}
        <button on:click={handleSecretKeySignUp} disabled={isLoading}>
          {isLoading ? "Creating..." : "Create Account"}
        </button>
      {:else}
        <button on:click={handleSecretKeyLogin} disabled={isLoading}>
          {isLoading ? "Logging in..." : "Log In"}
        </button>
      {/if}
      <button class="back-button" on:click={() => view = 'initial'} disabled={isLoading}>
        &larr; Back
      </button>
    </div>
    {:else if view === 'secretKeyRecover'}
    <div class="form-view">
      <h2>Recover Account</h2>
      <input
        type="text"
        placeholder="Username"
        bind:value={username}
        disabled={isLoading}
      />
      <input
        type="password"
        placeholder="Recovery Phrase"
        bind:value={recoveryPhrase}
        disabled={isLoading}
      />
      <button on:click={handleRecovery} disabled={isLoading}>
        {isLoading ? "Recovering..." : "Recover"}
      </button>
      <button class="back-button" on:click={() => view = 'initial'} disabled={isLoading}>
        &larr; Back
      </button>
    </div>
  {/if}

  {#if error}
    <p class="error-message">{error}</p>
  {/if}
</div>

<style>
  .matchbox-auth-container {
    font-family: sans-serif;
    border: 1px solid #ccc;
    border-radius: 8px;
    padding: 20px;
    max-width: 350px;
    background-color: #f9f9f9;
  }
  h2, h3 {
    text-align: center;
    color: #333;
  }
  button {
    display: block;
    width: 100%;
    padding: 10px;
    margin: 10px 0;
    border-radius: 5px;
    border: none;
    background-color: #007bff;
    color: white;
    font-size: 16px;
    cursor: pointer;
    transition: background-color 0.2s;
  }
  button:hover {
    background-color: #0056b3;
  }
  button:disabled {
    background-color: #ccc;
    cursor: not-allowed;
  }
  .back-button {
    background-color: #6c757d;
  }
  .back-button:hover {
    background-color: #5a6268;
  }
  input {
    display: block;
    width: calc(100% - 20px);
    padding: 10px;
    margin: 10px 0;
    border: 1px solid #ccc;
    border-radius: 5px;
    font-size: 16px;
  }
  .error-message {
    color: #d93025;
    text-align: center;
    margin-top: 10px;
  }
  .initial-view, .form-view, .welcome-view {
    display: flex;
    flex-direction: column;
  }
</style>
