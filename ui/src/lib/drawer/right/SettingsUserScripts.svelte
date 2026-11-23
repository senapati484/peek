<script lang="ts">
  import { send, type UserScriptSummary } from '../../ipc';

  interface Props {
    userScripts: UserScriptSummary[];
  }

  let { userScripts }: Props = $props();

  let isEditingScript = $state(false);
  let scriptEditorName = $state('');
  let scriptEditorCode = $state('');

  function openNewScriptEditor() {
    scriptEditorName = 'New Script';
    scriptEditorCode = `// ==UserScript==
// @name         New Custom Script
// @version      1.0
// @description  Custom userscript for Peek
// @author       You
// @match        *://*/*
// @run-at       document-end
// @grant        none
// ==/UserScript==

(function() {
  'use strict';
  console.log('[Peek UserScript] Running on:', window.location.href);
})();
`;
    isEditingScript = true;
  }

  function handleSaveScript() {
    if (!scriptEditorName.trim() || !scriptEditorCode.trim()) return;
    send({
      type: 'saveUserScript',
      name: scriptEditorName.trim(),
      code: scriptEditorCode,
    });
    isEditingScript = false;
  }

  function handleDeleteScript(id: string) {
    send({ type: 'deleteUserScript', id });
  }

  function handleToggleScript(id: string, enabled: boolean) {
    send({ type: 'toggleUserScript', id, enabled: !enabled });
  }
</script>

<div class="section-title-row">
  <div class="section-title">UserScripts (Tampermonkey Engine)</div>
  <button
    class="primary-btn sm"
    onclick={openNewScriptEditor}
    type="button"
  >
    + New Script
  </button>
</div>

{#if isEditingScript}
  <div class="card script-editor-card">
    <div class="editor-header-row">
      <span class="editor-title">Script Editor</span>
      <input
        type="text"
        placeholder="Script Name"
        bind:value={scriptEditorName}
        class="text-input sm script-name-input"
      />
    </div>
    <textarea
      class="script-editor-textarea"
      bind:value={scriptEditorCode}
      rows="14"
      spellcheck="false"
    ></textarea>
    <div class="editor-actions-row">
      <button class="primary-btn sm" onclick={handleSaveScript} type="button">Save Script</button>
      <button class="ghost-btn sm" onclick={() => (isEditingScript = false)} type="button">Cancel</button>
    </div>
  </div>
{/if}

<div class="card settings-card" style="margin-bottom: 12px;">
  <p class="field-desc">
    Peek executes lightweight <code>.user.js</code> scripts natively with zero Electron bloat. Supports <code>@match</code>, <code>@include</code>, <code>@run-at</code>, and <code>GM_*</code> APIs.
  </p>
</div>

<div class="section-title">Installed Scripts ({userScripts.length})</div>
{#if userScripts.length === 0}
  <div class="card empty-card">
    <div class="empty-icon">📜</div>
    <p class="note">No userscripts installed yet. Click "+ New Script" above or drop a <code>.user.js</code> file into your profile directory.</p>
  </div>
{:else}
  <div class="scripts-list">
    {#each userScripts as script (script.id)}
      <div class="card script-card">
        <div class="script-header">
          <div>
            <div class="script-title">
              {script.name}
              <span class="script-version">v{script.version}</span>
            </div>
            {#if script.description}
              <div class="script-desc">{script.description}</div>
            {/if}
          </div>
          <div class="script-controls">
            <label class="switch">
              <input
                type="checkbox"
                checked={script.enabled}
                onchange={() => handleToggleScript(script.id, script.enabled)}
              />
              <span class="slider"></span>
            </label>
            <button
              class="danger-icon-btn"
              onclick={() => handleDeleteScript(script.id)}
              title="Delete script"
              type="button"
            >
              ✕
            </button>
          </div>
        </div>

        <div class="script-meta-badges">
          <span class="badge run-at">{script.runAt}</span>
          {#each script.matchPatterns.slice(0, 3) as pattern}
            <span class="badge pattern">{pattern}</span>
          {/each}
          {#if script.matchPatterns.length > 3}
            <span class="badge pattern">+{script.matchPatterns.length - 3} more</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>

  .section-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .script-editor-card {
    margin-bottom: 14px;
    background: var(--surface-2);
    border: 1px solid var(--accent);
  }

  .editor-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .editor-title {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--accent);
  }

  .script-name-input {
    max-width: 180px;
  }

  .script-editor-textarea {
    width: 100%;
    box-sizing: border-box;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11.5px;
    line-height: 1.45;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px;
    resize: vertical;
    outline: none;
  }

  .script-editor-textarea:focus {
    border-color: var(--accent);
  }

  .editor-actions-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .scripts-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .script-card {
    padding: 10px 12px;
  }

  .script-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }

  .script-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }

  .script-version {
    font-size: 10.5px;
    color: var(--muted);
    font-weight: normal;
  }

  .script-desc {
    font-size: 11px;
    color: var(--muted);
    margin-top: 2px;
  }

  .script-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .script-meta-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }

  .badge.run-at {
    background: var(--surface-2);
    color: var(--muted);
    border: 1px solid var(--border-subtle);
  }

  .badge.pattern {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
    font-family: monospace;
    font-size: 9px;
  }
</style>
