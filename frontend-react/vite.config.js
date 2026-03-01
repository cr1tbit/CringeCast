import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

const API_PROXY_PATHS = [
  "/say",
  "/mow",
  "/guess",
  "/play",
  "/vol",
  "/stop",
  "/teapot",
  "/getFilelist",
  "/uploader",
];

export default defineConfig(({ command }) => {
  const isBuild = command === "build";

  return {
    plugins: [react()],
    base: isBuild ? "/static/metro/" : "/",
    build: {
      outDir: resolve(__dirname, "../CringeCast/static/metro"),
      emptyOutDir: true,
      sourcemap: false,
    },
    server: {
      host: "0.0.0.0",
      port: 5173,
      strictPort: true,
      proxy: API_PROXY_PATHS.reduce((acc, path) => {
        acc[path] = {
          target: "http://127.0.0.1:42069",
          changeOrigin: false,
        };
        return acc;
      }, {}),
    },
  };
});
