// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { AiMessage, ChatEvent } from "../../lib/types";

const flush = async () => { for (let i = 0; i < 4; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
const msg = (role: AiMessage["role"], content: string): AiMessage => ({ id: content, conversation_id: "cv", role, content, tool_name: null, created_at: 0 });
const conv = (id: string, title = "Conversa") => ({ id, book_id: "b", title, rolling_summary: "", folded_count: 0, closed: false, created_at: 0, updated_at: 0 });

vi.mock("../../lib/api", () => ({
  aiKeyStatus: vi.fn(async () => true),
  aiKeySet: vi.fn(async () => {}),
  aiConversation: vi.fn(async () => ({ conversation: conv("cv"), messages: [msg("user", "oi"), msg("assistant", "olá")] })),
  aiConversationNew: vi.fn(async () => ({ conversation: conv("cv2"), messages: [] })),
  aiConversations: vi.fn(async () => [conv("cv2", "Nova"), { ...conv("cv", "Sobre o Rafa"), closed: true }]),
  aiConversationOpen: vi.fn(async () => ({ conversation: conv("cv", "Sobre o Rafa"), messages: [msg("user", "antiga")] })),
  aiSend: vi.fn(),
  proposalsPending: vi.fn(async () => []),
  aiRetry: vi.fn(),
}));
import * as api from "../../lib/api";
import ChatView from "./ChatView.vue";

let app: App | null = null;
async function mount(mode = "conselheira") {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(ChatView, { bookId: "b", sceneId: "s1", mode });
  app.mount(el);
  await flush();
  return el;
}
async function type(el: HTMLElement, text: string) {
  const ta = el.querySelector("textarea")!;
  ta.value = text;
  ta.dispatchEvent(new Event("input"));
  await nextTick();
  ta.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  await flush();
}
const bubbles = (el: HTMLElement) => [...el.querySelectorAll(".msg")].map((m) => m.textContent!.trim());
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

const proposal = { id: "p1", book_id: "b", conversation_id: "cv", scene_id: "s1", text: "Rafa sorriu.", status: "pendente" as const, created_at: 0 };

describe("ChatView", () => {
  it("cartão de proposta: os 3 botões emitem e o cartão some", async () => {
    vi.mocked(api.proposalsPending).mockResolvedValueOnce([proposal, { ...proposal, id: "p2" }, { ...proposal, id: "p3" }]);
    const emitted: string[] = [];
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(ChatView, {
      bookId: "b", sceneId: "s1", mode: "coautora",
      onInsertProposal: () => emitted.push("insert"), onProposalAsScene: () => emitted.push("scene"), onDiscardProposal: () => emitted.push("discard"),
    });
    app.mount(el);
    await flush();
    expect(el.querySelectorAll(".proposal")).toHaveLength(3);
    (el.querySelector('[data-test="p-insert"]') as HTMLButtonElement).click(); await flush();
    (el.querySelector('[data-test="p-scene"]') as HTMLButtonElement).click(); await flush();
    (el.querySelector('[data-test="p-discard"]') as HTMLButtonElement).click(); await flush();
    expect(emitted).toEqual(["insert", "scene", "discard"]);
    expect(el.querySelectorAll(".proposal")).toHaveLength(0);
  });

  it("conselheira nunca mostra cartão de proposta", async () => {
    vi.mocked(api.proposalsPending).mockResolvedValueOnce([proposal]);
    vi.mocked(api.aiSend).mockImplementation(async (_b, _s, _t, on: (e: ChatEvent) => void) => {
      on({ type: "proposal", proposal });
      on({ type: "done", message_id: "m" });
    });
    const el = await mount("conselheira");
    await type(el, "escreve");
    expect(el.querySelector(".proposal")).toBeNull();
  });

  it("mostra o histórico ao abrir", async () => {
    const el = await mount();
    expect(bubbles(el)).toEqual(["oi", "olá"]);
  });

  it("transmite os pedaços na última bolha e mostra notas", async () => {
    vi.mocked(api.aiSend).mockImplementation(async (_b, _s, _t, on: (e: ChatEvent) => void) => {
      on({ type: "note", text: "consultei a cena X" });
      on({ type: "delta", text: "Ele " });
      on({ type: "delta", text: "sabia." });
      on({ type: "done", message_id: "m9" });
    });
    const el = await mount();
    await type(el, "o Rafa sabia?");
    expect(api.aiSend).toHaveBeenCalledWith("b", "s1", "o Rafa sabia?", expect.any(Function));
    expect(bubbles(el).slice(-2)).toEqual(["o Rafa sabia?", "Ele sabia."]);
    expect(el.querySelector(".note")!.textContent).toContain("consultei a cena X");
  });

  it("erro mostra tentar de novo e mantém a pergunta", async () => {
    vi.mocked(api.aiSend).mockImplementation(async (_b, _s, _t, on: (e: ChatEvent) => void) => {
      on({ type: "delta", text: "meio" });
      on({ type: "error", message: "Sem conexão" });
      throw "Sem conexão";
    });
    vi.mocked(api.aiRetry).mockImplementation(async (_b, _s, on: (e: ChatEvent) => void) => {
      on({ type: "delta", text: "Agora foi." });
      on({ type: "done", message_id: "m2" });
    });
    const el = await mount();
    await type(el, "pergunta");
    expect(bubbles(el).slice(-1)).toEqual(["pergunta"]);
    expect(el.textContent).toContain("Sem conexão");
    (el.querySelector('[data-test="retry"]') as HTMLButtonElement).click();
    await flush();
    expect(bubbles(el).slice(-2)).toEqual(["pergunta", "Agora foi."]);
    expect(el.textContent).not.toContain("Sem conexão");
  });

  it("sem chave pede a chave", async () => {
    vi.mocked(api.aiKeyStatus).mockResolvedValueOnce(false);
    const el = await mount();
    expect(el.querySelector('input[type="password"]')).not.toBeNull();
    expect(el.querySelector("textarea")).toBeNull();
  });

  it("nova conversa e reabrir antiga", async () => {
    const el = await mount();
    (el.querySelector('[data-test="new-conv"]') as HTMLButtonElement).click();
    await flush();
    expect(bubbles(el)).toEqual([]);
    (el.querySelector('[data-test="history"]') as HTMLButtonElement).click();
    await flush();
    const old = [...el.querySelectorAll(".conv-item")].find((b) => b.textContent!.includes("Sobre o Rafa")) as HTMLButtonElement;
    old.click();
    await flush();
    expect(api.aiConversationOpen).toHaveBeenCalledWith("cv");
    expect(bubbles(el)).toEqual(["antiga"]);
  });
});
