import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

export default defineConfig({
  plugins: [solidPlugin()],
  build: {
    target: "esnext",
    rollupOptions: {
      external: ["@tauri-apps/api", "@tauri-apps/api/core", "@tauri-apps/api/event"],
      output: {
        globals: {
          "@tauri-apps/api": "tauriApi",
          "@tauri-apps/api/core": "tauriApi.core",
          "@tauri-apps/api/event": "tauriApi.event",
        },
      },
    },
  },
});
