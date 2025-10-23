import { writable, get } from 'svelte/store';
import * as bip39 from 'bip39';
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha2.js';
import { ethers } from 'ethers';

// Enable synchronous methods for ed25519
ed.hashes.sha512 = sha512;
ed.hashes.sha512Async = (m) => Promise.resolve(sha512(m));

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
export const friendsList = writable(
    browser ? JSON.parse(localStorage.getItem('matchbox-friends') || '[]') : []
);


// --- Subscriptions ---

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

// Persist friends list to localStorage
friendsList.subscribe(list => {
    if (!browser) return;
    localStorage.setItem('matchbox-friends', JSON.stringify(list));
});


// --- Helper Functions ---

// Decode JWT (without verification - only for extracting claims)
function decodeJWT(token) {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = parts[1];
    const decoded = Buffer.from(payload.replace(/-/g, '+').replace(/_/g, '/'), 'base64').toString('utf8');
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

// Helper function for base64 decoding
function base64Decode(str) {
    const binaryString = atob(str);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
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
        time: 2,
        mem: 64 * 1024,
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
 * @returns {Promise<{token: string, recoveryPhrase: string, secretKey: string}>}
 */
export async function createAccount(username) {
    // 1. Generate a mnemonic phrase which will be the master recovery key.
    const mnemonic = bip39.generateMnemonic();

    // 2. Derive a deterministic seed from the mnemonic.
    const seed = await bip39.mnemonicToSeed(mnemonic);

    // 3. Use the first 32 bytes of the seed as the secret key.
    const secretKey = bytesToHex(seed.slice(0, 32));

    // 4. Login with the derived secret to register the public key on the server.
    const token = await loginWithSecret(username, secretKey);

    // 5. Store the mnemonic phrase for the session.
    recoveryPhrase.set(mnemonic);

    return {
        token,
        recoveryPhrase: mnemonic, // Return the mnemonic to the user.
        secretKey, // Also return the derived secret key for immediate use.
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
    const publicKey = await ed.getPublicKeyAsync(privateKey);
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
    const signature = await ed.signAsync(challengeBytes, privateKey);
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
 * WARNING: This feature requires backend implementation of /api/challenge/eth/:address
 * and /api/login/eth endpoints, which are not yet implemented.
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
    
    // Derive seed from mnemonic, same as in createAccount
    const seed = await bip39.mnemonicToSeed(mnemonic);
    
    // Use first 32 bytes as the secret key
    const secretKey = bytesToHex(seed.slice(0, 32));
    
    // Login with the recovered secret
    const token = await loginWithSecret(username, secretKey);
    
    // Store the recovery phrase for the new session
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

// --- Friend Management ---

/**
 * Generates a friend code for the current user.
 * A friend code is a base64 encoded JSON string containing the user's username and public key.
 * @returns {string} The friend code.
 */
export function generateMyFriendCode() {
    const user = get(currentUser);
    if (!user) {
        throw new Error('User not logged in.');
    }
    const friendInfo = {
        username: user.username,
        publicKey: user.publicKey
    };
    const json = JSON.stringify(friendInfo);
    const bytes = new TextEncoder().encode(json);
    return base64Encode(bytes);
}

/**
 * Adds a friend from a friend code.
 * @param {string} friendCode - The friend code to add.
 * @throws {Error} If the friend code is invalid or the friend already exists.
 */
export function addFriendFromCode(friendCode) {
    try {
        const bytes = base64Decode(friendCode);
        const json = new TextDecoder().decode(bytes);
        const friendInfo = JSON.parse(json);

        if (!friendInfo.username || !friendInfo.publicKey) {
            throw new Error('Invalid friend code format.');
        }

        const currentFriends = get(friendsList);
        if (currentFriends.some(friend => friend.publicKey === friendInfo.publicKey)) {
            throw new Error('Friend already exists.');
        }

        friendsList.update(list => [...list, friendInfo]);

    } catch (e) {
        console.error('Failed to add friend:', e);
        throw new Error('Invalid or malformed friend code.');
    }
}

/**
 * Removes a friend by their public key.
 * @param {string} publicKey - The public key of the friend to remove.
 */
export function removeFriend(publicKey) {
    friendsList.update(list => list.filter(friend => friend.publicKey !== publicKey));
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
