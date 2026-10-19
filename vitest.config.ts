import { configDefaults, defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config";

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: ["./src/test/setup.ts"],
      css: false,
      // Local agent worktrees live inside the checkout; never run their copies of the tests.
      exclude: [...configDefaults.exclude, ".claude/**"],
    },
  }),
);
