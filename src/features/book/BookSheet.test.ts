// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";

const flush = async () => { for (let i = 0; i < 4; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
vi.mock("../../lib/api", () => ({
  bookMetaGet: vi.fn(async () => ({ author: "Hefesto", synopsis: null, dedication: null, cover_image_id: null, back_image_id: null })),
  bookMetaSet: vi.fn(async () => {}),
  imageAdd: vi.fn(async () => "img-1"),
}));
vi.mock("./cover", () => ({ generateCover: vi.fn(async () => new Uint8Array([1])), wrapTitle: (t: string) => [t] }));
import * as api from "../../lib/api";
import BookSheet from "./BookSheet.vue";

let app: App | null = null;
async function mount() {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(BookSheet, { bookId: "b", title: "Até ter certeza", color: "#2f5d50" });
  app.mount(el);
  await flush();
  return el;
}
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("Ficha do livro", () => {
  it("mostra o que já foi salvo", async () => {
    const el = await mount();
    expect((el.querySelector('[data-test="author"]') as HTMLInputElement).value).toBe("Hefesto");
  });

  it("salva autor, sinopse e dedicatória; capa gerada fica sem imagem", async () => {
    const el = await mount();
    const set = (sel: string, v: string) => {
      const f = el.querySelector(sel) as HTMLInputElement;
      f.value = v;
      f.dispatchEvent(new Event("input"));
    };
    set('[data-test="author"]', "Hefesto");
    set('[data-test="synopsis"]', "João fica milionário em segredo.");
    set('[data-test="dedication"]', "Para a Marina");
    (el.querySelector('[data-test="save"]') as HTMLButtonElement).click();
    await flush();
    expect(api.bookMetaSet).toHaveBeenCalledWith("b", {
      author: "Hefesto",
      synopsis: "João fica milionário em segredo.",
      dedication: "Para a Marina",
      cover_image_id: null,
      back_image_id: null,
    });
  });
});
