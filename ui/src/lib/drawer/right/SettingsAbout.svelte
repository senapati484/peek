<script lang="ts">
  import { send } from '../../ipc';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();
</script>

<div class="about-hero">
  <img src="/logo.png" alt="Peek" class="about-logo" />
  <h2 class="about-name">Peek Browser</h2>
  <span class="about-tagline">v0.1.0 • Pure Rust Native Architecture</span>
</div>

<!-- Star on GitHub & Community Card -->
<div class="card settings-card github-star-card">
  <div class="github-card-header">
    <div class="github-badge-row">
      <svg class="github-icon" viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
      </svg>
      <span class="github-title">Open Source on GitHub</span>
    </div>
    <span class="github-stars-badge">★ Star Peek</span>
  </div>
  <p class="github-card-desc">
    If you're enjoying Peek's ultra-minimalist native workflow, giving it a star on GitHub helps the project grow and supports active development!
  </p>
  <div class="github-actions">
    <button
      class="btn-star-repo"
      type="button"
      onclick={() => {
        send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
        onClose();
      }}
      title="Star peek on GitHub"
    >
      <span class="star-sparkle">⭐</span>
      <span>Star peek on GitHub</span>
      <span class="out-arrow">↗</span>
    </button>
    <button
      class="btn-repo-issues"
      type="button"
      onclick={() => {
        send({ type: 'openUrl', url: 'https://github.com/senapati484/peek/issues' });
        onClose();
      }}
      title="Report an issue or suggest a feature on GitHub"
    >
      <span>Feedback & Issues</span>
      <span class="out-arrow">↗</span>
    </button>
  </div>
</div>

<div class="card settings-card" style="margin-top: 12px;">
  <p class="note">
    Peek is built with zero framework bloat using direct <code>tao</code> and <code>wry</code> bindings, native system WebViews, Brave adblock engine, and an ephemeral hover preview architecture.
  </p>
</div>

<div class="card settings-card" style="margin-top: 12px;">
  <div class="tech-row"><span>Engine</span> <strong>tao + wry (WebKit/WebView2)</strong></div>
  <div class="tech-row"><span>Database</span> <strong>SQLite FTS5 + Connection Pool</strong></div>
  <div class="tech-row"><span>Adblock</span> <strong>Brave Adblock Engine (Rust)</strong></div>
  <div class="tech-row"><span>Extensions</span> <strong>Manifest V3 Host Engine</strong></div>
  <div class="tech-row"><span>License</span> <strong>GNU General Public License v3.0 (GPLv3)</strong></div>
  <div class="tech-row">
    <span>Repository</span>
    <button
      class="repo-inline-link"
      type="button"
      onclick={() => {
        send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
        onClose();
      }}
    >
      github.com/senapati484/peek ↗
    </button>
  </div>
</div>

<style>

  .about-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    margin: 16px 0 18px;
    text-align: center;
  }

  .about-logo {
    width: 48px;
    height: 48px;
    display: block;
    margin-bottom: 8px;
    filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.25));
  }

  .about-name {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    letter-spacing: -0.02em;
  }

  .about-tagline {
    font-size: 11.5px;
    color: var(--muted);
    margin-top: 3px;
  }

  .github-star-card {
    background: linear-gradient(145deg, var(--surface), color-mix(in srgb, var(--accent) 6%, var(--surface)));
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
    padding: 14px 16px;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
  }

  .github-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .github-badge-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .github-icon {
    color: var(--text);
  }

  .github-title {
    font-size: 13px;
    font-weight: 650;
    color: var(--text);
  }

  .github-stars-badge {
    font-size: 10px;
    font-weight: 700;
    color: #eab308;
    background: rgba(234, 179, 8, 0.14);
    border: 1px solid rgba(234, 179, 8, 0.28);
    padding: 2px 7px;
    border-radius: 999px;
    letter-spacing: 0.02em;
  }

  .github-card-desc {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    margin: 0 0 12px;
  }

  .github-actions {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .btn-star-repo {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--accent);
    color: #ffffff;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 2px 8px var(--accent-glow);
    transition: all 0.16s ease;
    box-sizing: border-box;
  }

  .btn-star-repo:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
    box-shadow: 0 4px 14px var(--accent-glow);
  }

  .star-sparkle {
    font-size: 13px;
  }

  .out-arrow {
    font-size: 12px;
    opacity: 0.8;
  }

  .btn-repo-issues {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: 6px 12px;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.14s ease;
    box-sizing: border-box;
  }

  .btn-repo-issues:hover {
    background: var(--hover);
    color: var(--text);
  }

  .tech-row {
    display: flex;
    justify-content: space-between;
    font-size: 11.5px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tech-row:last-child {
    border-bottom: none;
  }

  .tech-row span {
    color: var(--muted);
  }

  .repo-inline-link {
    all: unset;
    color: var(--accent);
    cursor: pointer;
    font-size: 11.5px;
    font-weight: 550;
    transition: text-decoration 0.12s ease;
  }

  .repo-inline-link:hover {
    text-decoration: underline;
  }
</style>
