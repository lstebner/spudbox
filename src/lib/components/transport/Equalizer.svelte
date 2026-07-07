<script lang="ts">
  import { tick } from "svelte";
  import { SlidersVertical } from "@lucide/svelte";
  import { player } from "$lib/stores/player.svelte";

  const BAND_FREQUENCIES = [63, 125, 250, 500, 1000, 2000, 4000, 8000];
  const BAND_LABELS = ["63", "125", "250", "500", "1k", "2k", "4k", "8k"];
  const BAND_COUNT = BAND_FREQUENCIES.length;
  const MAX_GAIN_DB = 12;
  const EQ_Q = 1.414;
  const DISPLAY_SAMPLE_RATE = 48000;

  const SVG_WIDTH = 220;
  const SVG_HEIGHT = 44;

  // Gains are stored as integers or 0.5-step floats; epsilon guards against
  // any floating-point rounding in the round-trip through the backend.
  const GAIN_EPSILON = 0.01;

  const PRESETS: Record<string, number[]> = {
    Custom:        [],
    Flat:          [ 0,  0,  0,  0,  0,  0,  0,  0],
    "Bass Boost":  [ 8,  5,  2,  0,  0,  0,  0,  0],
    "Treble Boost":[ 0,  0,  0,  0,  0,  2,  5,  8],
    Rock:          [ 6,  4,  1, -2, -2,  1,  4,  6],
    "Mid Boost":   [-4, -2,  0,  3,  4,  3,  0, -4],
    Classical:     [ 4,  2,  0, -1, -1,  0,  2,  4],
    Vocal:         [-2,  0,  1,  3,  4,  3,  1, -2],
  };

  const PRESET_NAMES = Object.keys(PRESETS);

  function matchPreset(gains: number[]): string | null {
    for (const [name, presetGains] of Object.entries(PRESETS)) {
      if (name === "Custom") continue;
      if (gains.every((g, i) => Math.abs(g - presetGains[i]) < GAIN_EPSILON)) {
        return name;
      }
    }
    return null;
  }

  let open = $state(false);
  let presetOpen = $state(false);
  let currentPreset = $state<string>("Custom");
  let customGains = $state<number[]>(new Array(BAND_COUNT).fill(0));
  let focusedOptionIndex = $state(0);
  let optionElements: (HTMLButtonElement | null)[] = [];
  let presetButtonElement = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    const matched = matchPreset(player.eqGains);
    if (matched !== null) {
      currentPreset = matched;
    } else {
      currentPreset = "Custom";
      customGains = [...player.eqGains];
    }
  });

  function applyPreset(name: string) {
    if (name === "Custom") {
      player.setEq([...customGains], player.eqEnabled);
    } else if (name in PRESETS) {
      player.setEq([...PRESETS[name]], player.eqEnabled);
    }
    presetOpen = false;
    presetButtonElement?.focus();
  }

  async function openPresetList() {
    presetOpen = true;
    focusedOptionIndex = Math.max(0, PRESET_NAMES.indexOf(currentPreset));
    await tick();
    optionElements[focusedOptionIndex]?.focus();
  }

  function handleListKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusedOptionIndex = (focusedOptionIndex + 1) % PRESET_NAMES.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "ArrowUp":
        event.preventDefault();
        focusedOptionIndex =
          (focusedOptionIndex - 1 + PRESET_NAMES.length) % PRESET_NAMES.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "Home":
        event.preventDefault();
        focusedOptionIndex = 0;
        optionElements[0]?.focus();
        break;
      case "End":
        event.preventDefault();
        focusedOptionIndex = PRESET_NAMES.length - 1;
        optionElements[PRESET_NAMES.length - 1]?.focus();
        break;
      case "Escape":
        presetOpen = false;
        presetButtonElement?.focus();
        break;
    }
  }

  function handleButtonKeydown(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && !presetOpen) {
      event.preventDefault();
      openPresetList();
    }
  }

  function closePresetOnOutsideClick(node: HTMLElement) {
    function onMouseDown(event: MouseEvent) {
      if (!node.contains(event.target as Node)) presetOpen = false;
    }
    document.addEventListener("mousedown", onMouseDown);
    return { destroy() { document.removeEventListener("mousedown", onMouseDown); } };
  }

  function peakingCoeffs(f: number, sr: number, gainDb: number, q: number) {
    const amplitude = Math.pow(10, gainDb / 40);
    const omega = (2 * Math.PI * f) / sr;
    const cosOmega = Math.cos(omega);
    const alpha = Math.sin(omega) / (2 * q);
    const a0 = 1 + alpha / amplitude;
    return {
      b0: (1 + alpha * amplitude) / a0,
      b1: (-2 * cosOmega) / a0,
      b2: (1 - alpha * amplitude) / a0,
      a1: (-2 * cosOmega) / a0,
      a2: (1 - alpha / amplitude) / a0,
    };
  }

  function lowShelfCoeffs(f: number, sr: number, gainDb: number) {
    const amplitude = Math.pow(10, gainDb / 40);
    const omega = (2 * Math.PI * f) / sr;
    const cosOmega = Math.cos(omega);
    const alpha = (Math.sin(omega) / 2) * Math.sqrt(2);
    const twoSqrtAAlpha = 2 * Math.sqrt(amplitude) * alpha;
    const a0 = amplitude + 1 + (amplitude - 1) * cosOmega + twoSqrtAAlpha;
    return {
      b0: (amplitude * (amplitude + 1 - (amplitude - 1) * cosOmega + twoSqrtAAlpha)) / a0,
      b1: (2 * amplitude * (amplitude - 1 - (amplitude + 1) * cosOmega)) / a0,
      b2: (amplitude * (amplitude + 1 - (amplitude - 1) * cosOmega - twoSqrtAAlpha)) / a0,
      a1: (-2 * (amplitude - 1 + (amplitude + 1) * cosOmega)) / a0,
      a2: (amplitude + 1 + (amplitude - 1) * cosOmega - twoSqrtAAlpha) / a0,
    };
  }

  function highShelfCoeffs(f: number, sr: number, gainDb: number) {
    const amplitude = Math.pow(10, gainDb / 40);
    const omega = (2 * Math.PI * f) / sr;
    const cosOmega = Math.cos(omega);
    const alpha = (Math.sin(omega) / 2) * Math.sqrt(2);
    const twoSqrtAAlpha = 2 * Math.sqrt(amplitude) * alpha;
    const a0 = amplitude + 1 - (amplitude - 1) * cosOmega + twoSqrtAAlpha;
    return {
      b0: (amplitude * (amplitude + 1 + (amplitude - 1) * cosOmega + twoSqrtAAlpha)) / a0,
      b1: (-2 * amplitude * (amplitude - 1 + (amplitude + 1) * cosOmega)) / a0,
      b2: (amplitude * (amplitude + 1 + (amplitude - 1) * cosOmega - twoSqrtAAlpha)) / a0,
      a1: (2 * (amplitude - 1 - (amplitude + 1) * cosOmega)) / a0,
      a2: (amplitude + 1 - (amplitude - 1) * cosOmega - twoSqrtAAlpha) / a0,
    };
  }

  type BiquadCoeffs = ReturnType<typeof peakingCoeffs>;

  function magnitudeDb(coeffsList: BiquadCoeffs[], freq: number, sr: number): number {
    const w = (2 * Math.PI * freq) / sr;
    const cosW = Math.cos(w);
    const sinW = Math.sin(w);
    const cos2W = 2 * cosW * cosW - 1;
    const sin2W = 2 * sinW * cosW;

    let magSq = 1.0;
    for (const c of coeffsList) {
      const nR = c.b0 + c.b1 * cosW + c.b2 * cos2W;
      const nI = -(c.b1 * sinW + c.b2 * sin2W);
      const dR = 1 + c.a1 * cosW + c.a2 * cos2W;
      const dI = -(c.a1 * sinW + c.a2 * sin2W);
      magSq *= (nR * nR + nI * nI) / (dR * dR + dI * dI);
    }
    return 10 * Math.log10(Math.max(magSq, 1e-10));
  }

  const svgPoints = $derived(
    (() => {
      const coeffsList = BAND_FREQUENCIES.map((f, i) => {
        if (i === 0) return lowShelfCoeffs(f, DISPLAY_SAMPLE_RATE, player.eqGains[i]);
        if (i === BAND_FREQUENCIES.length - 1)
          return highShelfCoeffs(f, DISPLAY_SAMPLE_RATE, player.eqGains[i]);
        return peakingCoeffs(f, DISPLAY_SAMPLE_RATE, player.eqGains[i], EQ_Q);
      });

      const N = 80;
      const logFMin = Math.log10(20);
      const logFMax = Math.log10(20000);

      return Array.from({ length: N + 1 }, (_, i) => {
        const logF = logFMin + (i / N) * (logFMax - logFMin);
        const freq = Math.pow(10, logF);
        const gainDb = magnitudeDb(coeffsList, freq, DISPLAY_SAMPLE_RATE);
        const x = (i / N) * SVG_WIDTH;
        const y = SVG_HEIGHT / 2 - (gainDb / MAX_GAIN_DB) * (SVG_HEIGHT / 2) * 0.85;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      }).join(" ");
    })(),
  );

  function setGain(bandIndex: number, gainDb: number) {
    const newGains = [...player.eqGains];
    newGains[bandIndex] = gainDb;
    player.setEq(newGains, player.eqEnabled);
  }

  function clickOutside(node: HTMLElement) {
    function onMouseDown(event: MouseEvent) {
      if (!node.contains(event.target as Node)) open = false;
    }
    document.addEventListener("mousedown", onMouseDown);
    return {
      destroy() {
        document.removeEventListener("mousedown", onMouseDown);
      },
    };
  }
</script>

<div class="eq-wrapper" use:clickOutside>
  <button
    class="eq-button"
    class:active={open}
    onclick={() => (open = !open)}
    aria-label="Equalizer"
    aria-expanded={open}
    title="Equalizer"
  >
    <SlidersVertical size={16} />
  </button>

  {#if open}
    <div class="eq-popover" role="dialog" aria-label="Equalizer settings">
      <div class="curve-area" class:disabled={!player.eqEnabled}>
        <svg
          viewBox="0 0 {SVG_WIDTH} {SVG_HEIGHT}"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <line
            x1="0"
            y1={SVG_HEIGHT / 2}
            x2={SVG_WIDTH}
            y2={SVG_HEIGHT / 2}
            class="reference-line"
          />
          <polyline points={svgPoints} class="curve" />
        </svg>
      </div>

      <div class="bands" class:disabled={!player.eqEnabled}>
        {#each BAND_FREQUENCIES as _freq, i}
          <div class="band">
            <input
              type="range"
              min={-MAX_GAIN_DB}
              max={MAX_GAIN_DB}
              step="0.5"
              value={player.eqGains[i]}
              oninput={(e) => setGain(i, Number((e.target as HTMLInputElement).value))}
              aria-label="{BAND_LABELS[i]} Hz gain"
              aria-valuetext="{player.eqGains[i].toFixed(1)} dB"
            />
            <span class="band-label" aria-hidden="true">{BAND_LABELS[i]}</span>
          </div>
        {/each}
      </div>

      <div class="eq-footer">
        <label class="enabled-label">
          <input
            type="checkbox"
            checked={player.eqEnabled}
            onchange={(e) =>
              player.setEq(player.eqGains, (e.target as HTMLInputElement).checked)}
          />
          Enabled
        </label>
        <div
          class="preset-wrapper"
          use:closePresetOnOutsideClick
          onfocusout={(e) => {
            if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node | null)) {
              presetOpen = false;
            }
          }}
        >
          <button
            bind:this={presetButtonElement}
            class="preset-button"
            onclick={() => (presetOpen ? (presetOpen = false) : openPresetList())}
            onkeydown={handleButtonKeydown}
            aria-haspopup="listbox"
            aria-expanded={presetOpen}
          >
            {currentPreset}
            <span aria-hidden="true">{presetOpen ? "▴" : "▾"}</span>
          </button>
          {#if presetOpen}
            <div
              class="preset-list"
              role="listbox"
              aria-label="EQ preset"
              tabindex="-1"
              onkeydown={handleListKeydown}
            >
              {#each PRESET_NAMES as name, i}
                <button
                  role="option"
                  aria-selected={name === currentPreset}
                  tabindex="-1"
                  bind:this={optionElements[i]}
                  class:selected={name === currentPreset}
                  onclick={() => applyPreset(name)}
                >
                  {name}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .eq-wrapper {
    position: relative;
  }

  .eq-button {
    width: 28px;
    height: 28px;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
  }

  .eq-button:hover {
    color: var(--text-primary);
  }

  .eq-button.active {
    color: var(--accent);
  }

  .eq-popover {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    width: 340px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .curve-area {
    width: 100%;
    height: 64px;
    background: var(--bg-base);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .curve-area svg {
    width: 100%;
    height: 100%;
  }

  .reference-line {
    stroke: var(--border);
    stroke-width: 1;
  }

  .curve {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .curve-area.disabled .curve {
    stroke: var(--text-tertiary);
  }

  .bands {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 2px;
    padding: 0 2px;
  }

  .bands.disabled {
    opacity: 0.35;
    pointer-events: none;
  }

  .band {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    flex: 1;
  }

  .band input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    -webkit-appearance: slider-vertical;
    appearance: slider-vertical;
    width: 20px;
    height: 190px;
    cursor: pointer;
    accent-color: var(--accent);
    flex-shrink: 0;
  }

  .band-label {
    font-size: 1rem;
    color: var(--text-tertiary);
    text-align: center;
    line-height: 1;
  }

  .eq-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 2px;
  }

  .enabled-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 1rem;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }

  .enabled-label input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
    margin: 0;
    align-self: center;
  }

  .preset-wrapper {
    position: relative;
  }

  .preset-button {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 1rem;
    font-family: inherit;
    padding: 2px 8px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }

  .preset-button:hover {
    color: var(--text-primary);
    background: var(--bg-selected);
  }

  .preset-list {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    list-style: none;
    margin: 0;
    padding: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    min-width: 100%;
    z-index: 101;
  }

  .preset-list button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 4px 10px;
    font-size: 1rem;
    font-family: inherit;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }

  .preset-list button:hover,
  .preset-list button:focus,
  .preset-list button.selected {
    background: var(--bg-hover);
    color: var(--text-primary);
    outline: none;
  }
</style>
