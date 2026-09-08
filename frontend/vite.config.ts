import path from "node:path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
// vitest/config 的 defineConfig = Vite 的 + `test` 键类型（devDependency，构建期只读配置）
import { defineConfig } from "vitest/config"

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
  // Vitest 默认 `css: false` 会把 CSS 模块桩成空串——`__tests__/contrast.test.ts`
  // 需要 `?raw` 读到 index.css 的真实 token 值（2026-09-08 批次 C）。
  test: {
    css: true,
  },
})
