/**
 * Antes de fechar a janela ou sair do app, cada tela com texto pendente grava o que tiver.
 * Um flusher devolve `false` se não conseguiu salvar: aí a saída é cancelada.
 */
type Flusher = () => Promise<boolean>;
const flushers = new Set<Flusher>();

export function registerFlusher(f: Flusher): () => void {
  flushers.add(f);
  return () => flushers.delete(f);
}

export function resetFlushers() {
  flushers.clear();
}

export async function canExit(): Promise<boolean> {
  const results = await Promise.all([...flushers].map((f) => f().catch(() => false)));
  return results.every(Boolean);
}

export async function handleCloseRequest(ev: { preventDefault(): void }): Promise<void> {
  if (!(await canExit())) ev.preventDefault();
}
