<script lang="ts">
  type ThemeMode = 'system' | 'dark' | 'light';
  type AccentColor = 'blue' | 'purple' | 'emerald' | 'amber' | 'rose';

  interface Props {
    themeMode: ThemeMode;
    accentColor: AccentColor;
    onApplyTheme: (mode: ThemeMode) => void;
    onApplyAccent: (accent: AccentColor) => void;
  }

  let {
    themeMode,
    accentColor,
    onApplyTheme,
    onApplyAccent,
  }: Props = $props();
</script>

<div class="section-title">Color Scheme</div>
<div class="theme-options">
  <button
    class="choice-btn"
    class:selected={themeMode === 'system'}
    onclick={() => onApplyTheme('system')}
  >
    <span class="theme-icon">◐</span>
    Auto System
  </button>
  <button
    class="choice-btn"
    class:selected={themeMode === 'dark'}
    onclick={() => onApplyTheme('dark')}
  >
    <span class="theme-icon">☾</span>
    Dark Mode
  </button>
  <button
    class="choice-btn"
    class:selected={themeMode === 'light'}
    onclick={() => onApplyTheme('light')}
  >
    <span class="theme-icon">☼</span>
    Light Mode
  </button>
</div>

<div class="section-title" style="margin-top: 20px;">Accent Hue</div>
<div class="card settings-card">
  <div class="accent-palette">
    <button
      class="accent-dot"
      class:selected={accentColor === 'blue'}
      style="background: #3b82f6;"
      onclick={() => onApplyAccent('blue')}
      title="Electric Blue"
    ></button>
    <button
      class="accent-dot"
      class:selected={accentColor === 'purple'}
      style="background: #8b5cf6;"
      onclick={() => onApplyAccent('purple')}
      title="Neon Violet"
    ></button>
    <button
      class="accent-dot"
      class:selected={accentColor === 'emerald'}
      style="background: #10b981;"
      onclick={() => onApplyAccent('emerald')}
      title="Emerald Green"
    ></button>
    <button
      class="accent-dot"
      class:selected={accentColor === 'amber'}
      style="background: #f59e0b;"
      onclick={() => onApplyAccent('amber')}
      title="Warm Amber"
    ></button>
    <button
      class="accent-dot"
      class:selected={accentColor === 'rose'}
      style="background: #f43f5e;"
      onclick={() => onApplyAccent('rose')}
      title="Rose Quartz"
    ></button>
  </div>
</div>

<style>

  .theme-options {
    display: flex;
    gap: 8px;
  }

  .choice-btn {
    all: unset;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 11.5px;
    color: var(--text);
    background: var(--surface);
    cursor: pointer;
    box-sizing: border-box;
    transition: all 0.15s ease;
  }

  .choice-btn.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .theme-icon {
    font-size: 15px;
  }

  .accent-palette {
    display: flex;
    gap: 12px;
    padding: 4px 0;
  }

  .accent-dot {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .accent-dot:hover {
    transform: scale(1.15);
  }

  .accent-dot.selected {
    border-color: var(--text);
    transform: scale(1.12);
  }
</style>
