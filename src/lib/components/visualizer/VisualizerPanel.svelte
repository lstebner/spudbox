<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { onVisualizerData } from "$lib/api/events";
  import { player } from "$lib/stores/player.svelte";
  import Dropdown from "$lib/components/common/Dropdown.svelte";

  const TRANSITION_DURATION_MILLISECONDS = 200;
  const NUM_BANDS = 64;
  // How far (in degrees) the hue drifts across the full band range.
  // Low-frequency bands shift one way, high-frequency the other, but the
  // total swing is narrow enough that every theme still reads as its own color.
  const HUE_SPREAD_DEGREES = 50;
  // Sub-bass (bands 0–5, ~20–40Hz) and ultra-highs (bands 58–63, ~12–20kHz)
  // are nearly silent in most music and create a visible flat gap where the
  // outline closes at 12 o'clock. The outline uses only this inner slice,
  // spread evenly around the full circle so the shape stays round.
  const OUTLINE_BAND_START = 6;
  const OUTLINE_BAND_END = 57; // inclusive

  type VisualizerStyle = "bars" | "outline";
  const STYLE_OPTIONS: { value: VisualizerStyle; label: string }[] = [
    { value: "bars", label: "Bars" },
    { value: "outline", label: "Outline" },
  ];

  let { onclose }: { onclose: () => void } = $props();

  const STYLE_STORAGE_KEY = "spudbox.visualizerStyle";

  let canvas: HTMLCanvasElement | undefined = $state();
  let visualizerStyle: VisualizerStyle = $state(
    (localStorage.getItem(STYLE_STORAGE_KEY) as VisualizerStyle | null) ?? "bars",
  );

  $effect(() => {
    localStorage.setItem(STYLE_STORAGE_KEY, visualizerStyle);
  });

  let currentBands = new Float32Array(NUM_BANDS);
  let targetBands = new Float32Array(NUM_BANDS);
  let animationFrameId = 0;
  // HSL components of the active theme's accent color, parsed once on mount.
  let accentH = 0;
  let accentS = 0;
  let accentL = 0;

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onclose();
  }

  function hexToHsl(hex: string): [number, number, number] {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const l = (max + min) / 2;
    if (max === min) return [0, 0, l * 100];
    const d = max - min;
    const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    let h = 0;
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
    else if (max === g) h = ((b - r) / d + 2) / 6;
    else h = ((r - g) / d + 4) / 6;
    return [h * 360, s * 100, l * 100];
  }

  function drawBars(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    innerRadius: number,
    maxBarHeight: number,
    angleStep: number,
    outerClipRadius: number,
  ) {
    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, outerClipRadius, 0, Math.PI * 2);
    ctx.clip();
    ctx.shadowBlur = 18;

    for (let i = 0; i < NUM_BANDS; i++) {
      const magnitude = currentBands[i];
      if (magnitude < 0.002) continue;

      const barLength = magnitude * maxBarHeight;
      const midAngle = i * angleStep - Math.PI / 2;
      const halfGap = angleStep * 0.08;
      const startAngle = midAngle - angleStep / 2 + halfGap;
      const endAngle = midAngle + angleStep / 2 - halfGap;

      const hueOffset = (i / (NUM_BANDS - 1) - 0.5) * HUE_SPREAD_DEGREES;
      const barH = ((accentH + hueOffset) % 360 + 360) % 360;
      const barLighter = Math.min(accentL + 15, 92);

      ctx.shadowColor = `hsla(${barH}, ${accentS}%, ${accentL}%, 0.55)`;

      const gradient = ctx.createRadialGradient(
        cx, cy, innerRadius * 0.9,
        cx, cy, innerRadius + barLength,
      );
      gradient.addColorStop(0, `hsla(${barH}, ${accentS}%, ${accentL}%, 0.45)`);
      gradient.addColorStop(1, `hsla(${barH}, ${accentS}%, ${barLighter}%, 1.0)`);
      ctx.fillStyle = gradient;

      ctx.beginPath();
      ctx.arc(cx, cy, innerRadius, startAngle, endAngle);
      ctx.arc(cx, cy, innerRadius + barLength, endAngle, startAngle, true);
      ctx.closePath();
      ctx.fill();
    }

    ctx.restore();
  }

  function drawOutline(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    innerRadius: number,
    maxBarHeight: number,
    _angleStep: number,
    outerClipRadius: number,
  ) {
    const bandCount = OUTLINE_BAND_END - OUTLINE_BAND_START + 1;

    // Compute tip positions using only the musically active band slice.
    // Angles are derived from position within the slice (not global band index)
    // so the points are spread evenly around the full 360°.
    const tipX = new Float32Array(bandCount);
    const tipY = new Float32Array(bandCount);
    for (let index = 0; index < bandCount; index++) {
      const bandIndex = OUTLINE_BAND_START + index;
      const radius = innerRadius + currentBands[bandIndex] * maxBarHeight;
      const angle = (index / bandCount) * Math.PI * 2 - Math.PI / 2;
      tipX[index] = cx + radius * Math.cos(angle);
      tipY[index] = cy + radius * Math.sin(angle);
    }

    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, outerClipRadius, 0, Math.PI * 2);
    ctx.clip();
    ctx.lineWidth = 2.5;
    ctx.lineCap = "round";
    ctx.shadowBlur = 14;

    // Draw each segment as a quadratic Bezier from the midpoint before this
    // tip to the midpoint after it, using the tip itself as the control point.
    // This gives a smooth C1-continuous closed curve through all tip points.
    for (let index = 0; index < bandCount; index++) {
      const previous = (index - 1 + bandCount) % bandCount;
      const next = (index + 1) % bandCount;

      const startX = (tipX[previous] + tipX[index]) / 2;
      const startY = (tipY[previous] + tipY[index]) / 2;
      const endX = (tipX[index] + tipX[next]) / 2;
      const endY = (tipY[index] + tipY[next]) / 2;

      // Keep hue relative to position in the full frequency range so the color
      // meaning (low = one end, high = other) is preserved even with fewer bands.
      const bandIndex = OUTLINE_BAND_START + index;
      const hueOffset = (bandIndex / (NUM_BANDS - 1) - 0.5) * HUE_SPREAD_DEGREES;
      const segH = ((accentH + hueOffset) % 360 + 360) % 360;
      const segLighter = Math.min(accentL + 15, 92);

      ctx.shadowColor = `hsla(${segH}, ${accentS}%, ${accentL}%, 0.7)`;
      ctx.strokeStyle = `hsla(${segH}, ${accentS}%, ${segLighter}%, 0.95)`;

      ctx.beginPath();
      ctx.moveTo(startX, startY);
      ctx.quadraticCurveTo(tipX[index], tipY[index], endX, endY);
      ctx.stroke();
    }

    ctx.restore();
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

    const artSize = Math.min(width * 0.45, height * 0.45, 480);
    const innerRadius = artSize / 2 + 4;
    const availableRadius = Math.min(width, height) / 2 - innerRadius;
    const maxBarHeight = availableRadius * 0.88;

    // Interpolate toward target bands for smooth motion between FFT frames.
    for (let i = 0; i < NUM_BANDS; i++) {
      currentBands[i] = currentBands[i] * 0.72 + targetBands[i] * 0.28;
    }

    const angleStep = (Math.PI * 2) / NUM_BANDS;
    const outerClipRadius = innerRadius + maxBarHeight + 20;

    // Decay trail confined to the bar ring (annulus) only: the inner circle is
    // punched out with a CCW arc so winding is zero there, leaving the center
    // and the area outside the bars fully transparent on the canvas.
    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, outerClipRadius, 0, Math.PI * 2);
    ctx.arc(cx, cy, innerRadius - 4, 0, Math.PI * 2, true);
    ctx.clip();
    // Outline trails more slowly so ghosts pile up into visible artifacts.
    const decayAlpha = visualizerStyle === "outline" ? 0.05 : 0.15;
    ctx.fillStyle = `rgba(0, 0, 0, ${decayAlpha})`;
    ctx.fillRect(0, 0, width, height);
    ctx.restore();

    if (visualizerStyle === "bars") {
      drawBars(ctx, cx, cy, innerRadius, maxBarHeight, angleStep, outerClipRadius);
    } else {
      drawOutline(ctx, cx, cy, innerRadius, maxBarHeight, angleStep, outerClipRadius);
    }
  }

  let unlistenVisualizer: (() => void) | undefined;

  onMount(() => {
    const hex = getComputedStyle(document.documentElement)
      .getPropertyValue("--accent")
      .trim();
    [accentH, accentS, accentL] = hexToHsl(hex);

    commands.playbackEnableVisualizer();

    onVisualizerData((bands) => {
      for (let i = 0; i < NUM_BANDS; i++) {
        targetBands[i] = bands[i] ?? 0;
      }
    }).then((fn) => {
      unlistenVisualizer = fn;
    });

    animationFrameId = requestAnimationFrame(drawFrame);

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
    unlistenVisualizer?.();
    commands.playbackDisableVisualizer();
  });

  const artPlaying = $derived(player.snapshot.state === "playing");
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

  <!-- Stop Escape from bubbling past this control so it closes the dropdown
       without also closing the visualizer. -->
  <div
    class="style-control"
    role="none"
    onkeydown={(event) => { if (event.key === "Escape") event.stopPropagation(); }}
  >
    <Dropdown
      options={STYLE_OPTIONS}
      value={visualizerStyle}
      onChange={(style) => { visualizerStyle = style; }}
      ariaLabel="Visualizer style"
    />
  </div>

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
    /* Horizontal gradient: darker at the edges so the blurred background art
       glows through in the center while the sides fade to near-black. */
    background: rgba(0, 0, 0, 0.55);
    background: linear-gradient(
      to right,
      rgba(0, 0, 0, 0.88) 0%,
      rgba(0, 0, 0, 0.52) 28%,
      rgba(0, 0, 0, 0.52) 72%,
      rgba(0, 0, 0, 0.88) 100%
    );
  }

  .style-control {
    position: absolute;
    top: 0.75em;
    left: 0.75em;
    z-index: 1;
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

  .art-and-canvas {
    position: relative;
    width: min(45vw, 45vh, 480px);
    height: min(45vw, 45vh, 480px);
  }

  .canvas {
    position: absolute;
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
