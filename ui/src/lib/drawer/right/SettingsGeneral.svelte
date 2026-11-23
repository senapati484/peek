<script lang="ts">
  import type { EdgeSide } from '../../ipc';

  interface Props {
    searchEngine: string;
    configuredDrawerSide: EdgeSide;
    edgeZone: number;
    startBlank: boolean;
    maxLive: number;
    idleMinutes: number;
    onSetting: (key: string, value: unknown) => void;
  }

  let {
    searchEngine,
    configuredDrawerSide,
    edgeZone,
    startBlank,
    maxLive,
    idleMinutes,
    onSetting,
  }: Props = $props();
</script>

<div class="section-title">Search & Browsing</div>
<div class="card settings-card">
  <div class="field">
    <label for="search-eng">Default Search Engine</label>
    <select
      id="search-eng"
      value={searchEngine}
      onchange={(e) =>
        onSetting(
          'searchEngine',
          (e.currentTarget as HTMLSelectElement).value,
        )}
    >
      <option value="Google">Google</option>
      <option value="DuckDuckGo">DuckDuckGo</option>
      <option value="Brave">Brave Search</option>
      <option value="Bing">Bing</option>
      <option value="Ecosia">Ecosia</option>
    </select>
  </div>

  <div class="field">
    <label for="side">Default Drawer Side</label>
    <select
      id="side"
      value={configuredDrawerSide}
      onchange={(e) =>
        onSetting(
          'drawerSide',
          (e.currentTarget as HTMLSelectElement).value,
        )}
    >
      <option value="right">Right</option>
      <option value="left">Left</option>
    </select>
  </div>

  <div class="field">
    <label for="edge">Edge Hover Zone (px)</label>
    <input
      id="edge"
      type="number"
      min="1"
      max="16"
      value={edgeZone}
      onchange={(e) =>
        onSetting(
          'edgeZone',
          Number((e.currentTarget as HTMLInputElement).value),
        )}
    />
  </div>

  <div class="field switch-field">
    <span>Always open new tabs with Start Page</span>
    <label class="switch">
      <input
        type="checkbox"
        checked={startBlank}
        onchange={(e) =>
          onSetting(
            'startBlank',
            (e.currentTarget as HTMLInputElement).checked,
          )}
      />
      <span class="slider"></span>
    </label>
  </div>
</div>

<div class="section-title" style="margin-top: 18px;">
  Tab & Memory Budget
</div>
<div class="card settings-card">
  <div class="field">
    <div>
      <label for="max-live">Max Live Tabs (LRU)</label>
      <p class="field-desc">
        Background tabs beyond this count are discarded
      </p>
    </div>
    <input
      id="max-live"
      type="number"
      min="1"
      max="20"
      value={maxLive}
      onchange={(e) =>
        onSetting(
          'maxLive',
          Number((e.currentTarget as HTMLInputElement).value),
        )}
    />
  </div>

  <div class="field" style="border-bottom: none;">
    <div>
      <label for="idle-time">Idle Discard (minutes)</label>
      <p class="field-desc">
        Inactive background tabs are put to sleep
      </p>
    </div>
    <input
      id="idle-time"
      type="number"
      min="1"
      max="120"
      value={idleMinutes}
      onchange={(e) =>
        onSetting(
          'idleDiscardMinutes',
          Number((e.currentTarget as HTMLInputElement).value),
        )}
    />
  </div>
</div>

<style>
  /* Local styles if needed */
</style>
