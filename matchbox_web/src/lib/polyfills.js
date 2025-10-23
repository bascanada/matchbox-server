// Polyfills for browser environment
import { Buffer } from 'buffer/';

// Make Buffer available globally for libraries that expect it (like bip39)
if (typeof window !== 'undefined') {
  window.Buffer = Buffer;
  globalThis.Buffer = Buffer;
}

export { Buffer };
