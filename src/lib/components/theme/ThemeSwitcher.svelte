<script lang="ts">
  import { tick } from "svelte";
  import { Palette, Check } from "@lucide/svelte";
  import { theme, THEMES } from "$lib/stores/theme.svelte";

  let open = $state(false);
  let focusedOptionIndex = $state(0);
  let optionElements: (HTMLButtonElement | null)[] = [];
  let themeButtonElement = $state<HTMLButtonElement | null>(null);

  function selectTheme(id: (typeof THEMES)[number]["id"]) {
    theme.setTheme(id);
    open = false;
    themeButtonElement?.focus();
  }

  async function openThemeList() {
    open = true;
    focusedOptionIndex = Math.max(0, THEMES.findIndex((t) => t.id === theme.current));
    await tick();
    optionElements[focusedOptionIndex]?.focus();
  }

  function handleListKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusedOptionIndex = (focusedOptionIndex + 1) % THEMES.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "ArrowUp":
        event.preventDefault();
        focusedOptionIndex = (focusedOptionIndex - 1 + THEMES.length) % THEMES.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "Home":
        event.preventDefault();
        focusedOptionIndex = 0;
        optionElements[0]?.focus();
        break;
      case "End":
        event.preventDefault();
        focusedOptionIndex = THEMES.length - 1;
        optionElements[THEMES.length - 1]?.focus();
        break;
      case "Escape":
        open = false;
        themeButtonElement?.focus();
        break;
    }
  }

  function handleButtonKeydown(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && !open) {
      event.preventDefault();
      openThemeList();
    }
  }

  function closeOnOutsideClick(node: HTMLElement) {
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

<div
  class="theme-switcher"
  use:closeOnOutsideClick
  onfocusout={(e) => {
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node | null)) {
      open = false;
    }
  }}
>
  <button
    bind:this={themeButtonElement}
    class="theme-button"
    class:active={open}
    onclick={() => (open ? (open = false) : openThemeList())}
    onkeydown={handleButtonKeydown}
    aria-label="Change theme"
    aria-haspopup="listbox"
    aria-expanded={open}
    title="Change theme"
  >
    <Palette size={20} />
  </button>

  {#if open}
    <div
      class="theme-popover"
      role="listbox"
      aria-label="Theme"
      tabindex="-1"
      onkeydown={handleListKeydown}
    >
      {#each THEMES as option, i (option.id)}
        <button
          role="option"
          aria-selected={option.id === theme.current}
          tabindex="-1"
          bind:this={optionElements[i]}
          class="theme-option"
          class:selected={option.id === theme.current}
          onclick={() => selectTheme(option.id)}
        >
          <span class="swatch" data-theme={option.id}></span>
          {option.label}
          {#if option.id === theme.current}
            <Check size={14} class="check" />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-switcher {
    position: relative;
  }

  .theme-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .theme-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .theme-button.active {
    color: var(--accent);
  }

  .theme-popover {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 160px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px;
    z-index: 101;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .theme-option {
    display: flex;
    align-items: center;
    gap: 0.6em;
    width: 100%;
    text-align: left;
    padding: 0.4em 0.6em;
    font-size: 1rem;
    font-family: inherit;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .theme-option:hover,
  .theme-option.selected {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .theme-option :global(.check) {
    margin-left: auto;
    color: var(--accent);
  }

  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex-shrink: 0;
    border: 1px solid var(--border);
  }

  /* Swatches show each theme's own --bg-base, regardless of the currently
   * active theme, so this is the one place a theme's color is looked up by
   * an explicit data-theme selector rather than inherited from the root.
   * --bg-base (not --accent) is what actually previews "what this theme
   * looks like": using --accent made dark/light backwards (dark's accent
   * is brighter than light's, since it needs to pop against a near-black
   * background — the opposite of what "Dark"/"Light" implies at a
   * glance), and for mint/grape/lemon, --accent is a deep, low-lightness
   * tone chosen for text contrast, not their identity color. */
  .swatch[data-theme="dark"] { background: #121214; }
  .swatch[data-theme="light"] { background: #f2f2f4; }
  .swatch[data-theme="mint"] { background: #cef3e5; }
  .swatch[data-theme="grape"] { background: #dfccf0; }
  .swatch[data-theme="lemon"] { background: #ffff81; }
</style>
