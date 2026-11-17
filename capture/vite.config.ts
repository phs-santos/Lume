import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [vue()],
  // reaproveita a cena da escrivaninha do app do computador
  server: { port: 5180, fs: { allow: [".."] }, proxy: { "/api": "http://127.0.0.1:3020" } },
});
