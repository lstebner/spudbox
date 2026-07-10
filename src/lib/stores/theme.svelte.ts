import { commands } from "$lib/api/commands";

export type Theme = "dark" | "light" | "mint" | "grape" | "lemon";

export const THEMES: { id: Theme; label: string }[] = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "mint", label: "Mint" },
  { id: "grape", label: "Grape" },
  { id: "lemon", label: "Lemon" },
];

const DEFAULT_THEME: Theme = "dark";

function isTheme(value: string): value is Theme {
  return THEMES.some((t) => t.id === value);
}

function apply(theme: Theme) {
  document.documentElement.setAttribute("data-theme", theme);
}

function createThemeStore() {
  let current = $state<Theme>(DEFAULT_THEME);

  return {
    get current() {
      return current;
    },

    async init() {
      const saved = await commands.appearanceGetTheme();
      current = isTheme(saved) ? saved : DEFAULT_THEME;
      apply(current);
    },

    async setTheme(theme: Theme) {
      current = theme;
      apply(theme);
      await commands.appearanceSetTheme(theme);
    },
  };
}

export const theme = createThemeStore();
