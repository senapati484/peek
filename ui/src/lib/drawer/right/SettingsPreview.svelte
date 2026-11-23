<script lang="ts">
  interface Props {
    previewModifierOnly: boolean;
    hoverDelay: number;
    previewWidth: number;
    previewHeight: number;
    previewZoom: number;
    onSetting: (key: string, value: unknown) => void;
  }

  let {
    previewModifierOnly = $bindable(true),
    hoverDelay = $bindable(150),
    previewWidth = $bindable(480),
    previewHeight = $bindable(320),
    previewZoom = $bindable(0.75),
    onSetting,
  }: Props = $props();
</script>

<div class="section-title">Hover Preview & Link QuickLook</div>
<div class="card settings-card">
  <div
    class="field"
    style="background: var(--surface-2); border-radius: 8px; padding: 10px 12px; margin-bottom: 8px;"
  >
    <div>
      <span class="field-label-bold" style="color: var(--accent);">✦ Space to Peek (Finder QuickLook)</span>
      <p class="field-desc">
        Hover over any link and press <kbd class="preview-kbd">Space</kbd>
        or hold <kbd class="preview-kbd">Shift</kbd> to pop open an instant live card preview.
      </p>
    </div>
  </div>

  <div class="field switch-field">
    <div>
      <span class="field-label-bold">Require Modifier Key</span>
      <p class="field-desc">
        Require ⌘ or Shift key to trigger preview on hover
      </p>
    </div>
    <label class="switch">
      <input
        type="checkbox"
        checked={previewModifierOnly}
        onchange={(e) => {
          previewModifierOnly = (e.currentTarget as HTMLInputElement).checked;
          onSetting('previewModifierOnly', previewModifierOnly);
        }}
      />
      <span class="slider"></span>
    </label>
  </div>

  <div class="field">
    <label for="h-delay">Hover Delay (ms)</label>
    <input
      id="h-delay"
      type="number"
      min="50"
      max="1500"
      step="50"
      value={hoverDelay}
      onchange={(e) => {
        hoverDelay = Number((e.currentTarget as HTMLInputElement).value);
        onSetting('hoverDelayMs', hoverDelay);
      }}
    />
  </div>

  <div class="field">
    <label for="p-width">Preview Width (px)</label>
    <input
      id="p-width"
      type="number"
      min="320"
      max="800"
      step="20"
      value={previewWidth}
      onchange={(e) => {
        previewWidth = Number((e.currentTarget as HTMLInputElement).value);
        onSetting('previewWidth', previewWidth);
      }}
    />
  </div>

  <div class="field">
    <label for="p-height">Preview Height (px)</label>
    <input
      id="p-height"
      type="number"
      min="200"
      max="600"
      step="20"
      value={previewHeight}
      onchange={(e) => {
        previewHeight = Number((e.currentTarget as HTMLInputElement).value);
        onSetting('previewHeight', previewHeight);
      }}
    />
  </div>

  <div class="field" style="border-bottom: none;">
    <label for="p-zoom">Preview Zoom Ratio</label>
    <input
      id="p-zoom"
      type="number"
      min="0.5"
      max="1.0"
      step="0.05"
      value={previewZoom}
      onchange={(e) => {
        previewZoom = Number((e.currentTarget as HTMLInputElement).value);
        onSetting('previewZoom', previewZoom);
      }}
    />
  </div>
</div>

<style>

  .preview-kbd {
    background: var(--surface);
    padding: 1px 4px;
    border-radius: 3px;
    border: 1px solid var(--border);
    font-size: 11px;
    font-family: inherit;
  }
</style>
