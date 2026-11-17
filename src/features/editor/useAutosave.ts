type Save = (json: string, words: number) => Promise<void>;

/**
 * Salvamento com espera: agrupa edições seguidas num único `save`.
 * `flush()` grava o que estiver pendente na hora (troca de cena, fechar janela).
 */
export function useAutosave(save: Save, delayMs = 800) {
  let pending: { json: string; words: number } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  async function flush() {
    if (timer) clearTimeout(timer);
    timer = null;
    if (!pending) return;
    const p = pending;
    pending = null;
    try {
      await save(p.json, p.words);
    } catch (e) {
      // Não perde o texto: se nada mais novo chegou, volta a ficar pendente.
      pending ??= p;
      throw e;
    }
  }

  function schedule(json: string, words: number) {
    pending = { json, words };
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      flush().catch(() => {});
    }, delayMs);
  }

  return { schedule, flush };
}
