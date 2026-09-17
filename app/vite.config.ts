import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    cors: true,
    allowedHosts: true,
    watch: { ignored: ["**/src-tauri/**"] },
    fs: {
      strict: false,
      allow: [path.resolve(__dirname)],
    },
  },
  build: {
    target: "chrome110",
    sourcemap: false,
  },
});
