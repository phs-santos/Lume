import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAutosave } from "./useAutosave";

describe("useAutosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("três schedule em 800 ms viram um save com o último valor", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("1", 1);
    vi.advanceTimersByTime(300);
    a.schedule("2", 2);
    vi.advanceTimersByTime(300);
    a.schedule("3", 3);
    await vi.advanceTimersByTimeAsync(800);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith("3", 3);
  });

  it("flush com algo pendente salva na hora", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const a = useAutosave(save, 800);
    a.schedule("x", 1);
    await a.flush();
    expect(save).toHaveBeenCalledWith("x", 1);
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
    a.schedule("y", 1);
    await expect(a.flush()).rejects.toThrow("disco");
    await a.flush();
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("y", 1);
  });
});
