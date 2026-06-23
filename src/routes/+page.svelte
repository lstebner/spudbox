<script lang="ts">
  import AlbumGrid from "$lib/components/browser/AlbumGrid.svelte";
  import TrackTable from "$lib/components/tracklist/TrackTable.svelte";
  import { library } from "$lib/stores/library.svelte";
</script>

<div class="page">
  {#if library.selectedAlbumId !== null}
    <TrackTable />
  {:else}
    <AlbumGrid />
  {/if}
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
  }
</style>
