import { writable } from 'svelte/store';
import * as bip39 from 'bip39';
import * as ed from 'noble-ed25519';
import { ethers } from 'ethers';

// --- Configuration ---
const API_BASE_URL = writable('http://localhost:3536'); // Default, user can change this
let apiBaseUrlValue;
API_BASE_URL.subscribe(value => apiBaseUrlValue = value);


// --- State Management ---
// Check if we're in a browser environment (compatible with all bundlers)
const browser = typeof window !== 'undefined';

export const isLoggedIn = writable(false);
export const currentUser = writable(null);
export const jwt = writable(browser ? localStorage.getItem('matchbox-jwt') : null);
export const recoveryPhrase = writable(browser ? localStorage.getItem('matchbox-recovery') : null);

// Automatically update login status when JWT changes
jwt.subscribe(token => {
  if (!browser) return; // Don't run on the server
  isLoggedIn.set(!!token);
  if (!token) {
    currentUser.set(null);
    localStorage.removeItem('matchbox-jwt');
  } else {
    localStorage.setItem('matchbox-jwt', token);
    // Decode JWT to get user info
    const claims = decodeJWT(token);
    if (claims) {
      currentUser.set({ 
        username: claims.username, 
        publicKey: claims.sub, 
        isWallet: false 
      });
    }
  }
});

// Store recovery phrase securely
recoveryPhrase.subscribe(phrase => {
  if (!browser) return;
  if (phrase) {
    localStorage.setItem('matchbox-recovery', phrase);
  } else {
    localStorage.removeItem('matchbox-recovery');
  }
});


// --- Helper Functions ---

// Decode JWT (without verification - only for extracting claims)
function decodeJWT(token) {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = parts[1];
    const decoded = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(decoded);
  } catch (e) {
    console.error('Failed to decode JWT:', e);
    return null;
  }
}

// Helper function for bytes to hex
function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

// Helper function for hex to bytes
function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  return bytes;
}

// Helper function for base64 encoding
function base64Encode(bytes) {
  return btoa(String.fromCharCode(...bytes));
}

/**
 * Derives a salt from a username using SHA-256.
 * @param {string} username - The username.
 * @returns {Promise<Uint8Array>} The salt (first 16 bytes of SHA-256 hash).
 */
const getSalt = async (username) => {
    const encoder = new TextEncoder();
    const data = encoder.encode(username);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = new Uint8Array(hashBuffer);
    return hashArray.slice(0, 16); // Use first 16 bytes as salt
};

/**
 * Derives a private key from a secret using Argon2.
 * @param {string} username - The username (used for salt derivation).
 * @param {string} secret - The user's secret/password.
 * @returns {Promise<Uint8Array>} The 32-byte private key.
 */
export async function getPrivateKey(username, secret) {
    if (!argon2) {
        throw new Error('Argon2 not initialized. Please wait for the module to load.');
    }
    
    const salt = await getSalt(username);
    const hash = await argon2.hash({
        pass: secret,
        salt: salt,
        time: 1,
        mem: 16 * 1024,
        hashLen: 32,
        parallelism: 1,
        type: argon2.ArgonType?.Argon2id || 2, // Argon2id = 2
    });
    return hash.hash;
}

/**
 * Generates a random secret key (32 bytes as hex).
 * @returns {string} A random secret key in hex format.
 */
function generateSecretKey() {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    return bytesToHex(bytes);
}

/**
 * Generates a recovery key (same as secret key, 64-char hex string).
 * @returns {string} A random recovery key in hex format.
 */
function generateRecoveryKey() {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    return bytesToHex(bytes);
}


// --- Service Methods ---

/**
 * Creates a new account by generating a random secret key and recovery key.
 * @param {string} username - The desired username.
 * @returns {Promise<{token: string, recoveryKey: string, secretKey: string}>}
 */
export async function createAccount(username) {
    // Generate a random secret key (32 bytes)
    const secretKey = generateSecretKey();
    
    // Generate a real mnemonic for recovery
    const mnemonic = bip39.generateMnemonic();
    
    // Login with the generated secret to create the account on the server
    const token = await loginWithSecret(username, secretKey);
    
    // Store the mnemonic
    recoveryPhrase.set(mnemonic);
    
    return {
        token,
        recoveryPhrase: mnemonic,
        secretKey,
    };
}

/**
 * Logs in a user with their username and secret.
 * @param {string} username - The user's username.
 * @param {string} secret - The user's secret key (hex string).
 */
export async function loginWithSecret(username, secret) {
    // 1. Derive private key from username + secret
    const privateKey = await getPrivateKey(username, secret);
    const publicKey = await ed.getPublicKey(privateKey);
    const publicKeyB64 = base64Encode(publicKey);

    // 2. Get challenge
    const challengeResponse = await fetch(`${apiBaseUrlValue}/auth/challenge`, {
        method: 'POST',
    });
    if (!challengeResponse.ok) {
        const error = await challengeResponse.text();
        throw new Error(`Failed to get challenge: ${error}`);
    }
    const { challenge } = await challengeResponse.json();

    // 3. Sign challenge (convert string to bytes first)
    const encoder = new TextEncoder();
    const challengeBytes = encoder.encode(challenge);
    const signature = await ed.sign(challengeBytes, privateKey);
    const signatureB64 = base64Encode(signature);

    // 4. Request JWT
    const loginResponse = await fetch(`${apiBaseUrlValue}/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            public_key_b64: publicKeyB64,
            username: username,
            challenge: challenge,
            signature_b64: signatureB64,
        }),
    });

    if (!loginResponse.ok) {
        const error = await loginResponse.text();
        throw new Error(`Login failed: ${error}`);
    }

    const { token } = await loginResponse.json();
    jwt.set(token);
    // currentUser will be set automatically by JWT subscription

    return token;
}

/**
 * Initiates login process using a browser wallet (e.g., MetaMask).
 */
export async function loginWithWallet() {
    if (!window.ethereum) {
        throw new Error('No crypto wallet found. Please install a wallet extension like MetaMask.');
    }

    try {
        const provider = new ethers.BrowserProvider(window.ethereum);
        const signer = await provider.getSigner();
        const address = await signer.getAddress();

        // 1. Get challenge (assuming a different endpoint for ETH addresses)
        const challengeResponse = await fetch(`${apiBaseUrlValue}/api/challenge/eth/${address}`);
        if (!challengeResponse.ok) {
            throw new Error('Failed to get challenge for wallet address. The server may not support wallet login yet.');
        }
        const { challenge } = await challengeResponse.json();

        // 2. Sign challenge
        const signature = await signer.signMessage(challenge);

        // 3. Request JWT (assuming a different endpoint for ETH addresses)
        const loginResponse = await fetch(`${apiBaseUrlValue}/api/login/eth`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                address: address,
                signature: signature,
            }),
        });

        if (!loginResponse.ok) {
            const error = await loginResponse.text();
            throw new Error(`Wallet login failed: ${error}`);
        }

        const { token } = await loginResponse.json();
        jwt.set(token);
        // For wallet users, the username can be their address
        currentUser.set({ username: address, publicKey: address, isWallet: true });

        return token;
    } catch (err) {
        console.error("Wallet login error:", err);
        throw err;
    }
}

/**
 * Recovers an account using the recovery phrase.
 * Derives the secret key from the mnemonic and logs in.
 * @param {string} username - The username.
 * @param {string} mnemonic - The 24-word recovery phrase.
 */
export async function recoverAccount(username, mnemonic) {
    if (!bip39.validateMnemonic(mnemonic)) {
        throw new Error('Invalid recovery phrase');
    }
    
    // Derive seed from mnemonic
    const seed = await bip39.mnemonicToSeed(mnemonic);
    
    // Use first 32 bytes as secret key
    const secretKey = bytesToHex(seed.slice(0, 32));
    
    // Login with the recovered secret
    const token = await loginWithSecret(username, secretKey);
    
    // Store the recovery phrase
    recoveryPhrase.set(mnemonic);
    
    return token;
}

/**
 * Logs the current user out.
 */
export function logout() {
    jwt.set(null);
    recoveryPhrase.set(null);
}

/**
 * Allows changing the Matchbox server URL.
 * @param {string} newUrl - The new URL for the Matchbox server.
 */
export function setApiUrl(newUrl) {
    API_BASE_URL.set(newUrl);
}

// Use dynamic import for argon2-browser and fallback to global/window if needed
let argon2;
// Export a promise that resolves when argon2 is ready
export const argon2Ready = (async () => {
  try {
    const argon2Module = await import('argon2-browser');
    argon2 = argon2Module.ArgonType ? argon2Module : (typeof window !== 'undefined' ? window.argon2 : undefined);
    if (!argon2) {
      throw new Error('argon2-browser module not found.');
    }
  } catch (e) {
    if (typeof window !== 'undefined') {
      argon2 = window.argon2;
    }
    if (!argon2) {
      console.error('argon2-browser could not be loaded:', e);
      throw e; // Propagate error to fail promise
    }
  }
})();
