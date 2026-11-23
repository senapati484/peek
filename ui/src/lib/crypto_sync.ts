/**
 * End-to-End Encrypted Cloud Sync Utility for Peek Browser.
 * Uses Web Crypto API: PBKDF2 (SHA-256, 100,000 iterations) + AES-256-GCM.
 * Zero external dependencies. Zero telemetry. Zero plaintext leakage.
 */

import type { BookmarkItem, PinnedSite, TabGroup } from './ipc';

export interface SyncPayload {
  version: number;
  syncedAt: number;
  deviceId: string;
  bookmarks: BookmarkItem[];
  pinnedSites: PinnedSite[];
  notes: string;
  tabGroups: TabGroup[];
}

export interface EncryptedSyncBundle {
  format: 'peek-e2ee-v1';
  salt: string;       // hex
  iv: string;         // hex
  ciphertext: string; // hex
  timestamp: number;
}

function bufToHex(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function hexToBuf(hex: string): Uint8Array {
  const clean = hex.trim();
  if (clean.length % 2 !== 0) {
    throw new Error('Invalid hex string length');
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    bytes[i / 2] = parseInt(clean.substring(i, i + 2), 16);
  }
  return bytes;
}

async function deriveKey(passphrase: string, salt: Uint8Array): Promise<CryptoKey> {
  const enc = new TextEncoder();
  const baseKey = await window.crypto.subtle.importKey(
    'raw',
    enc.encode(passphrase),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  );

  return window.crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: salt as unknown as BufferSource,
      iterations: 100000,
      hash: 'SHA-256',
    },
    baseKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

/**
 * Encrypt a SyncPayload object using a user-provided passphrase.
 * Returns a base64-encoded encrypted token suitable for export, sync, or storage.
 */
export async function encryptSyncPayload(passphrase: string, payload: SyncPayload): Promise<string> {
  if (!passphrase || passphrase.length < 4) {
    throw new Error('Passphrase must be at least 4 characters');
  }

  const salt = window.crypto.getRandomValues(new Uint8Array(16));
  const iv = window.crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveKey(passphrase, salt);

  const enc = new TextEncoder();
  const plaintext = enc.encode(JSON.stringify(payload));

  const encryptedBuffer = await window.crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: iv as unknown as BufferSource,
    },
    key,
    plaintext
  );

  const bundle: EncryptedSyncBundle = {
    format: 'peek-e2ee-v1',
    salt: bufToHex(salt),
    iv: bufToHex(iv),
    ciphertext: bufToHex(encryptedBuffer),
    timestamp: Date.now(),
  };

  const rawJson = JSON.stringify(bundle);
  return btoa(unescape(encodeURIComponent(rawJson)));
}

/**
 * Decrypt an encrypted sync token using the passphrase.
 * Returns the verified SyncPayload.
 */
export async function decryptSyncPayload(passphrase: string, token: string): Promise<SyncPayload> {
  if (!passphrase) {
    throw new Error('Passphrase is required');
  }

  let bundle: EncryptedSyncBundle;
  try {
    const rawJson = decodeURIComponent(escape(atob(token.trim())));
    bundle = JSON.parse(rawJson);
  } catch {
    // Try parsing as plain JSON if not base64
    try {
      bundle = JSON.parse(token.trim());
    } catch {
      throw new Error('Invalid sync token format');
    }
  }

  if (bundle.format !== 'peek-e2ee-v1' || !bundle.salt || !bundle.iv || !bundle.ciphertext) {
    throw new Error('Unsupported or corrupted sync bundle format');
  }

  const salt = hexToBuf(bundle.salt);
  const iv = hexToBuf(bundle.iv);
  const ciphertext = hexToBuf(bundle.ciphertext);
  const key = await deriveKey(passphrase, salt);

  try {
    const decryptedBuffer = await window.crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: iv as unknown as BufferSource,
      },
      key,
      ciphertext as unknown as BufferSource
    );

    const dec = new TextDecoder();
    const jsonStr = dec.decode(decryptedBuffer);
    return JSON.parse(jsonStr) as SyncPayload;
  } catch {
    throw new Error('Decryption failed: Incorrect passphrase or altered sync data.');
  }
}

/**
 * Get or create a persistent local Device ID for sync conflict resolution.
 */
export function getOrCreateDeviceId(): string {
  const KEY = '__peek_sync_device_id';
  let id = localStorage.getItem(KEY);
  if (!id) {
    id = 'peek-node-' + Math.random().toString(36).substring(2, 9) + '-' + Date.now().toString(36);
    localStorage.setItem(KEY, id);
  }
  return id;
}
