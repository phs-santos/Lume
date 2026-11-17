// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { Outline as OutlineT } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  chapterCreate: vi.fn(), chapterDelete: vi.fn(), chapterRename: vi.fn(async () => {}),
  sceneCreate: vi.fn(), sceneMove: vi.fn(), sceneRename: vi.fn(async () => {}),
}));
import * as api from "../../lib/api";
import Outline from "./Outline.vue";

const sc = (id: string, chapter_id: string, title: string) => ({ id, chapter_id, title, word_count: 10, position: 0 });
const outline: OutlineT = {
  book: { id: "b", title: "L", ai_mode: "conselheira", created_at: 0 },
  chapters: [
    { chapter: { id: "c1", book_id: "b", title: "Linha 340", position: 0 }, scenes: [sc("s1", "c1", "Linha 340")] },
    { chapter: { id: "c2", book_id: "b", title: "Parte", position: 1 }, scenes: [sc("s2", "c2", "Cena A"), sc("s3", "c2", "Cena B")] },
    { chapter: { id: "c3", book_id: "b", title: "Vazio", position: 2 }, scenes: [] },
  ],
};

let app: App | null = null;
function mount() {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(Outline, { outline, currentSceneId: "s1" });
  app.mount(el);
  return el;
}
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("Outline", () => {
  it("capítulo de cena única: a linha é a própria cena (arrastável), sem cabeçalho separado", () => {
    const el = mount();
    const [single] = el.querySelectorAll("section.chapter");
    expect(single.querySelector(".chapter-head")).toBeNull();
    const li = single.querySelector("li.scene")!;
    expect(li.textContent).toContain("Linha 340");
    expect(li.classList.contains("current")).toBe(true);
    expect(single.querySelector('[aria-label="Nova cena no fim do capítulo"]')).not.toBeNull();
    expect(single.querySelector('[aria-label="Apagar capítulo"]')).not.toBeNull();
  });

  it("capítulo vazio tem botão de nova cena", () => {
    const el = mount();
    const empty = el.querySelectorAll("section.chapter")[2];
    expect(empty.querySelector('[aria-label="Nova cena no fim do capítulo"]')).not.toBeNull();
  });

  it("F2 numa cena abre a renomeação", async () => {
    const el = mount();
    const btn = el.querySelectorAll("section.chapter")[1].querySelector(".scene-btn") as HTMLElement;
    btn.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true }));
    await nextTick();
    expect(el.querySelector("#rename-s2")).not.toBeNull();
  });

  it("renomear a linha única renomeia capítulo e cena", async () => {
    const el = mount();
    const btn = el.querySelector("section.chapter .scene-btn") as HTMLElement;
    btn.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true }));
    await nextTick();
    const input = el.querySelector("#rename-s1") as HTMLInputElement;
    input.value = "Novo nome";
    input.dispatchEvent(new Event("input"));
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await new Promise((r) => setTimeout(r, 0));
    expect(api.chapterRename).toHaveBeenCalledWith("c1", "Novo nome");
    expect(api.sceneRename).toHaveBeenCalledWith("s1", "Novo nome");
  });
});
