// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";

const flush = async () => { for (let i = 0; i < 5; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
const settings: Record<string, string> = {};
vi.mock("../../lib/api", () => ({
  aiEnable: vi.fn(async () => {}),
  aiFirstPass: vi.fn(async () => {}),
  aiFirstPassCancel: vi.fn(async () => {}),
  aiKeyStatus: vi.fn(async () => true),
  aiPendingCount: vi.fn(async () => 20),
  settingGet: vi.fn(async (k: string) => settings[k] ?? null),
  settingSet: vi.fn(async (k: string, v: string) => void (settings[k] = v)),
}));
vi.mock("./ChatView.vue", () => ({ default: { name: "ChatView", render: () => null } }));
vi.mock("./BibleView.vue", () => ({ default: { name: "BibleView", render: () => null } }));
import * as api from "../../lib/api";
import AssistantPanel from "./AssistantPanel.vue";

let app: App | null = null;
async function mount() {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(AssistantPanel, { bookId: "b", sceneId: "s", mode: "conselheira", sceneTitles: {} });
  app.mount(el);
  await flush();
  return el;
}
afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  for (const k of Object.keys(settings)) delete settings[k];
  vi.clearAllMocks();
});

describe("leitura do livro só com permissão", () => {
  it("primeira vez: pergunta, pausa o fundo e não lê", async () => {
    const el = await mount();
    expect(el.textContent).toContain("Quer que eu leia o livro");
    expect(el.textContent).toContain("20 cenas");
    expect(api.aiFirstPass).not.toHaveBeenCalled();
    expect(api.settingSet).toHaveBeenCalledWith("ai.enabled.b", "0");
  });

  it("'Ler o livro' liga e começa a ler", async () => {
    const el = await mount();
    (el.querySelector('[data-test="read-yes"]') as HTMLButtonElement).click();
    await flush();
    expect(settings["ai.consent.b"]).toBe("sim");
    expect(api.aiEnable).toHaveBeenCalledWith("b");
    expect(api.aiFirstPass).toHaveBeenCalled();
  });

  it("'Agora não' guarda a escolha e não pergunta de novo", async () => {
    const el = await mount();
    (el.querySelector('[data-test="read-no"]') as HTMLButtonElement).click();
    await flush();
    expect(settings["ai.consent.b"]).toBe("nao");
    expect(api.aiFirstPass).not.toHaveBeenCalled();
    app!.unmount();
    const el2 = await mount();
    expect(el2.textContent).not.toContain("Quer que eu leia o livro");
  });

  it("quem já disse sim só lê o que mudou, sem perguntar", async () => {
    settings["ai.consent.b"] = "sim";
    const el = await mount();
    expect(el.textContent).not.toContain("Quer que eu leia o livro");
    expect(api.aiEnable).toHaveBeenCalledWith("b");
    expect(api.aiFirstPass).toHaveBeenCalled();
  });

  it("sem chave de IA não pergunta nada", async () => {
    vi.mocked(api.aiKeyStatus).mockResolvedValueOnce(false);
    const el = await mount();
    expect(el.textContent).not.toContain("Quer que eu leia o livro");
  });
});
