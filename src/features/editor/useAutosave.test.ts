import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAutosave } from "./useAutosave";

describe("useAutosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("três schedule em 800 ms viram um save com o último valor", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("A", "1", 1);
    vi.advanceTimersByTime(300);
    a.schedule("A", "2", 2);
    vi.advanceTimersByTime(300);
    a.schedule("A", "3", 3);
    await vi.advanceTimersByTimeAsync(800);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith("A", "3", 3);
  });

  it("flush com algo pendente salva na hora", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("A", "x", 1);
    await a.flush();
    expect(save).toHaveBeenCalledWith("A", "x", 1);
    await vi.advanceTimersByTimeAsync(2000);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("flush sem nada pendente não salva", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    await useAutosave(save, 800).flush();
    expect(save).not.toHaveBeenCalled();
  });

  it("falha ao salvar mantém o conteúdo pendente para o próximo flush", async () => {
    const save = vi.fn().mockRejectedValueOnce(new Error("disco")).mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("A", "y", 1);
    await expect(a.flush()).rejects.toThrow("disco");
    await a.flush();
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("A", "y", 1);
  });

  it("cada cena grava na própria cena, mesmo agendadas em sequência", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("A", "texto A", 2);
    a.schedule("B", "texto B", 2);
    await a.flush();
    expect(save).toHaveBeenCalledWith("A", "texto A", 2);
    expect(save).toHaveBeenCalledWith("B", "texto B", 2);
    expect(save).toHaveBeenCalledTimes(2);
  });

  it("flush espera um save que já estava em andamento", async () => {
    let release!: () => void;
    const save = vi.fn().mockImplementationOnce(() => new Promise<void>((r) => (release = r)));
    const a = useAutosave(save, 800);
    a.schedule("A", "x", 1);
    await vi.advanceTimersByTimeAsync(800); // timer dispara; save fica pendurado
    let done = false;
    const f = a.flush().then(() => (done = true));
    await vi.advanceTimersByTimeAsync(0);
    expect(done).toBe(false);
    release();
    await f;
    expect(done).toBe(true);
  });

  it("save em andamento que falha volta para a fila da cena certa", async () => {
    let fail!: (e: Error) => void;
    const save = vi
      .fn()
      .mockImplementationOnce(() => new Promise<void>((_, rej) => (fail = rej)))
      .mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("A", "texto A", 1);
    await vi.advanceTimersByTimeAsync(800);
    const f = a.flush();
    fail(new Error("disco"));
    await f; // o flush re-tenta o que falhou
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("A", "texto A", 1);
  });
});
