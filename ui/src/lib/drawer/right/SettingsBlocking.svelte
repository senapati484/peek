<script lang="ts">
  import { send } from '../../ipc';

  interface Props {
    blocked: number;
    host: string | null;
    allowedOnSite: boolean;
    cookiesCount: number;
  }

  let {
    blocked,
    host,
    allowedOnSite,
    cookiesCount,
  }: Props = $props();
</script>

<div class="card stat-card">
  <div class="stat-big">{blocked}</div>
  <div class="stat-label">Trackers & Ads Blocked This Session</div>
</div>

{#if host}
  <div class="card settings-card" style="margin-top: 14px;">
    <div class="field switch-field" style="border-bottom: none;">
      <div>
        <span class="field-label-bold">Protection on {host}</span>
        <p class="field-desc">
          Adblock and tracker protection on current site
        </p>
      </div>
      <label class="switch">
        <input
          type="checkbox"
          checked={!allowedOnSite}
          onchange={() => {
            if (host)
              send({
                type: 'setSiteAdblock',
                host,
                enabled: allowedOnSite,
              });
          }}
        />
        <span class="slider"></span>
      </label>
    </div>
  </div>
{/if}

<div class="section-title" style="margin-top: 18px;">
  Built-in Privacy Engine
</div>
<div class="card settings-card">
  <div class="filter-item">
    <div class="filter-status-dot"></div>
    <div>
      <div class="filter-name">EasyList & EasyPrivacy</div>
      <div class="filter-desc">
        Network-level blocking & cosmetic filtering
      </div>
    </div>
  </div>
  <div class="filter-item">
    <div class="filter-status-dot"></div>
    <div>
      <div class="filter-name">uBlock Origin Filters</div>
      <div class="filter-desc">
        Anti-malware & behavioral telemetry prevention
      </div>
    </div>
  </div>
</div>

<div class="card settings-card" style="margin-top: 14px;">
  <div class="field" style="border-bottom: none;">
    <div>
      <span class="field-label-bold">Cookies & Local Storage</span>
      <p class="field-desc">{cookiesCount} stored domain cookies</p>
    </div>
    <button
      class="danger-btn"
      onclick={() => send({ type: 'clearCookies' })}
    >
      Clear Data
    </button>
  </div>
</div>

<style>

  .filter-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .filter-item:last-child {
    border-bottom: none;
  }

  .filter-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
  }

  .filter-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }

  .filter-desc {
    font-size: 10.5px;
    color: var(--muted);
  }
</style>
