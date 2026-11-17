import vue from "@vitejs/plugin-vue";
import { configDefaults, defineConfig } from "vitest/config";

// O app do computador; `capture/` e `server/` têm os próprios testes e configurações.
export default defineConfig({ plugins: [vue()], test: { exclude: [...configDefaults.exclude, "capture/**", "server/**"] } });
