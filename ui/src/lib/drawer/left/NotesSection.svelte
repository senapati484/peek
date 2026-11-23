<script lang="ts">
  import { isBlank, send, type TabInfo } from '../../ipc';

  interface Props {
    noteContent: string;
    activeTab: TabInfo | undefined;
    onSaveNote?: (content: string) => void;
  }

  let { noteContent = $bindable(''), activeTab, onSaveNote }: Props = $props();

  let copiedNotes = $state(false);
  let noteSaveTimer: ReturnType<typeof setTimeout> | undefined;

  let noteWords = $derived(
    noteContent.trim()
      ? noteContent.trim().split(/\s+/).filter(Boolean).length
      : 0,
  );

  function onNoteInput(event: Event) {
    const val = (event.target as HTMLTextAreaElement).value;
    noteContent = val;
    if (noteSaveTimer) clearTimeout(noteSaveTimer);
    noteSaveTimer = setTimeout(() => {
      send({ type: 'saveNote', content: noteContent });
      onSaveNote?.(noteContent);
    }, 400);
  }

  function insertCurrentTabLink() {
    if (!activeTab || isBlank(activeTab.url)) return;
    const title = activeTab.title || activeTab.url;
    const mdLink = `\n- [${title}](${activeTab.url})\n`;
    noteContent += mdLink;
    send({ type: 'saveNote', content: noteContent });
    onSaveNote?.(noteContent);
  }

  function copyNotes() {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(noteContent);
      copiedNotes = true;
      setTimeout(() => {
        copiedNotes = false;
      }, 1800);
    }
  }
</script>

<div class="scratchpad-header">
  <div class="scratchpad-meta">
    <span class="scratchpad-badge">RESEARCH SCRATCHPAD</span>
    <span class="scratchpad-count">{noteWords} words · {noteContent.length} chars</span>
  </div>
  <div class="scratchpad-actions">
    {#if activeTab && !isBlank(activeTab.url)}
      <button
        class="scratch-action-btn"
        title="Insert link to active tab"
        type="button"
        onclick={insertCurrentTabLink}
      >
        + Link Tab
      </button>
    {/if}
    <button
      class="scratch-action-btn"
      title="Copy all notes to clipboard"
      type="button"
      onclick={copyNotes}
    >
      {copiedNotes ? '✓ Copied' : 'Copy'}
    </button>
    <button
      class="scratch-action-btn danger"
      title="Clear notes"
      type="button"
      onclick={() => {
        if (confirm('Clear scratchpad notes?')) {
          noteContent = '';
          send({ type: 'saveNote', content: '' });
          onSaveNote?.('');
        }
      }}
    >
      Clear
    </button>
  </div>
</div>

<div class="body scratchpad-body">
  <textarea
    class="scratchpad-editor"
    placeholder="Jot down quick thoughts, research findings, and task notes here...&#10;&#10;• Autosaved locally to your SQLite database.&#10;• Use '+ Link Tab' to insert current page as markdown.&#10;• Copy out to Notion, Slack, or Obsidian."
    bind:value={noteContent}
    oninput={onNoteInput}
    spellcheck="false"
  ></textarea>
</div>

<style>
  .scratchpad-header {
    padding: 12px 14px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-subtle);
    gap: 8px;
  }

  .scratchpad-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .scratchpad-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--accent);
  }

  .scratchpad-count {
    font-size: 11px;
    color: var(--muted);
  }

  .scratchpad-actions {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .scratch-action-btn {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    transition: all 0.12s ease;
  }

  .scratch-action-btn:hover {
    background: var(--hover);
  }

  .scratch-action-btn.danger:hover {
    color: #ef4444;
    border-color: #ef4444;
  }

  .body.scratchpad-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 10px;
    box-sizing: border-box;
    overflow: hidden;
  }

  .scratchpad-editor {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    flex: 1;
    background: transparent;
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12.5px;
    line-height: 1.6;
    resize: none;
    padding: 8px;
    border-radius: 8px;
    outline: none;
  }

  .scratchpad-editor::placeholder {
    color: var(--muted);
    opacity: 0.65;
    font-family: inherit;
  }
</style>
