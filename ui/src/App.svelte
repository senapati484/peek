<script lang="ts">
  import Drawer from './lib/Drawer.svelte';
  import Omnibar from './lib/Omnibar.svelte';

  const route = window.location.hash.replace('#', '') || 'bar';
</script>

{#if route === 'drawer'}
  <Drawer />
{:else}
  <Omnibar />
{/if}
