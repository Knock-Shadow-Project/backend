import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

const PI_HOST = process.env.PI_HOST ?? "192.168.1.146:8080";

export default defineConfig({
  plugins: [react(), viteSingleFile()],
  build: {
    outDir: "dist",
    assetsInlineLimit: 100000000,
    cssCodeSplit: false,
  },
  server: {
    proxy: {
      "/health": `http://${PI_HOST}`,
      "/sensors": `http://${PI_HOST}`,
      "/training": `http://${PI_HOST}`,
      "/trainings": `http://${PI_HOST}`,
      "/live/accel": { target: `ws://${PI_HOST}`, ws: true },
      "/live": { target: `ws://${PI_HOST}`, ws: true },
    },
  },
});
