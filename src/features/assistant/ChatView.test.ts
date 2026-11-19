// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { AiMessage, ChatEvent } from "../../lib/types";

const flush = async () => { for (let i = 0; i < 4; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
const msg = (role: AiMessage["role"], content: string): AiMessage => ({ id: content, conversation_id: "cv", role, content, tool_name: null, created_at: 0 });
const conv = (id: string, title = "Conversa") => ({ id, book_id: "b", title, rolling_summary: "", folded_count: 0, closed: false, created_at: 0, updated_at: 0, purpose: "livro" as const });

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn(async () => {}) }));
vi.mock("../../lib/api", () => ({
  aiKeyStatus: vi.fn(async () => true),
  aiKeySet: vi.fn(async () => {}),
  aiConversation: vi.fn(async () => ({ conversation: conv("cv"), messages: [msg("user", "oi"), msg("assistant", "olá")] })),
  aiConversationNew: vi.fn(async () => ({ conversation: conv("cv2"), messages: [] })),
  aiConversations: vi.fn(async () => [conv("cv2", "Nova"), { ...conv("cv", "Sobre o Rafa"), closed: true }]),
  aiConversationOpen: vi.fn(async () => ({ conversation: conv("cv", "Sobre o Rafa"), messages: [msg("user", "antiga")] })),
  aiSend: vi.fn(),
  aiConversationRename: vi.fn(async () => {}),
  aiConversationDelete: vi.fn(async () => {}),
  proposalsPending: vi.fn(async () => []),
  mapProposalsPending: vi.fn(async () => []),
  mapProposalAccept: vi.fn(async () => []),
  mapProposalDiscard: vi.fn(async () => {}),
  aiConversationNewIdea: vi.fn(async () => ({ conversation: { ...conv("cvi", "Desenvolvendo a ideia"), purpose: "ideia" }, messages: [] })),
  aiRetry: vi.fn(),
}));
import * as api from "../../lib/api";
import ChatView from "./ChatView.vue";

let app: App | null = null;
async function mount(mode = "conselheira", draft?: string) {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(ChatView, { bookId: "b", sceneId: "s1", mode, draft });
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
      on({ type: "done", message_id: "m", input_tokens: 1000, output_tokens: 200, cost: 0.004 });
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
      on({ type: "done", message_id: "m9", input_tokens: 1000, output_tokens: 200, cost: 0.004 });
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
      on({ type: "done", message_id: "m2", input_tokens: 1000, output_tokens: 200, cost: 0.004 });
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

  it("um erro não apaga respostas anteriores da sessão", async () => {
    let n = 0;
    vi.mocked(api.aiSend).mockImplementation(async (_b, _s, _t, on: (e: ChatEvent) => void) => {
      n++;
      if (n <= 2) {
        on({ type: "delta", text: `resposta ${n}` });
        on({ type: "done", message_id: `m${n}`, input_tokens: 10, output_tokens: 2, cost: null });
      } else {
        on({ type: "delta", text: "meio" });
        on({ type: "error", message: "Sem conexão" });
        throw "Sem conexão";
      }
    });
    const el = await mount();
    await type(el, "p1");
    await type(el, "p2");
    await type(el, "p3");
    expect(bubbles(el)).toEqual(["oi", "olá", "p1", "resposta 1", "p2", "resposta 2", "p3"]);
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

  it("rascunho vindo de uma ideia preenche o campo sem enviar", async () => {
    const el = await mount("conselheira", "Sobre esta ideia: «Marina foge.»\n\n");
    expect((el.querySelector("textarea") as HTMLTextAreaElement).value).toBe("Sobre esta ideia: «Marina foge.»\n\n");
    expect(api.aiSend).not.toHaveBeenCalled();
  });

  it("avisa que usou o rascunho, para ele não voltar ao reabrir", async () => {
    const used = vi.fn();
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(ChatView, { bookId: "b", sceneId: "s1", mode: "conselheira", draft: "Sobre esta ideia: «x»\n\n", onDraftUsed: used });
    app.mount(el);
    await flush();
    expect(used).toHaveBeenCalled();
  });

  it("conversa vazia mostra perguntas prontas; clicar envia", async () => {
    vi.mocked(api.aiConversation).mockResolvedValueOnce({ conversation: conv("cv"), messages: [] });
    vi.mocked(api.aiSend).mockImplementation(async () => {});
    const el = await mount();
    const chips = el.querySelectorAll('[data-test="suggestion"]');
    expect(chips.length).toBeGreaterThan(2);
    (chips[0] as HTMLButtonElement).click();
    await flush();
    expect(api.aiSend).toHaveBeenCalledWith("b", "s1", chips[0].textContent!.trim(), expect.any(Function));
  });

  it("cada resposta mostra tokens e custo estimado", async () => {
    vi.mocked(api.aiSend).mockImplementation(async (_b, _s, _t, on: (e: ChatEvent) => void) => {
      on({ type: "delta", text: "ok" });
      on({ type: "done", message_id: "m", input_tokens: 5000, output_tokens: 200, cost: 0.0123 });
    });
    const el = await mount();
    await type(el, "pergunta");
    expect(el.querySelector('[data-test="meta"]')!.textContent).toBe("5,2 mil tokens · ~US$ 0,01");
  });

  it("histórico: renomear e apagar (com confirmação)", async () => {
    const el = await mount();
    (el.querySelector('[data-test="history"]') as HTMLButtonElement).click();
    await flush();
    (el.querySelectorAll('[data-test="conv-rename"]')[1] as HTMLButtonElement).click();
    await flush();
    const inp = el.querySelector("#conv-rename-cv") as HTMLInputElement;
    inp.value = "Sobre a Bia";
    inp.dispatchEvent(new Event("input"));
    inp.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await flush();
    expect(api.aiConversationRename).toHaveBeenCalledWith("cv", "Sobre a Bia");
    expect(el.querySelector(".history")!.textContent).toContain("Sobre a Bia");
    const del = () => el.querySelectorAll('[data-test="conv-delete"]')[1] as HTMLButtonElement;
    del().click();
    await flush();
    expect(api.aiConversationDelete).not.toHaveBeenCalled();
    del().click();
    await flush();
    expect(api.aiConversationDelete).toHaveBeenCalledWith("cv");
    expect(el.querySelectorAll(".conv-row")).toHaveLength(1);
  });

  it("proposta diz a cena de destino e pode entrar no cursor", async () => {
    vi.mocked(api.proposalsPending).mockResolvedValueOnce([proposal]);
    const got: unknown[][] = [];
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(ChatView, { bookId: "b", sceneId: "s1", mode: "coautora", sceneTitles: { s1: "Linha 340" }, onInsertProposal: (...a: unknown[]) => got.push(a) });
    app.mount(el);
    await flush();
    expect(el.querySelector(".proposal-label")!.textContent).toContain("Linha 340");
    (el.querySelector('[data-test="p-cursor"]') as HTMLButtonElement).click();
    await flush();
    expect(got[0][1]).toBe("cursor");
  });

  it("map_proposal event shows block in conselheira", async () => {
    vi.mocked(api.aiSend).mockImplementationOnce(async (_b, _s, _t, on) => {
      on({ type: "map_proposal", proposal: { id: "mp", book_id: "b", conversation_id: "cv", kind: "cartoes", status: "pendente", created_at: 0, payload_json: JSON.stringify({ cartoes: [{ ref: "a", texto: "Marina" }], setas: [] }) } });
      on({ type: "done", message_id: "m", input_tokens: 1, output_tokens: 1, cost: null });
    });
    const el = await mount("conselheira");
    await type(el, "organiza o mapa");
    expect(el.textContent).toContain("A IA sugere 1 cartão para o mapa");
  });

  it("pending map proposals come back when opening", async () => {
    vi.mocked(api.mapProposalsPending).mockResolvedValueOnce([{ id: "n", book_id: "b", conversation_id: "cv", kind: "nome", status: "pendente", created_at: 0, payload_json: JSON.stringify({ nome: "O Farol" }) }]);
    const el = await mount("conselheira");
    expect(el.textContent).toContain("Usar “O Farol” como nome do livro?");
  });

  it("startIdea opens idea conversation and sends the idea", async () => {
    vi.mocked(api.aiSend).mockImplementationOnce(async () => {});
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(ChatView, { bookId: "b", sceneId: null, mode: "conselheira" });
    const vm = app.mount(el) as unknown as { startIdea: (t: string) => Promise<void> };
    await flush();
    await vm.startIdea("Uma faroleira que recebe cartas");
    await flush();
    expect(api.aiConversationNewIdea).toHaveBeenCalledWith("b");
    expect(api.aiSend).toHaveBeenCalledWith("b", null, "Uma faroleira que recebe cartas", expect.any(Function));
    expect(bubbles(el)).toContain("Uma faroleira que recebe cartas");
  });

  it("startIdea right after mount is not wiped by the initial load", async () => {
    let release!: () => void;
    vi.mocked(api.aiConversation).mockImplementationOnce(() => new Promise((r) => { release = () => r({ conversation: conv("cv"), messages: [msg("user", "velha")] }); }));
    vi.mocked(api.aiSend).mockImplementationOnce(async () => {});
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(ChatView, { bookId: "b", sceneId: null, mode: "conselheira" });
    const vm = app.mount(el) as unknown as { startIdea: (t: string) => Promise<void> };
    await flush();
    const started = vm.startIdea("Minha ideia");
    await flush();
    release();
    await started;
    await flush();
    expect(bubbles(el)).toEqual(["Minha ideia"]);
  });
});
