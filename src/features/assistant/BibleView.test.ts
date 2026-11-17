// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { BibleEntry } from "../../lib/types";

const flush = async () => { for (let i = 0; i < 4; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
const entry = (id: string, kind: BibleEntry["kind"], name: string, extra: Partial<BibleEntry> = {}): BibleEntry => ({
  id, book_id: "b", kind, name, aliases: [], body: `Sobre ${name}`, ai_notes: [], story_order: null,
  source_scene_id: null, edited_by_user: false, updated_at: 0, ...extra,
});
const ENTRIES = [
  entry("e1", "personagem", "Rafa", { aliases: ["Rafael"], source_scene_id: "s2" }),
  entry("e2", "lugar", "Padaria"),
  entry("e3", "personagem", "Marina", { edited_by_user: true, ai_notes: [{ fact: "Tem um irmão", source_scene_id: null }] }),
  entry("t1", "linha_do_tempo", "Mudança", { story_order: 1 }),
  entry("t2", "linha_do_tempo", "Descoberta", { story_order: 2 }),
];
vi.mock("../../lib/api", () => ({
  bibleList: vi.fn(async () => ENTRIES),
  bibleUpdate: vi.fn(async (id: string) => ENTRIES.find((e) => e.id === id)),
  bibleAbsorbNote: vi.fn(async (id: string) => ({ ...ENTRIES.find((e) => e.id === id)!, ai_notes: [], body: "Sobre Marina\nTem um irmão" })),
  bibleDelete: vi.fn(async () => {}),
}));
import * as api from "../../lib/api";
import BibleView from "./BibleView.vue";

let app: App | null = null;
let opened: string[] = [];
let exposed: any = null;
async function mount() {
  const el = document.createElement("div");
  document.body.appendChild(el);
  opened = [];
  app = createApp(BibleView, { bookId: "b", sceneTitles: { s2: "A Porta do Elevador" }, onOpenScene: (id: string) => opened.push(id) });
  exposed = app.mount(el);
  await flush();
  return el;
}
const names = (el: HTMLElement) => [...el.querySelectorAll(".entry-name")].map((n) => n.textContent!.trim());
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("BibleView", () => {
  it("agrupa por tipo e busca também por apelido", async () => {
    const el = await mount();
    expect([...el.querySelectorAll(".group h4")].map((h) => h.textContent!.trim().split(" ")[0])).toEqual(["Personagens", "Lugares", "Linha"]);
    expect(names(el)).toEqual(["Marina", "Rafa", "Padaria", "Mudança", "Descoberta"]);
    const q = el.querySelector('input[type="search"]') as HTMLInputElement;
    q.value = "rafael";
    q.dispatchEvent(new Event("input"));
    await flush();
    expect(names(el)).toEqual(["Rafa"]);
  });

  it("editar salva com bibleUpdate", async () => {
    const el = await mount();
    (el.querySelector('[data-id="e2"] .entry-head') as HTMLElement).click();
    await flush();
    const body = el.querySelector('[data-id="e2"] textarea') as HTMLTextAreaElement;
    body.value = "Abre às seis";
    body.dispatchEvent(new Event("input"));
    const al = el.querySelector('[data-id="e2"] input[name="aliases"]') as HTMLInputElement;
    al.value = "padoca, a da esquina";
    al.dispatchEvent(new Event("input"));
    await nextTick();
    (el.querySelector('[data-id="e2"] [data-test="save"]') as HTMLButtonElement).click();
    await flush();
    expect(api.bibleUpdate).toHaveBeenCalledWith("e2", "Padaria", ["padoca", "a da esquina"], "Abre às seis", null);
  });

  it("mostra notas da IA separadas e incorpora", async () => {
    const el = await mount();
    (el.querySelector('[data-id="e3"] .entry-head') as HTMLElement).click();
    await flush();
    const notes = el.querySelector('[data-id="e3"] .notes')!;
    expect(notes.textContent).toContain("Tem um irmão");
    (notes.querySelector("button") as HTMLButtonElement).click();
    await flush();
    expect(api.bibleAbsorbNote).toHaveBeenCalledWith("e3", 0);
  });

  it("reordenar a linha do tempo grava a nova ordem", async () => {
    await mount();
    await exposed.reorderTimeline(["t2", "t1"]);
    expect(api.bibleUpdate).toHaveBeenCalledWith("t2", "Descoberta", [], "Sobre Descoberta", 1);
    expect(api.bibleUpdate).toHaveBeenCalledWith("t1", "Mudança", [], "Sobre Mudança", 2);
  });

  it("origem abre a cena", async () => {
    const el = await mount();
    (el.querySelector('[data-id="e1"] .entry-head') as HTMLElement).click();
    await flush();
    const link = el.querySelector('[data-id="e1"] [data-test="source"]') as HTMLButtonElement;
    expect(link.textContent).toContain("A Porta do Elevador");
    link.click();
    expect(opened).toEqual(["s2"]);
  });
});
