<script lang="ts">
  import { Palette, Check } from "@lucide/svelte";
  import { theme, THEMES } from "$lib/stores/theme.svelte";

  let open = $state(false);

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

<div class="theme-switcher" use:clickOutside>
  <button
    class="theme-button"
    class:active={open}
    onclick={() => (open = !open)}
    aria-label="Change theme"
    aria-haspopup="listbox"
    aria-expanded={open}
    title="Change theme"
  >
    <Palette size={20} />
  </button>

  {#if open}
    <div class="theme-popover" role="listbox" aria-label="Theme">
      {#each THEMES as option (option.id)}
        <button
          role="option"
          aria-selected={option.id === theme.current}
          class="theme-option"
          class:selected={option.id === theme.current}
          onclick={() => {
            theme.setTheme(option.id);
            open = false;
          }}
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

  /* Swatches show each theme's own accent, regardless of the currently
   * active theme, so this is the one place accent is looked up by an
   * explicit data-theme selector rather than inherited from the root. */
  .swatch[data-theme="dark"] { background: #818cf8; }
  .swatch[data-theme="light"] { background: #5b5fd6; }
  .swatch[data-theme="green"] { background: #78b08b; }
  .swatch[data-theme="purple"] { background: #a478ba; }
  .swatch[data-theme="yellow"] { background: #bf9040; }
</style>
