import { createStore, del, entries, set, update } from "idb-keyval";

/** Ideia já cifrada esperando para subir. Fica no aparelho até o servidor confirmar. */
export interface Pending {
  id: string;
  kind: "audio" | "text";
  mime: string;
  body: Uint8Array;
  createdAt: number;
  attempts: number;
}

const store = () => createStore("lume-fila", "ideias");
let running: Promise<{ sent: number; left: number }> | null = null;

export async function enqueue(p: Omit<Pending, "id" | "attempts">): Promise<Pending> {
  const item: Pending = { ...p, id: crypto.randomUUID(), attempts: 0 };
  await set(item.id, item, store());
  return item;
}

export async function pending(): Promise<Pending[]> {
  const all = (await entries<string, Pending>(store())).map(([, v]) => v);
  return all.sort((a, b) => a.createdAt - b.createdAt);
}

/** Envia o que estiver na fila, em ordem. Para no primeiro erro (sem rede, provavelmente). */
export function flush(send: (p: Pending) => Promise<void>): Promise<{ sent: number; left: number }> {
  running ??= (async () => {
    let sent = 0;
    const items = await pending();
    for (const p of items) {
      try {
        await send(p);
        await del(p.id, store());
        sent++;
      } catch {
        await update<Pending>(p.id, (v) => (v ? { ...v, attempts: v.attempts + 1 } : v!), store());
        break;
      }
    }
    return { sent, left: items.length - sent };
  })().finally(() => (running = null));
  return running;
}
