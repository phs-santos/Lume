// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { createApp, h, nextTick, ref, type App } from "vue";
import PasswordInput from "./PasswordInput.vue";

let app: App | null = null;
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; });

async function mount() {
  const value = ref("segredo");
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp({ render: () => h(PasswordInput, { modelValue: value.value, "onUpdate:modelValue": (v: string) => (value.value = v), autocomplete: "current-password" }) });
  app.mount(el);
  await nextTick();
  return { el, value };
}

describe("PasswordInput", () => {
  it("começa escondida e o olho mostra e esconde", async () => {
    const { el } = await mount();
    const input = el.querySelector("input")!;
    const eye = el.querySelector("button")!;
    expect(input.type).toBe("password");
    expect(eye.getAttribute("aria-label")).toBe("Mostrar senha");
    eye.click();
    await nextTick();
    expect(input.type).toBe("text");
    expect(eye.getAttribute("aria-label")).toBe("Esconder senha");
    eye.click();
    await nextTick();
    expect(input.type).toBe("password");
  });

  it("o olho não envia o formulário", async () => {
    const { el } = await mount();
    expect(el.querySelector("button")!.getAttribute("type")).toBe("button");
  });

  it("digitar atualiza o valor", async () => {
    const { el, value } = await mount();
    const input = el.querySelector("input")!;
    input.value = "nova";
    input.dispatchEvent(new Event("input"));
    expect(value.value).toBe("nova");
  });
});
