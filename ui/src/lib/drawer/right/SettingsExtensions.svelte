<script lang="ts">
  import { send, type ExtensionSummary } from '../../ipc';

  interface Props {
    extensions: ExtensionSummary[];
  }

  let { extensions }: Props = $props();

  let extensionInstallPath = $state('');

  function handleInstallExtension() {
    const trimmed = extensionInstallPath.trim();
    if (!trimmed) return;
    send({ type: 'installExtension', path: trimmed });
    extensionInstallPath = '';
  }

  function handleToggleExtension(id: string, current: boolean) {
    send({ type: 'toggleExtension', id, enabled: !current });
  }

  function handleUninstallExtension(id: string) {
    if (confirm('Uninstall extension?')) {
      send({ type: 'uninstallExtension', id });
    }
  }
</script>

<div class="section-title">Install Chrome Extension (MV3)</div>
<div class="card settings-card">
  <p class="field-desc" style="margin-bottom: 8px;">
    Supports Manifest V3 extensions directly from the Chrome Web Store (paste URL or 32-character ID), or provide a local .crx package or unpacked folder.
  </p>
  <div class="input-action-row">
    <input
      type="text"
      placeholder="Web Store URL, Extension ID, or /path/to/extension.crx"
      bind:value={extensionInstallPath}
      class="text-input"
    />
    <button class="primary-btn" onclick={handleInstallExtension}>Install</button>
  </div>
  <div style="margin-top: 10px; display: flex; justify-content: flex-end;">
    <button
      class="ghost-btn"
      style="font-size: 11px; padding: 4px 10px; border-radius: 6px; color: #38bdf8; display: inline-flex; align-items: center; gap: 4px;"
      onclick={() => send({ type: 'navigate', input: 'https://chromewebstore.google.com' })}
    >
      <span>Browse Chrome Web Store ↗</span>
    </button>
  </div>
</div>

<div class="section-title" style="margin-top: 18px;">
  Installed Extensions ({extensions.length})
</div>
{#if extensions.length === 0}
  <div class="card empty-card">
    <div class="empty-icon">🧩</div>
    <p class="note">
      No extensions installed yet. Enter an unpacked directory path
      above to install.
    </p>
  </div>
{:else}
  <div class="extensions-list">
    {#each extensions as ext (ext.id)}
      <div class="card ext-card">
        <div class="ext-header">
          <div>
            <div class="ext-title">
              {ext.name}
              <span class="ext-version">v{ext.version}</span>
            </div>
            <div class="ext-id">{ext.id}</div>
          </div>
          <div class="ext-controls">
            <label class="switch">
              <input
                type="checkbox"
                checked={ext.enabled}
                onchange={() => handleToggleExtension(ext.id, ext.enabled)}
              />
              <span class="slider"></span>
            </label>
            <button
              class="danger-icon-btn"
              onclick={() => handleUninstallExtension(ext.id)}
              title="Uninstall"
            >
              ✕
            </button>
          </div>
        </div>

        <div class="ext-badges">
          {#if ext.fullySupported}
            <span class="badge verified">Tier 1 Verified</span>
          {:else}
            <span class="badge partial">Partial MV3</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>

  .extensions-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ext-card {
    padding: 10px 12px;
  }

  .ext-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .ext-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }

  .ext-version {
    font-size: 10.5px;
    color: var(--muted);
    font-weight: normal;
  }

  .ext-id {
    font-size: 10px;
    color: var(--muted);
    font-family: monospace;
    margin-top: 1px;
  }

  .ext-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ext-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 7px;
  }
</style>
