import { describe, expect, it } from "vitest";
import { matchShortcut, SHORTCUTS, label } from "./shortcuts";

const k = (key: string, mods: Partial<{ meta: boolean; ctrl: boolean; shift: boolean; alt: boolean }> = {}) => ({
  key,
  metaKey: !!mods.meta,
  ctrlKey: !!mods.ctrl,
  shiftKey: !!mods.shift,
  altKey: !!mods.alt,
});

describe("matchShortcut no Mac (Cmd)", () => {
  it("Cmd+S salva agora", () => expect(matchShortcut(k("s", { meta: true }), true)).toBe("save"));
  it("Cmd+N cria cena depois da atual", () => expect(matchShortcut(k("n", { meta: true }), true)).toBe("newScene"));
  it("Cmd+Shift+N cria capítulo", () => expect(matchShortcut(k("N", { meta: true, shift: true }), true)).toBe("newChapter"));
  it("Cmd+Option+↓ vai para a próxima cena", () => expect(matchShortcut(k("ArrowDown", { meta: true, alt: true }), true)).toBe("nextScene"));
  it("Cmd+Option+↑ volta uma cena", () => expect(matchShortcut(k("ArrowUp", { meta: true, alt: true }), true)).toBe("prevScene"));
  it("Cmd+\\ mostra/esconde o sumário", () => expect(matchShortcut(k("\\", { meta: true }), true)).toBe("toggleSidebar"));
  it("Cmd+Shift+F liga o modo foco", () => expect(matchShortcut(k("f", { meta: true, shift: true }), true)).toBe("toggleFocus"));
  it("Cmd+Shift+L troca claro/escuro", () => expect(matchShortcut(k("L", { meta: true, shift: true }), true)).toBe("toggleTheme"));
  it("Cmd+/ abre a lista de atalhos", () => expect(matchShortcut(k("/", { meta: true }), true)).toBe("help"));
  it("Cmd+Shift+I abre as ideias", () => expect(matchShortcut(k("I", { meta: true, shift: true }), true)).toBe("ideas"));
  it("Esc fecha painéis", () => expect(matchShortcut(k("Escape"), true)).toBe("close"));
  it("Ctrl+S no Mac não é atalho", () => expect(matchShortcut(k("s", { ctrl: true }), true)).toBeNull());
  it("letra sem modificador não é atalho", () => expect(matchShortcut(k("s"), true)).toBeNull());
  it("Cmd+I fica com o editor (itálico)", () => expect(matchShortcut(k("i", { meta: true }), true)).toBeNull());
});

describe("matchShortcut no Windows (Ctrl)", () => {
  it("Ctrl+S salva", () => expect(matchShortcut(k("s", { ctrl: true }), false)).toBe("save"));
  it("Ctrl+Alt+↓ próxima cena", () => expect(matchShortcut(k("ArrowDown", { ctrl: true, alt: true }), false)).toBe("nextScene"));
});

describe("label", () => {
  it("mostra símbolos no Mac e nomes no Windows", () => {
    const s = SHORTCUTS.find((x) => x.action === "newChapter")!;
    expect(label(s, true)).toBe("⌘ ⇧ N");
    expect(label(s, false)).toBe("Ctrl Shift N");
  });
  it("toda ação tem descrição em português", () => {
    for (const s of SHORTCUTS) expect(s.description.length).toBeGreaterThan(3);
  });
});
