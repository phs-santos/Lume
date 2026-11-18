// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, nextTick, type App } from "vue";

const WORDS = Array.from({ length: 24 }, (_, i) => `palavra${i + 1}`).join(" ");
vi.mock("../../lib/api", () => ({ vaultCreate: vi.fn(async () => WORDS) }));
import CreateVault from "./CreateVault.vue";

const flush = async () => { for (let i = 0; i < 3; i++) { await new Promise((r) => setTimeout(r, 0)); await nextTick(); } };
let app: App | null = null;
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; });

function fill(el: HTMLElement, sel: string, v: string) {
  const i = el.querySelector(sel) as HTMLInputElement;
  i.value = v;
  i.dispatchEvent(new Event("input"));
}

describe("CreateVault", () => {
  it("regra da senha aparece antes de digitar e diz por que o botão está desligado", async () => {
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(CreateVault);
    app.mount(el);
    await flush();
    expect(el.textContent).toContain("Pelo menos 8 caracteres");
    fill(el, 'input[autocomplete="username"]', "Bia");
    fill(el, 'input[autocomplete="new-password"]', "curta");
    await nextTick();
    expect(el.querySelector('[data-test="why"]')!.textContent).toContain("8 caracteres");
    expect((el.querySelector("button.btn") as HTMLButtonElement).disabled).toBe(true);
  });

  it("só libera depois de conferir duas palavras", async () => {
    let done = false;
    const el = document.createElement("div");
    document.body.appendChild(el);
    app = createApp(CreateVault, { onDone: () => (done = true) });
    app.mount(el);
    await flush();
    fill(el, 'input[autocomplete="username"]', "Bia");
    const [p1, p2] = el.querySelectorAll('input[type="password"]');
    for (const p of [p1, p2]) { (p as HTMLInputElement).value = "senha-longa"; p.dispatchEvent(new Event("input")); }
    await nextTick();
    (el.querySelector("form") as HTMLFormElement).dispatchEvent(new Event("submit"));
    await flush();
    (el.querySelector('[data-test="to-check"]') as HTMLButtonElement).click();
    await nextTick();
    const labels = [...el.querySelectorAll(".field span")].map((s) => Number(s.textContent!.replace(/\D/g, "")));
    const btn = () => el.querySelector('[data-test="done"]') as HTMLButtonElement;
    fill(el, '[data-test="answer-0"]', "errada");
    fill(el, '[data-test="answer-1"]', `Palavra${labels[1]}`);
    await nextTick();
    expect(btn().disabled).toBe(true);
    fill(el, '[data-test="answer-0"]', ` palavra${labels[0]} `);
    await nextTick();
    expect(btn().disabled).toBe(false);
    btn().click();
    await flush();
    expect(done).toBe(true);
  });
});
