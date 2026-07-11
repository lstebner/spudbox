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
    coverage: {
      provider: "v8",
      include: ["src/lib/**/*.{ts,svelte}"],
      exclude: [
        "src/lib/api/**",
        "src/lib/types.ts",
        "src/**/*.test.ts",
        "src/vitest.setup.ts",
      ],
    },
  },
});
