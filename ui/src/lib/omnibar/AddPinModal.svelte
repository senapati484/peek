<script lang="ts">
  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: (url: string, title?: string) => void;
  }

  let { isOpen, onClose, onSubmit }: Props = $props();

  let pinUrl = $state('');
  let pinTitle = $state('');

  function handleSubmit() {
    const trimmedUrl = pinUrl.trim();
    if (!trimmedUrl) return;
    onSubmit(trimmedUrl, pinTitle.trim() || undefined);
    pinUrl = '';
    pinTitle = '';
    onClose();
  }
</script>

{#if isOpen}
  <div
    class="pin-modal-backdrop"
    onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    role="presentation"
  >
    <div
      class="pin-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      tabindex="-1"
      onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
    >
      <div class="modal-header">
        <h3 id="modal-title" class="modal-title">Pin Website</h3>
        <button class="modal-close-btn" type="button" onclick={onClose}>×</button>
      </div>
      <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="modal-form">
        <label class="modal-label">
          <span>Website URL</span>
          <input
            type="text"
            placeholder="https://example.com"
            bind:value={pinUrl}
            class="modal-input"
            required
          />
        </label>
        <label class="modal-label">
          <span>Display Name (optional)</span>
          <input
            type="text"
            placeholder="Example"
            bind:value={pinTitle}
            class="modal-input"
          />
        </label>
        <div class="modal-actions">
          <button type="button" class="btn secondary" onclick={onClose}>Cancel</button>
          <button type="submit" class="btn primary">Pin Website</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .pin-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }

  .pin-modal {
    width: 380px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px;
    box-shadow: var(--shadow-lg);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }

  .modal-close-btn {
    all: unset;
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    color: var(--muted);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .modal-close-btn:hover {
    color: var(--text);
    background: var(--hover);
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .modal-input {
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--text);
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .modal-input:focus {
    border-color: var(--accent);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .btn {
    all: unset;
    cursor: pointer;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12.5px;
    font-weight: 550;
    transition: all 0.15s ease;
  }

  .btn.secondary {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .btn.secondary:hover {
    background: var(--hover);
    color: var(--text);
  }

  .btn.primary {
    background: var(--accent);
    color: #ffffff;
  }

  .btn.primary:hover {
    filter: brightness(1.1);
  }
</style>
