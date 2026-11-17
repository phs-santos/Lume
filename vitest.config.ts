import { configDefaults, defineConfig } from "vitest/config";

// O app do computador; `capture/` e `server/` têm os próprios testes e configurações.
export default defineConfig({ test: { exclude: [...configDefaults.exclude, "capture/**", "server/**"] } });
