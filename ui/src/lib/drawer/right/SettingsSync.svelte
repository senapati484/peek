<script lang="ts">
  import {
    decryptSyncPayload,
    encryptSyncPayload,
    getOrCreateDeviceId,
    type SyncPayload,
  } from '../../crypto_sync';
  import { send, type BookmarkItem, type TabGroup } from '../../ipc';
  import { formatRelativeTime } from '../../utils';

  interface Props {
    bookmarksList: BookmarkItem[];
    tabGroups: TabGroup[];
    noteContent: string;
    noteWords: number;
    onTabGroupsChange?: (groups: TabGroup[]) => void;
    onNoteChange?: (content: string) => void;
  }

  let {
    bookmarksList,
    tabGroups,
    noteContent,
    noteWords,
    onTabGroupsChange,
    onNoteChange,
  }: Props = $props();

  let syncPassphrase = $state(
    typeof window !== 'undefined'
      ? localStorage.getItem('peek_sync_passphrase') || ''
      : '',
  );
  let showPassphrase = $state(false);
  let syncBundleOutput = $state('');
  let syncBundleInput = $state('');
  let syncStatus = $state<string | null>(null);
  let syncError = $state<string | null>(null);
  let lastSyncedTime = $state<number | null>(
    typeof window !== 'undefined' && localStorage.getItem('peek_last_sync_time')
      ? Number(localStorage.getItem('peek_last_sync_time'))
      : null,
  );
  let deviceId = $state(
    typeof window !== 'undefined' ? getOrCreateDeviceId() : 'peek-node',
  );

  async function handleExportSync() {
    if (!syncPassphrase || syncPassphrase.length < 4) {
      syncError = 'Passphrase must be at least 4 characters.';
      return;
    }
    syncError = null;
    try {
      const payload: SyncPayload = {
        version: 1,
        syncedAt: Date.now(),
        deviceId,
        bookmarks: bookmarksList,
        pinnedSites: [],
        notes: noteContent,
        tabGroups,
      };
      const token = await encryptSyncPayload(syncPassphrase, payload);
      syncBundleOutput = token;
      lastSyncedTime = Date.now();
      localStorage.setItem('peek_sync_passphrase', syncPassphrase);
      localStorage.setItem('peek_last_sync_time', String(lastSyncedTime));
      syncStatus = 'Encrypted sync token generated! (AES-256-GCM + PBKDF2)';
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(token);
        syncStatus = 'Encrypted token copied to clipboard! (AES-256-GCM)';
      }
    } catch (err: any) {
      syncError = err?.message || 'Failed to encrypt sync bundle';
    }
  }

  async function handleImportSync() {
    if (!syncPassphrase) {
      syncError = 'Passphrase is required to decrypt the sync payload.';
      return;
    }
    if (!syncBundleInput.trim()) {
      syncError = 'Please paste an encrypted sync token.';
      return;
    }
    syncError = null;
    try {
      const payload = await decryptSyncPayload(syncPassphrase, syncBundleInput.trim());

      // Restore tab groups
      if (Array.isArray(payload.tabGroups) && payload.tabGroups.length > 0) {
        send({ type: 'saveTabGroups', groups: payload.tabGroups });
        onTabGroupsChange?.(payload.tabGroups);
      }

      // Restore notes
      if (payload.notes) {
        send({ type: 'saveNote', content: payload.notes });
        onNoteChange?.(payload.notes);
      }

      // Restore bookmarks
      if (Array.isArray(payload.bookmarks)) {
        for (const bm of payload.bookmarks) {
          send({
            type: 'toggleBookmark',
            url: bm.url,
            title: bm.title || bm.url,
          });
        }
      }

      localStorage.setItem('peek_sync_passphrase', syncPassphrase);
      lastSyncedTime = Date.now();
      localStorage.setItem('peek_last_sync_time', String(lastSyncedTime));
      syncStatus = 'Synced data merged successfully! Reload or view tabs/saved.';
      syncBundleInput = '';
    } catch (err: any) {
      syncError = err?.message || 'Decryption failed. Check passphrase and token.';
    }
  }
</script>

<div class="section-title">Encrypted Cloud Sync (E2EE)</div>
<div class="card settings-card">
  <div class="sync-security-badge-row">
    <span class="security-badge">
      <svg viewBox="0 0 16 16" fill="currentColor" class="security-icon">
        <path d="M8 1a3.5 3.5 0 0 0-3.5 3.5V6H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-.5V4.5A3.5 3.5 0 0 0 8 1zm2 5H6V4.5a2 2 0 1 1 4 0V6z"/>
      </svg>
      AES-256-GCM + PBKDF2 Zero-Knowledge
    </span>
    <span class="device-id-badge" title="Local Device ID">{deviceId}</span>
  </div>
  <p class="field-desc" style="margin-top: 8px;">
    Your bookmarks, notes, pinned websites, and tab groups are encrypted on your device with your secret passphrase before leaving Peek. No server ever sees your plaintext.
  </p>

  <div class="field" style="margin-top: 12px;">
    <label for="sync-passphrase">Encryption Passphrase</label>
    <div class="input-action-row">
      <input
        id="sync-passphrase"
        type={showPassphrase ? 'text' : 'password'}
        placeholder="Enter strong passphrase..."
        bind:value={syncPassphrase}
        class="text-input"
      />
      <button
        class="secondary-btn sm"
        type="button"
        onclick={() => (showPassphrase = !showPassphrase)}
      >
        {showPassphrase ? 'Hide' : 'Show'}
      </button>
    </div>
  </div>

  {#if syncStatus}
    <div class="sync-status-msg success">{syncStatus}</div>
  {/if}
  {#if syncError}
    <div class="sync-status-msg error">{syncError}</div>
  {/if}

  <div class="sync-stats-row">
    <span class="sync-stat">★ {bookmarksList.length} Bookmarks</span>
    <span class="sync-stat">◫ {tabGroups.length} Groups</span>
    <span class="sync-stat">✎ {noteWords} Words</span>
    {#if lastSyncedTime}
      <span class="sync-stat muted">Synced {formatRelativeTime(lastSyncedTime)}</span>
    {/if}
  </div>

  <div class="group-btn-actions" style="margin-top: 14px;">
    <button class="primary-btn" onclick={handleExportSync} type="button">
      Export Encrypted Bundle
    </button>
  </div>
</div>

{#if syncBundleOutput}
  <div class="section-title" style="margin-top: 16px;">Encrypted Bundle Token</div>
  <div class="card settings-card">
    <textarea
      class="token-textarea"
      readonly
      rows="4"
      value={syncBundleOutput}
    ></textarea>
    <p class="field-desc" style="font-size: 11px;">
      Token is ready to transfer or paste into another Peek instance with the same passphrase.
    </p>
  </div>
{/if}

<div class="section-title" style="margin-top: 16px;">Import Encrypted Bundle</div>
<div class="card settings-card">
  <textarea
    class="token-textarea"
    placeholder="Paste encrypted sync token from another Peek device..."
    rows="4"
    bind:value={syncBundleInput}
  ></textarea>
  <div class="group-btn-actions" style="margin-top: 10px;">
    <button class="primary-btn" onclick={handleImportSync} type="button">
      Decrypt & Merge
    </button>
    {#if syncBundleInput}
      <button class="ghost-btn sm" onclick={() => (syncBundleInput = '')} type="button">
        Clear
      </button>
    {/if}
  </div>
</div>

<style>

  .sync-security-badge-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .security-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    font-weight: 600;
    color: #10b981;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.25);
    padding: 3px 8px;
    border-radius: 999px;
  }

  .security-icon {
    width: 12px;
    height: 12px;
  }

  .device-id-badge {
    font-family: monospace;
    font-size: 10px;
    color: var(--muted);
    background: var(--surface-2);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
  }

  .sync-stats-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border-subtle);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .sync-stat.muted {
    color: var(--muted);
  }

  .sync-status-msg {
    margin-top: 10px;
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 11.5px;
  }

  .sync-status-msg.success {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #10b981;
  }

  .sync-status-msg.error {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  .token-textarea {
    width: 100%;
    box-sizing: border-box;
    font-family: monospace;
    font-size: 10px;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px;
    resize: none;
    outline: none;
    line-height: 1.4;
  }

  .token-textarea:focus {
    border-color: var(--accent);
  }
</style>
