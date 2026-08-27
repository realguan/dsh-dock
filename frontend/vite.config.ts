import path from "node:path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

// Tauri 约定：端口 1420 且 strictPort（tauri.conf devUrl 指死这里，被占用
// 就直接失败，不允许静默漂移到其他端口）；clearScreen 关掉避免吞 Rust 报错。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "chrome105",
    // Vite 8 起默认压缩器为 oxc（esbuild 已不再内置，勿显式指定）。
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
