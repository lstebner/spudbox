import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    // Use browser builds of packages (not SSR/server builds).
    conditions: ["browser"],
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "happy-dom",
    setupFiles: ["./src/vitest.setup.ts"],
  },
});
