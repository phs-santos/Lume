import { createStore, del, entries, set, update } from "idb-keyval";
import { ApiError } from "./api";

/** Ideia já cifrada esperando para subir. Fica no aparelho até o servidor confirmar. */
export interface Pending {
  id: string;
  kind: "audio" | "text";
  mime: string;
  body: Uint8Array;
  createdAt: number;
  attempts: number;
  /** Conta para a qual foi cifrada: só sobe com a sessão dessa conta. */
  nickname: string;
}

export interface Failed extends Pending {
  reason: string;
}

/** A sessão expirou: é preciso entrar de novo. As ideias continuam guardadas. */
export class AuthExpired extends Error {}

const queue = () => createStore("lume-fila", "ideias");
const failures = () => createStore("lume-falhas", "ideias");
let running: Promise<{ sent: number; left: number }> | null = null;

export async function enqueue(p: Omit<Pending, "id" | "attempts">): Promise<Pending> {
  const item: Pending = { ...p, id: crypto.randomUUID(), attempts: 0 };
  await set(item.id, item, queue());
  return item;
}

export async function pending(): Promise<Pending[]> {
  const all = (await entries<string, Pending>(queue())).map(([, v]) => v);
  return all.sort((a, b) => a.createdAt - b.createdAt);
}

export async function failed(): Promise<Failed[]> {
  return (await entries<string, Failed>(failures())).map(([, v]) => v).sort((a, b) => a.createdAt - b.createdAt);
}

export async function discardFailed(id: string) {
  await del(id, failures());
}

/**
 * Envia, em ordem, as ideias da conta logada.
 * - rede caiu ou servidor com problema (5xx): para e tenta depois;
 * - sessão expirada (401): para e lança `AuthExpired`;
 * - recusa definitiva (outros 4xx, como 413): tira da frente e guarda em "falhas", sem travar as outras.
 */
export function flush(nickname: string, send: (p: Pending) => Promise<void>): Promise<{ sent: number; left: number }> {
  running ??= (async () => {
    let sent = 0;
    const mine = (await pending()).filter((p) => p.nickname === nickname);
    let left = mine.length;
    for (const p of mine) {
      try {
        await send(p);
        await del(p.id, queue());
        sent++;
        left--;
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) throw new AuthExpired(e.message);
        if (e instanceof ApiError && e.status >= 400 && e.status < 500) {
          await set(p.id, { ...p, reason: e.message } satisfies Failed, failures());
          await del(p.id, queue());
          left--;
          continue;
        }
        await update<Pending>(p.id, (v) => (v ? { ...v, attempts: v.attempts + 1 } : v!), queue());
        break;
      }
    }
    return { sent, left };
  })().finally(() => (running = null));
  return running;
}
