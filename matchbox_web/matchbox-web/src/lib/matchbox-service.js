import { writable } from 'svelte/store';
import * as bip39 from 'bip39';
import argon2 from 'argon2-browser';
import * as ed from 'noble-ed25519';
import { ethers } from 'ethers';

// --- Configuration ---
const API_BASE_URL = writable('http://localhost:3536'); // Default, user can change this
let apiBaseUrlValue;
API_BASE_URL.subscribe(value => apiBaseUrlValue = value);


// --- State Management ---
import { browser } from '$app/environment';

export const isLoggedIn = writable(false);
export const currentUser = writable(null);
export const jwt = writable(browser ? localStorage.getItem('matchbox-jwt') : null);

// Automatically update login status when JWT changes
jwt.subscribe(token => {
  if (!browser) return; // Don't run on the server
  isLoggedIn.set(!!token);
  if (!token) {
    currentUser.set(null);
    localStorage.removeItem('matchbox-jwt');
  } else {
    localStorage.setItem('matchbox-jwt', token);
    // You might want to decode the JWT to get user info here
    // For now, we'll set it on login
  }
});


// --- Helper Functions ---

/**
 * Derives a salt from a username using SHA-256.
 * @param {string} username - The username.
 * @returns {Promise<Uint8Array>} The salt.
 */
const getSalt = async (username) => {
    const encoder = new TextEncoder();
    const data = encoder.encode(username);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    return new Uint8Array(hashBuffer);
};

/**
 * Derives a private key from a username and secret using Argon2.
 * @param {string} username - The username (acts as part of the salt).
 * @param {string} secret - The user's secret/password.
 * @returns {Promise<Uint8Array>} The 32-byte private key.
 */
export async function getPrivateKey(username, secret) {
    const salt = await getSalt(username);
    const hash = await argon2.hash({
        pass: secret,
        salt: salt,
        time: 1,
        mem: 16 * 1024,
        hashLen: 32,
        parallelism: 1,
        type: argon2.ArgonType.Argon2id,
    });
    // Return the raw hash which will be our private key
    return hash.hash;
};


// --- Service Methods ---

/**
 * Creates a new account with a username and secret.
 * @param {string} username - The desired username.
 * @param {string} secret - The desired secret.
 */
export async function createAccount(username, secret) {
    const privateKey = await getPrivateKey(username, secret);
    const publicKey = await ed.getPublicKey(privateKey);
    const publicKeyHex = ed.utils.bytesToHex(publicKey);

    const response = await fetch(`${apiBaseUrlValue}/api/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ public_key: publicKeyHex }),
    });

    if (!response.ok) {
        const error = await response.text();
        throw new Error(`Failed to create account: ${error}`);
    }

    // After creating the account, automatically log in.
    return await loginWithSecret(username, secret);
}

/**
 * Logs in a user with their username and secret.
 * @param {string} username - The user's username.
 * @param {string} secret - The user's secret.
 */
export async function loginWithSecret(username, secret) {
    const privateKey = await getPrivateKey(username, secret);
    const publicKey = await ed.getPublicKey(privateKey);
    const publicKeyHex = ed.utils.bytesToHex(publicKey);

    // 1. Get challenge
    const challengeResponse = await fetch(`${apiBaseUrlValue}/api/challenge/${publicKeyHex}`);
    if (!challengeResponse.ok) {
        const error = await challengeResponse.text();
        throw new Error(`Failed to get challenge: ${error}`);
    }
    const { challenge } = await challengeResponse.json();

    // 2. Sign challenge
    const signature = await ed.sign(challenge, privateKey);
    const signatureHex = ed.utils.bytesToHex(signature);

    // 3. Request JWT
    const loginResponse = await fetch(`${apiBaseUrlValue}/api/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            public_key: publicKeyHex,
            signature: signatureHex,
        }),
    });

    if (!loginResponse.ok) {
        const error = await loginResponse.text();
        throw new Error(`Login failed: ${error}`);
    }

    const { token } = await loginResponse.json();
    jwt.set(token);
    currentUser.set({ username, publicKey: publicKeyHex, isWallet: false });

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
 * A placeholder for the account recovery logic.
 * @param {string} username - The username.
 * @param {string} recoveryPhrase - The recovery phrase.
 */
export async function recoverAccount(username, recoveryPhrase) {
    // This is a placeholder. In a real implementation, you would use the
    // recovery phrase to regenerate the user's private key.
    console.log(`Recovering account for ${username} with phrase: ${recoveryPhrase}`);
    // For now, we'll just throw an error to indicate that this is not implemented.
    throw new Error('Account recovery is not yet implemented.');
}

/**
 * Logs the current user out.
 */
export function logout() {
    jwt.set(null);
}

/**
 * Allows changing the Matchbox server URL.
 * @param {string} newUrl - The new URL for the Matchbox server.
 */
export function setApiUrl(newUrl) {
    API_BASE_URL.set(newUrl);
}
