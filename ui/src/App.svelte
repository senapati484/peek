<script lang="ts">
  import Drawer from './lib/Drawer.svelte';
  import Omnibar from './lib/Omnibar.svelte';

  const rawHash = window.location.hash.replace('#', '');
  const route = rawHash.split('?')[0] || 'bar';
</script>

{#if route === 'drawer'}
  <Drawer />
{:else}
  <Omnibar />
{/if}
