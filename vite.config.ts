import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { codeInspectorPlugin } from "code-inspector-plugin";

export default defineConfig(({ command }) => ({
  root: "src",
  plugins: [
    command === "serve" &&
      codeInspectorPlugin({
        bundler: "vite",
      }),
    react(),
  ].filter(Boolean),
  base: "./",
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  server: {
    port: 3000,
    strictPort: true,
    // Vite 7 默认为严格主机名校验，列出 dev/SSH 隧道常见来源。
    // `true` 允许所有（开发场景最省事），具体主机名列表更适合共享环境。
    allowedHosts: true,
    // /api/* 转发到 cc-switch-mini-server（cargo run -p cc-switch-mini-server -- --port 3001）。
    proxy: {
      "/api": {
        target: "http://127.0.0.1:3001",
        changeOrigin: false,
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_"],
}));

