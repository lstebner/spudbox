<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  type TrackRow = {
    id: number;
    title: string;
    artist: string;
    album: string;
    duration_ms: number;
    sample_rate: number | null;
    bit_depth: number | null;
    channels: number | null;
    codec: string | null;
    track_no: number | null;
  };

  type ScanStats = {
    scanned: number;
    added: number;
    updated: number;
    unchanged: number;
    removed: number;
    errors: number;
  };

  let pingResult = $state("checking backend...");
  let libraryPath = $state("/home/luke/Dropbox/Music/CD Rips");
  let scanning = $state(false);
  let scanProgress = $state({ scanned: 0, total: 0 });
  let scanResult = $state<ScanStats | null>(null);
  let scanElapsedMs = $state(0);
  let tracks = $state<TrackRow[]>([]);
  let scanError = $state("");

  async function checkBackend() {
    try {
      pingResult = await invoke("ping");
    } catch (e) {
      pingResult = `error: ${e}`;
    }
  }

  function formatDuration(ms: number): string {
    const totalSeconds = Math.round(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  async function scanLibrary() {
    scanning = true;
    scanError = "";
    scanResult = null;
    scanProgress = { scanned: 0, total: 0 };
    const start = performance.now();
    try {
      await invoke("library_add_root", { path: libraryPath });
      scanResult = await invoke<ScanStats>("library_scan");
      scanElapsedMs = performance.now() - start;
      tracks = await invoke<TrackRow[]>("library_get_tracks");
    } catch (e) {
      scanError = String(e);
    } finally {
      scanning = false;
    }
  }

  checkBackend();
  listen<{ scanned: number; total: number }>("scan-progress", (event) => {
    scanProgress = event.payload;
  });
</script>

<main class="container">
  <h1>Music Player — Debug</h1>
  <p>Backend status: {pingResult}</p>

  <div class="scan-controls">
    <input bind:value={libraryPath} placeholder="Library path" />
    <button onclick={scanLibrary} disabled={scanning}>
      {scanning ? "Scanning..." : "Add Root & Scan"}
    </button>
  </div>

  {#if scanning}
    <p>Progress: {scanProgress.scanned} / {scanProgress.total}</p>
  {/if}

  {#if scanError}
    <p class="error">{scanError}</p>
  {/if}

  {#if scanResult}
    <p>
      Scanned {scanResult.scanned} files in {(scanElapsedMs / 1000).toFixed(2)}s — added {scanResult.added}, updated {scanResult.updated},
      unchanged {scanResult.unchanged}, removed {scanResult.removed}, errors {scanResult.errors}
    </p>
  {/if}

  {#if tracks.length > 0}
    <table>
      <thead>
        <tr>
          <th>#</th>
          <th>Title</th>
          <th>Artist</th>
          <th>Album</th>
          <th>Duration</th>
          <th>Codec</th>
          <th>Sample rate</th>
          <th>Bit depth</th>
          <th>Channels</th>
        </tr>
      </thead>
      <tbody>
        {#each tracks as t (t.id)}
          <tr>
            <td>{t.track_no ?? ""}</td>
            <td>{t.title}</td>
            <td>{t.artist}</td>
            <td>{t.album}</td>
            <td>{formatDuration(t.duration_ms)}</td>
            <td>{t.codec ?? ""}</td>
            <td>{t.sample_rate ?? ""}</td>
            <td>{t.bit_depth ?? ""}</td>
            <td>{t.channels ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  .container {
    margin: 0;
    padding: 2vh 2vw;
  }

  .scan-controls {
    display: flex;
    gap: 0.5em;
    margin: 1em 0;
  }

  .scan-controls input {
    flex: 1;
    padding: 0.4em;
  }

  .error {
    color: #c0392b;
  }

  table {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.85em;
  }

  th,
  td {
    border-bottom: 1px solid #ddd;
    padding: 0.25em 0.5em;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 20em;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }

    th,
    td {
      border-bottom-color: #444;
    }
  }
</style>
