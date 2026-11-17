type Save = (sceneId: string, json: string, words: number) => Promise<void>;
type Payload = { json: string; words: number };

/**
 * Salvamento com espera, por cena: agrupa edições seguidas num único `save`
 * e sempre grava na cena de onde a edição veio.
 * `flush()` grava tudo o que estiver pendente e espera o que já estava em andamento.
 */
export function useAutosave(save: Save, delayMs = 800) {
  const pending = new Map<string, Payload>();
  let timer: ReturnType<typeof setTimeout> | null = null;
  let inflight: Promise<void> = Promise.resolve();

  function drain(): Promise<void> {
    if (timer) clearTimeout(timer);
    timer = null;
    const batch = [...pending];
    pending.clear();
    const run = inflight.catch(() => {}).then(async () => {
      let firstError: unknown = null;
      for (const [id, p] of batch) {
        try {
          await save(id, p.json, p.words);
        } catch (e) {
          // Não perde o texto: se nada mais novo chegou para essa cena, volta para a fila.
          if (!pending.has(id)) pending.set(id, p);
          firstError ??= e;
        }
      }
      if (firstError) throw firstError;
    });
    inflight = run;
    return run;
  }

  async function flush() {
    await inflight.catch(() => {});
    if (pending.size) await drain();
    else await inflight;
  }

  function schedule(sceneId: string, json: string, words: number) {
    pending.set(sceneId, { json, words });
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      drain().catch(() => {});
    }, delayMs);
  }

  return { schedule, flush };
}
