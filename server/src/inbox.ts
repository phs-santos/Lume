import { randomUUID } from "node:crypto";
import type { DatabaseSync } from "node:sqlite";

export const MAX_ENVELOPE = 25 * 1024 * 1024;
export const KINDS = ["audio", "text"] as const;
/** Cota por conta: protege o disco da VPS, que é dividido com outros sites. */
export const DEFAULT_QUOTA = { count: 500, bytes: 500 * 1024 * 1024 };

export interface EnvelopeMeta { id: string; kind: string; mime: string; size: number; created_at: number }

export class Inbox {
  constructor(private db: DatabaseSync, private now: () => number) {}

  add(accountId: string, kind: string, mime: string, body: Buffer): string {
    const id = randomUUID();
    this.db
      .prepare("INSERT INTO envelopes(id,account_id,kind,mime,body,created_at) VALUES(?,?,?,?,?,?)")
      .run(id, accountId, kind, mime, body, this.now());
    return id;
  }

  usage(accountId: string): { count: number; bytes: number } {
    return this.db
      .prepare("SELECT count(*) AS count, coalesce(sum(length(body)),0) AS bytes FROM envelopes WHERE account_id = ?")
      .get(accountId) as { count: number; bytes: number };
  }

  list(accountId: string): EnvelopeMeta[] {
    return this.db
      .prepare("SELECT id, kind, mime, length(body) AS size, created_at FROM envelopes WHERE account_id = ? ORDER BY created_at, rowid")
      .all(accountId) as unknown as EnvelopeMeta[];
  }

  get(accountId: string, id: string): { kind: string; mime: string; body: Uint8Array } | null {
    const row = this.db
      .prepare("SELECT kind, mime, body FROM envelopes WHERE id = ? AND account_id = ?")
      .get(id, accountId) as { kind: string; mime: string; body: Uint8Array } | undefined;
    return row ?? null;
  }

  remove(accountId: string, id: string): boolean {
    return Number(this.db.prepare("DELETE FROM envelopes WHERE id = ? AND account_id = ?").run(id, accountId).changes) > 0;
  }
}
