// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { MapProposal } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  mapProposalAccept: vi.fn(async () => [{ id: "k1" }, { id: "k2" }]),
  mapProposalDiscard: vi.fn(async () => {}),
}));
import * as api from "../../lib/api";
import MapProposalCard from "./MapProposalCard.vue";

const flush = async () => { for (let i = 0; i < 3; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
const cards: MapProposal = {
  id: "p", book_id: "b", conversation_id: "cv", kind: "cartoes", status: "pendente", created_at: 0,
  payload_json: JSON.stringify({ cartoes: [{ ref: "a", texto: "Marina\nfaroleira" }, { ref: "b", texto: "A carta" }, { ref: "c", texto: "O irmão" }], setas: [{ de: "a", para: "b", rotulo: "" }] }),
};
const name: MapProposal = { ...cards, id: "n", kind: "nome", payload_json: JSON.stringify({ nome: "O Farol" }) };

let app: App | null = null;
async function mount(proposal: MapProposal, on: Record<string, (...a: unknown[]) => void> = {}) {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(MapProposalCard, { proposal, ...on });
  app.mount(el);
  await flush();
  return el;
}
const button = (el: HTMLElement, text: string) => [...el.querySelectorAll("button")].find((b) => b.textContent!.trim() === text) as HTMLButtonElement;
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("MapProposalCard", () => {
  it("lists cards checked and accepts only checked refs", async () => {
    const accepted: unknown[] = [];
    const el = await mount(cards, { onAccepted: (c) => accepted.push(c) });
    expect(el.textContent).toContain("A IA sugere 3 cartões para o mapa");
    expect(el.textContent).toContain("+ 1 ligação");
    const boxes = [...el.querySelectorAll<HTMLInputElement>("input[type=checkbox]")];
    expect(boxes.map((b) => b.checked)).toEqual([true, true, true]);
    boxes[1].click();
    await flush();
    button(el, "Pôr no mapa").click();
    await flush();
    expect(api.mapProposalAccept).toHaveBeenCalledWith("p", ["a", "c"]);
    expect(accepted).toHaveLength(1);
  });

  it("accept disabled with none checked", async () => {
    const el = await mount(cards);
    for (const b of el.querySelectorAll<HTMLInputElement>("input[type=checkbox]")) { b.click(); await flush(); }
    expect(button(el, "Pôr no mapa").disabled).toBe(true);
  });

  it("name proposal shows title and emits renamed", async () => {
    const renamed: unknown[] = [];
    const el = await mount(name, { onRenamed: (t) => renamed.push(t) });
    expect(el.textContent).toContain("Usar “O Farol” como nome do livro?");
    button(el, "Usar").click();
    await flush();
    expect(api.mapProposalAccept).toHaveBeenCalledWith("n", []);
    expect(renamed).toEqual(["O Farol"]);
  });

  it("discard calls api and emits", async () => {
    const gone: unknown[] = [];
    const el = await mount(name, { onDiscarded: () => gone.push(1) });
    button(el, "Agora não").click();
    await flush();
    expect(api.mapProposalDiscard).toHaveBeenCalledWith("n");
    expect(gone).toHaveLength(1);
  });

  it("shows error from the call in the block", async () => {
    vi.mocked(api.mapProposalAccept).mockRejectedValueOnce("proposta já resolvida");
    const el = await mount(cards);
    button(el, "Pôr no mapa").click();
    await flush();
    expect(el.textContent).toContain("proposta já resolvida");
  });
});
