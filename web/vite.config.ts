import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The Rust server embeds web/dist (ADR-0004) and listens on 127.0.0.1:8080.
// In development, Vite serves the UI and proxies the API (including SSE) to it.
// NICHE_API_URL overrides the target (e.g. when port 8080 is taken on the dev machine).
// changeOrigin stays false: the server checks that Origin matches Host on POSTs (CSRF guard).
const api = { target: process.env.NICHE_API_URL ?? "http://127.0.0.1:8080", changeOrigin: false };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(import.meta.dirname, "./src") },
  },
  server: { proxy: { "/api": api } },
  preview: { proxy: { "/api": api } },
  build: {
    outDir: "dist",
    assetsDir: "assets",
    sourcemap: false,
    chunkSizeWarningLimit: 1600,
  },
});
