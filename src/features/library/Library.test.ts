// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";
import type { Book } from "../../lib/types";

const book = (over: Partial<Book> = {}): Book => ({ id: "b1", title: "Livro sem nome", ai_mode: "conselheira", created_at: 0, title_provisional: true, ...over });

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../lib/api", () => ({
  bookList: vi.fn(async () => []),
  bookCreate: vi.fn(async () => book()),
  bookImportEpub: vi.fn(),
  vaultLock: vi.fn(),
  aiKeyStatus: vi.fn(async () => true),
}));
import * as api from "../../lib/api";
import Library from "./Library.vue";

const flush = async () => { for (let i = 0; i < 4; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
let app: App | null = null;
async function mount(on: Record<string, (...a: unknown[]) => void> = {}) {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(Library, on);
  app.mount(el);
  await flush();
  return el;
}
const byText = (el: HTMLElement, sel: string, text: string) => [...el.querySelectorAll<HTMLElement>(sel)].find((b) => b.textContent!.includes(text))!;
async function openDialog(el: HTMLElement) {
  byText(el, "button", "Comece seu primeiro livro").click();
  await flush();
}
const ideaRadio = (el: HTMLElement) => el.querySelector<HTMLInputElement>('input[name="start"][value="ideia"]')!;
const createBtn = (el: HTMLElement) => byText(el, "button", "Criar livro") as HTMLButtonElement;
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; vi.clearAllMocks(); });

describe("Novo livro", () => {
  it("creates with empty name", async () => {
    const opened: unknown[][] = [];
    const el = await mount({ onOpen: (...a: unknown[]) => opened.push(a) });
    await openDialog(el);
    expect(el.textContent).toContain("Pode ser um nome provisório. Dá para trocar depois.");
    expect(createBtn(el).disabled).toBe(false);
    createBtn(el).click();
    await flush();
    expect(api.bookCreate).toHaveBeenCalledWith("", "conselheira");
    expect(opened[0]).toEqual(["b1", undefined]);
  });

  it("idea option disabled without ai key", async () => {
    vi.mocked(api.aiKeyStatus).mockResolvedValueOnce(false);
    const el = await mount();
    await openDialog(el);
    expect(ideaRadio(el).disabled).toBe(true);
    expect(el.textContent).toContain("Para usar, configure a IA em Ajustes do assistente");
  });

  it("idea option requires text and emits open with idea", async () => {
    const opened: unknown[][] = [];
    const el = await mount({ onOpen: (...a: unknown[]) => opened.push(a) });
    await openDialog(el);
    ideaRadio(el).click();
    await flush();
    const ta = el.querySelector<HTMLTextAreaElement>("textarea")!;
    expect(ta.placeholder).toBe("Conte a ideia do jeito que vier…");
    expect(createBtn(el).disabled).toBe(true);
    ta.value = "  Uma faroleira que recebe cartas  ";
    ta.dispatchEvent(new Event("input"));
    await flush();
    createBtn(el).click();
    await flush();
    expect(opened[0]).toEqual(["b1", { idea: "Uma faroleira que recebe cartas" }]);
  });

  it("provisional cover shows tag", async () => {
    vi.mocked(api.bookList).mockResolvedValueOnce([book(), book({ id: "b2", title: "O Farol", title_provisional: false })]);
    const el = await mount();
    const covers = [...el.querySelectorAll(".cover:not(.new)")];
    expect(covers[0].textContent).toContain("nome provisório");
    expect(covers[1].textContent).not.toContain("nome provisório");
  });
});
