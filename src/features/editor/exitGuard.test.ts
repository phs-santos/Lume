import { beforeEach, describe, expect, it, vi } from "vitest";
import { canExit, handleCloseRequest, registerFlusher, resetFlushers } from "./exitGuard";

describe("exitGuard", () => {
  beforeEach(() => resetFlushers());

  it("sem nada registrado pode sair", async () => {
    expect(await canExit()).toBe(true);
  });

  it("um flusher que falha impede a saída", async () => {
    registerFlusher(async () => true);
    registerFlusher(async () => false);
    expect(await canExit()).toBe(false);
  });

  it("desregistrar remove o flusher", async () => {
    const off = registerFlusher(async () => false);
    off();
    expect(await canExit()).toBe(true);
  });

  it("fechar a janela com save falhando cancela o fechamento", async () => {
    registerFlusher(async () => false);
    const ev = { preventDefault: vi.fn() };
    await handleCloseRequest(ev);
    expect(ev.preventDefault).toHaveBeenCalled();
  });

  it("fechar a janela com tudo salvo deixa fechar", async () => {
    registerFlusher(async () => true);
    const ev = { preventDefault: vi.fn() };
    await handleCloseRequest(ev);
    expect(ev.preventDefault).not.toHaveBeenCalled();
  });
});
