// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";

const flush = () => new Promise((r) => setTimeout(r, 0));
vi.mock("../../lib/api", () => ({
  settingGet: vi.fn(async () => null), settingSet: vi.fn(async () => {}),
  aiKeyStatus: vi.fn(async () => true), aiKeySet: vi.fn(async () => {}),
  aiModelsGet: vi.fn(async () => ({ chat: "anthropic/claude-sonnet-5-5", background: "anthropic/claude-haiku-5-5" })),
  aiModelsSet: vi.fn(async () => {}),
  aiUsage: vi.fn(async () => [{ month: "2026-11", model: "m", input_tokens: 48200, output_tokens: 1800, cost: null }]),
}));
import * as api from "../../lib/api";
import SettingsPanel from "./SettingsPanel.vue";

let app: App | null = null;
async function mount() {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(SettingsPanel);
  app.mount(el);
  await flush(); await nextTick();
  return el;
}
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("Ajustes de IA", () => {
  it("mostra chave salva sem mostrar o valor", async () => {
    const el = await mount();
    expect(el.textContent).toContain("Chave salva");
    expect((el.querySelector('input[type="password"]') as HTMLInputElement | null)?.value ?? "").toBe("");
  });

  it("salvar chave chama aiKeySet e limpa o campo", async () => {
    vi.mocked(api.aiKeyStatus).mockResolvedValueOnce(false);
    const el = await mount();
    const input = el.querySelector('input[type="password"]') as HTMLInputElement;
    input.value = "CHAVE";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    (el.querySelector('[data-test="save-key"]') as HTMLButtonElement).click();
    await flush(); await nextTick();
    expect(api.aiKeySet).toHaveBeenCalledWith("CHAVE");
    expect(el.querySelector('input[type="password"]')).toBeNull(); // o campo some e a chave não fica na tela
    expect(el.textContent).toContain("Chave salva");
  });

  it("mostra o gasto do mês em tokens", async () => {
    const el = await mount();
    expect(el.textContent).toContain("50 mil tokens");
  });
});
