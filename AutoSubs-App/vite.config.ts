import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

const host = (process.env.TAURI_DEV_HOST as string) || undefined;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return;
          if (id.includes("@fontsource")) return "fonts";
          if (id.includes("@untitledui/file-icons")) return "file-icons";
          if (/[/\\](react|react-dom|react-router|react-router-dom|scheduler)[/\\]/.test(id)) return "react";
          if (/[/\\](recharts|d3-[^/\\]+)[/\\]/.test(id)) return "charts";
          if (/[/\\](react-markdown|remark-[^/\\]+|micromark[^/\\]*|mdast[^/\\]*|hast[^/\\]*|unified)[/\\]/.test(id)) return "markdown";
          if (/[/\\](react-joyride|react-floater)[/\\]/.test(id)) return "tour";
          if (/[/\\](motion|motion-dom|motion-utils|framer-motion)[/\\]/.test(id)) return "motion";
          if (id.includes("@radix-ui") || /[/\\](radix-ui|cmdk|vaul)[/\\]/.test(id)) return "ui";
          if (id.includes("@tauri-apps")) return "tauri";
          if (/[/\\](i18next|react-i18next)[/\\]/.test(id)) return "i18n";
          if (id.includes("@hookform") || /[/\\](react-hook-form|zod)[/\\]/.test(id)) return "forms";
          if (id.includes("@tanstack")) return "tables";
          if (/[/\\](lucide|lucide-react)[/\\]/.test(id)) return "icons";
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
