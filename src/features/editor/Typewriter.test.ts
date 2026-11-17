// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { createApp, type App } from "vue";
import Typewriter from "./Typewriter.vue";

let app: App | null = null;
function head(chapterTitle: string, sceneTitle: string) {
  const el = document.createElement("div");
  document.body.appendChild(el);
  app = createApp(Typewriter, { sceneId: "s", contentJson: "", focusMode: false, keySound: false, chapterTitle, sceneTitle, words: 0 });
  app.mount(el);
  return [...el.querySelectorAll(".running-head span")].map((x) => x.textContent!.trim()).filter(Boolean).join(" ");
}
afterEach(() => { app?.unmount(); app = null; document.body.innerHTML = ""; });

describe("cabeçalho da folha", () => {
  it("mostra a cena quando o nome difere do capítulo", () => {
    expect(head("Parte", "Cena B")).toBe("Parte Cena B");
  });
  it("não repete quando os nomes são iguais", () => {
    expect(head("Linha 340", "Linha 340")).toBe("Linha 340");
  });
});
