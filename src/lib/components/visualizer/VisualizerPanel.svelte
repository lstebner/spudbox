<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { onVisualizerData } from "$lib/api/events";
  import { player } from "$lib/stores/player.svelte";

  const TRANSITION_DURATION_MILLISECONDS = 200;
  const NUM_BANDS = 64;

  let { onclose }: { onclose: () => void } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();

  // Current smoothed bands used by the rAF loop, interpolating toward target.
  let currentBands = new Float32Array(NUM_BANDS);
  let targetBands = new Float32Array(NUM_BANDS);
  let animationFrameId = 0;
  let accentColor = '#818cf8';

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onclose();
  }

  function drawFrame() {
    animationFrameId = requestAnimationFrame(drawFrame);
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const cx = width / 2;
    const cy = height / 2;

    // Art geometry — must match the CSS formula.
    const artSize = Math.min(width * 0.45, height * 0.45, 480);
    const innerRadius = artSize / 2 + 4;
    // Cap to the actual space between innerRadius and the canvas edge so bars
    // never overflow the canvas boundary (the panel clips with overflow:hidden).
    const availableRadius = Math.min(width, height) / 2 - innerRadius;
    const maxBarHeight = availableRadius * 0.88;

    ctx.clearRect(0, 0, width, height);

    // Interpolate current bands toward target (smooth decay between FFT frames).
    for (let i = 0; i < NUM_BANDS; i++) {
      currentBands[i] = currentBands[i] * 0.72 + targetBands[i] * 0.28;
    }

    const angleStep = (Math.PI * 2) / NUM_BANDS;

    // Glow pass: draw wider, fully transparent bars first for bloom effect.
    ctx.shadowBlur = 18;
    ctx.shadowColor = accentColor + '80';

    const gradient = ctx.createRadialGradient(cx, cy, innerRadius, cx, cy, innerRadius + maxBarHeight);
    gradient.addColorStop(0, accentColor + '55');
    gradient.addColorStop(0.5, accentColor + 'bb');
    gradient.addColorStop(1, accentColor + 'ff');
    ctx.fillStyle = gradient;

    for (let i = 0; i < NUM_BANDS; i++) {
      const magnitude = currentBands[i];
      if (magnitude < 0.002) continue;

      const barLength = magnitude * maxBarHeight;
      // Start at 12 o'clock, rotate clockwise.
      const midAngle = i * angleStep - Math.PI / 2;
      const halfGap = angleStep * 0.08;
      const startAngle = midAngle - angleStep / 2 + halfGap;
      const endAngle = midAngle + angleStep / 2 - halfGap;

      ctx.beginPath();
      ctx.arc(cx, cy, innerRadius, startAngle, endAngle);
      ctx.arc(cx, cy, innerRadius + barLength, endAngle, startAngle, true);
      ctx.closePath();
      ctx.fill();
    }
  }

  onMount(() => {
    accentColor = getComputedStyle(document.documentElement)
      .getPropertyValue('--accent')
      .trim();

    commands.playbackEnableVisualizer();

    const unsubscribe = onVisualizerData((bands) => {
      for (let i = 0; i < NUM_BANDS; i++) {
        targetBands[i] = bands[i] ?? 0;
      }
    });

    animationFrameId = requestAnimationFrame(drawFrame);

    // Sync canvas size to its CSS size.
    const observer = new ResizeObserver(() => {
      if (!canvas) return;
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
    });
    if (canvas) {
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
      observer.observe(canvas);
    }

    return () => {
      observer.disconnect();
    };
  });

  onDestroy(() => {
    cancelAnimationFrame(animationFrameId);
    commands.playbackDisableVisualizer();
  });

  const artPlaying = $derived(player.snapshot.state === 'playing');
  const artOpacity = $derived(artPlaying ? 0.28 : 1.0);
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="visualizer"
  role="dialog"
  aria-modal="true"
  aria-label="Visualizer"
  transition:fade={{ duration: TRANSITION_DURATION_MILLISECONDS }}
>
  {#if player.snapshot.art_path}
    <div
      class="background-art"
      style:background-image="url({convertFileSrc(player.snapshot.art_path)})"
    ></div>
  {:else}
    <div class="background-gradient"></div>
  {/if}
  <div class="scrim"></div>

  <button class="close-button" onclick={onclose} aria-label="Close visualizer">
    <X size={20} />
  </button>

  <div class="content">
    <div class="art-and-canvas">
      <canvas bind:this={canvas} class="canvas" aria-hidden="true"></canvas>

      <div class="art-wrapper" class:playing={artPlaying} style:opacity={artOpacity}>
        {#if player.snapshot.art_path}
          <img
            class="art"
            src={convertFileSrc(player.snapshot.art_path)}
            alt={player.snapshot.album ?? ""}
          />
        {:else}
          <div class="art art-placeholder"></div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .visualizer {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: var(--transport-height);
    z-index: 100;
    overflow: hidden;
  }

  .background-art {
    position: absolute;
    inset: -40px;
    background-size: cover;
    background-position: center;
    filter: blur(40px);
    transform: scale(1.1);
  }

  .background-gradient {
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, var(--bg-base), var(--bg-elevated));
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim-heavy);
  }

  .close-button {
    position: absolute;
    top: 0.75em;
    right: 0.75em;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: rgba(255, 255, 255, 0.1);
    border: none;
    border-radius: var(--radius);
    color: var(--on-scrim);
    cursor: pointer;
  }

  .close-button:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  .content {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Square container that matches the art/canvas sizing formula. */
  .art-and-canvas {
    position: relative;
    width: min(45vw, 45vh, 480px);
    height: min(45vw, 45vh, 480px);
  }

  .canvas {
    position: absolute;
    /* Canvas extends beyond the art-and-canvas bounds so bars can overflow. */
    inset: calc(-1 * min(28vw, 28vh, 300px));
    width: calc(100% + 2 * min(28vw, 28vh, 300px));
    height: calc(100% + 2 * min(28vw, 28vh, 300px));
  }

  .art-wrapper {
    position: absolute;
    inset: 0;
    border-radius: var(--radius);
    overflow: hidden;
    transition: opacity 1.5s ease, border-radius 0.5s ease;
    z-index: 1;
  }

  .art-wrapper.playing {
    border-radius: 50%;
  }

  .art {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.03));
  }

</style>
