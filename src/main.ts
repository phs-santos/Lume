import { createApp } from "vue";
import "./styles/themes.css";
import App from "./App.vue";

async function start() {
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    (await import("./dev/mockBackend")).installMockBackend();
  }
  createApp(App).mount("#app");
}
start();
