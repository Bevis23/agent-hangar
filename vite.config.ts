import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 关键：cargo 往 target/ 写 DLL 时文件被锁，vite watcher 会 EBUSY 崩溃；
      // Rust 侧由 tauri CLI 自行监听，前端无需关心这两个目录。
      ignored: ["**/target/**", "**/src-tauri/**"],
    },
  },
});
