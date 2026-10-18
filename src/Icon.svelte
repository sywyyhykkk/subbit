<script lang="ts">
  let { name, size = 18 }: { name: 'plus' | 'settings' | 'search' | 'chevron' | 'close' | 'arrow' | 'back' | 'refresh' | 'download' | 'upload' | 'infinity' | 'calendar' | 'globe' | 'chart'; size?: number } = $props();
  const paths = {
    chart: "M4 3v18h17M8 16v-5m5 5V6m5 10V9",
    globe: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0M3 12h18M12 3c-5 5-5 13 0 18 5-5 5-13 0-18',
    calendar: 'M5 4h14v17H5zM8 2v4m8-4v4M5 9h14M8 13h2m4 0h2m-8 4h2m4 0h2',
    plus: 'M12 5v14M5 12h14',
    search: 'm21 21-4.5-4.5M19 10.5a8.5 8.5 0 1 1-17 0 8.5 8.5 0 0 1 17 0',
    chevron: 'm9 5 7 7-7 7',
    close: 'm6 6 12 12M6 18 18 6',
    arrow: 'M7 17 17 7M7 7h10v10',
    back: 'M19 12H5m6-6-6 6 6 6',
    refresh: 'M20 7v5h-5M4 17v-5h5M6.1 7a7 7 0 0 1 11.6-2L20 8M4 16l2.3 3A7 7 0 0 0 17.9 17',
    download: 'M12 3v12m-5-5 5 5 5-5M5 16v5h14v-5',
    upload: 'M12 16V4m-5 5 5-5 5 5M5 16v5h14v-5',
    infinity: 'M12 12c-3-5-8-6-9-2s3 7 9 2c6-5 10-2 9 2s-6 3-9-2',
  };
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false" class="ui-icon">
  {#if name === 'settings'}
    <path d="m9.5 3-.6 2.3-2 1.2-2.3-.6-2.5 4.2L3.8 12l-1.7 1.9 2.5 4.2 2.3-.6 2 1.2.6 2.3h5l.6-2.3 2-1.2 2.3.6 2.5-4.2L20.2 12l1.7-1.9-2.5-4.2-2.3.6-2-1.2-.6-2.3z"/><circle cx="12" cy="12" r="3"/>
  {:else}<path d={paths[name]}/>{/if}
</svg>
