<script lang="ts" generics="Value extends string">
  import { tick } from "svelte";
  import { ChevronDown } from "@lucide/svelte";

  type Option = { value: Value; label: string };

  let {
    options,
    value,
    onChange,
    ariaLabel,
  }: {
    options: Option[];
    value: Value;
    onChange: (value: Value) => void;
    ariaLabel: string;
  } = $props();

  let open = $state(false);
  let focusedOptionIndex = $state(0);
  let optionElements = $state<(HTMLButtonElement | null)[]>([]);
  let triggerElement = $state<HTMLButtonElement | null>(null);

  const selectedLabel = $derived(options.find((option) => option.value === value)?.label ?? "");

  function select(option: Option) {
    onChange(option.value);
    open = false;
    triggerElement?.focus();
  }

  async function openList() {
    open = true;
    focusedOptionIndex = Math.max(0, options.findIndex((option) => option.value === value));
    await tick();
    optionElements[focusedOptionIndex]?.focus();
  }

  function handleListKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusedOptionIndex = (focusedOptionIndex + 1) % options.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "ArrowUp":
        event.preventDefault();
        focusedOptionIndex = (focusedOptionIndex - 1 + options.length) % options.length;
        optionElements[focusedOptionIndex]?.focus();
        break;
      case "Home":
        event.preventDefault();
        focusedOptionIndex = 0;
        optionElements[0]?.focus();
        break;
      case "End":
        event.preventDefault();
        focusedOptionIndex = options.length - 1;
        optionElements[options.length - 1]?.focus();
        break;
      case "Escape":
        open = false;
        triggerElement?.focus();
        break;
    }
  }

  function handleButtonKeydown(event: KeyboardEvent) {
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && !open) {
      event.preventDefault();
      openList();
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
  class="dropdown"
  use:closeOnOutsideClick
  onfocusout={(event) => {
    if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)) {
      open = false;
    }
  }}
>
  <button
    bind:this={triggerElement}
    type="button"
    class="dropdown-trigger"
    class:active={open}
    onclick={() => (open ? (open = false) : openList())}
    onkeydown={handleButtonKeydown}
    aria-label={ariaLabel}
    aria-haspopup="listbox"
    aria-expanded={open}
  >
    <span class="dropdown-value">{selectedLabel}</span>
    <ChevronDown size={14} class="dropdown-chevron" />
  </button>

  {#if open}
    <div
      class="dropdown-popover"
      role="listbox"
      aria-label={ariaLabel}
      tabindex="-1"
      onkeydown={handleListKeydown}
    >
      {#each options as option, i (option.value)}
        <button
          type="button"
          role="option"
          aria-selected={option.value === value}
          tabindex="-1"
          bind:this={optionElements[i]}
          class="dropdown-option"
          class:selected={option.value === value}
          onclick={() => select(option)}
        >
          {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
  }

  .dropdown-trigger {
    display: flex;
    align-items: center;
    gap: 0.5em;
    background-color: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    cursor: pointer;
    font-family: inherit;
    font-size: inherit;
    padding: 0.4em 0.6em;
  }

  .dropdown-trigger:hover,
  .dropdown-trigger.active {
    border-color: var(--accent);
  }

  .dropdown-value {
    white-space: nowrap;
  }

  .dropdown-trigger :global(.dropdown-chevron) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .dropdown-popover {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 100%;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px;
    z-index: 101;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .dropdown-option {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.4em 0.6em;
    font-size: inherit;
    font-family: inherit;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }

  .dropdown-option:hover,
  .dropdown-option.selected {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
