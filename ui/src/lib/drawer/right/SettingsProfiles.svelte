<script lang="ts">
  import { send } from '../../ipc';

  interface Props {
    profiles: string[];
    activeProfile: string;
  }

  let { profiles, activeProfile }: Props = $props();

  let newProfileName = $state('');

  function handleCreateProfile() {
    const trimmed = newProfileName.trim();
    if (!trimmed) return;
    send({ type: 'createProfile', name: trimmed });
    newProfileName = '';
  }

  function handleSwitchProfile(name: string) {
    if (name === activeProfile) return;
    send({ type: 'switchProfile', name });
  }

  function handleDeleteProfile(name: string) {
    if (name === 'default' || name === activeProfile) return;
    if (confirm(`Delete profile "${name}" and all its data?`)) {
      send({ type: 'deleteProfile', name });
    }
  }
</script>

<div class="section-title">Active Profiles</div>
<ul class="profile-list">
  {#each profiles as p}
    <li class="profile-item card" class:active-profile={p === activeProfile}>
      <div class="profile-info">
        <div class="profile-avatar">{p[0].toUpperCase()}</div>
        <div class="profile-meta">
          <div class="profile-name-row">
            <span class="profile-name">{p}</span>
            {#if p === activeProfile}
              <span class="profile-badge">Active</span>
            {/if}
          </div>
          <span class="profile-subtext">
            {p === activeProfile
              ? 'Current workspace profile'
              : 'Isolated browsing profile'}
          </span>
        </div>
      </div>
      <div class="profile-actions">
        {#if p !== activeProfile}
          <button
            class="switch-profile-btn"
            onclick={() => handleSwitchProfile(p)}
          >
            Switch
          </button>
          <button
            class="delete-profile-btn"
            onclick={() => handleDeleteProfile(p)}
            title="Delete profile"
          >
            <svg
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              class="action-icon"
            >
              <path
                d="M3 4.5h10M5.5 4.5V3a1 1 0 011-1h3a1 1 0 011 1v1.5M6 7.5v5M10 7.5v5M4.5 4.5l.7 9a1.5 1.5 0 001.5 1.4h4.6a1.5 1.5 0 001.5-1.4l.7-9"
                stroke-width="1.3"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        {/if}
      </div>
    </li>
  {/each}
</ul>

<div class="card create-profile-card">
  <div class="create-profile-header">
    <span class="create-title">Create New Profile</span>
    <span class="create-desc">Isolated cookies, history, tabs, and storage</span>
  </div>
  <div class="create-profile-input-wrap">
    <input
      type="text"
      placeholder="Profile name (e.g. Work, Research)"
      bind:value={newProfileName}
      onkeydown={(e) => {
        if (e.key === 'Enter') handleCreateProfile();
      }}
      class="create-profile-input"
    />
    <button class="create-profile-btn" onclick={handleCreateProfile}>
      Create
    </button>
  </div>
</div>

<style>

  .profile-list {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 13px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    transition: all 0.15s ease;
  }

  .profile-item.active-profile {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }

  .profile-avatar {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: linear-gradient(
      135deg,
      var(--accent),
      color-mix(in srgb, var(--accent) 55%, #000)
    );
    color: #ffffff;
    font-weight: 700;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }

  .profile-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .profile-name-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .profile-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .profile-badge {
    font-size: 9.5px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    padding: 1px 7px;
    border-radius: 999px;
    font-weight: 650;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .profile-subtext {
    font-size: 11px;
    color: var(--muted);
  }

  .profile-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .switch-profile-btn {
    all: unset;
    padding: 5px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .switch-profile-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #ffffff;
  }

  .delete-profile-btn {
    all: unset;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    background: transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-profile-btn:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }

  .action-icon {
    width: 14px;
    height: 14px;
  }

  .create-profile-card {
    padding: 14px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    margin-top: 10px;
  }

  .create-profile-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 10px;
  }

  .create-title {
    font-size: 12px;
    font-weight: 650;
    color: var(--text);
  }

  .create-desc {
    font-size: 10.5px;
    color: var(--muted);
  }

  .create-profile-input-wrap {
    display: flex;
    gap: 8px;
  }

  .create-profile-input {
    flex: 1;
    padding: 7px 11px;
    border-radius: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 12px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .create-profile-input:focus {
    border-color: var(--accent);
  }

  .create-profile-btn {
    all: unset;
    padding: 7px 14px;
    border-radius: 8px;
    background: var(--accent);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s ease;
  }

  .create-profile-btn:hover {
    filter: brightness(1.1);
  }
</style>
