import { createHash, createHmac, randomBytes, randomUUID, timingSafeEqual } from "node:crypto";
import type { DatabaseSync } from "node:sqlite";

export const SESSION_MS = 90 * 24 * 3600 * 1000;
export const LOCK_WINDOW_MS = 15 * 60 * 1000;
export const MAX_FAILS = 10;
export const WRONG = "Apelido ou senha incorretos";

const sha256 = (b: Uint8Array | string) => createHash("sha256").update(b).digest("hex");

export class Accounts {
  private fails = new Map<string, number[]>(); // apelido (minúsculo) → horários das falhas

  constructor(private db: DatabaseSync, private invite: string, private now: () => number) {}

  register(nickname: string, invite: string, salt: Buffer, secret: Buffer): { token: string } | "forbidden" | "taken" {
    if (invite !== this.invite) return "forbidden";
    const exists = this.db.prepare("SELECT 1 FROM accounts WHERE nickname = ?").get(nickname);
    if (exists) return "taken";
    const id = randomUUID();
    this.db
      .prepare("INSERT INTO accounts(id,nickname,salt,secret_hash,created_at) VALUES(?,?,?,?,?)")
      .run(id, nickname, salt, sha256(secret), this.now());
    return { token: this.newSession(id) };
  }

  /** Sal da conta, ou um sal falso estável para apelidos que não existem (não revela quem tem conta). */
  salt(nickname: string): Buffer {
    const row = this.db.prepare("SELECT salt FROM accounts WHERE nickname = ?").get(nickname) as { salt: Uint8Array } | undefined;
    if (row) return Buffer.from(row.salt);
    return createHmac("sha256", this.invite).update(nickname.toLowerCase()).digest().subarray(0, 16);
  }

  login(nickname: string, secret: Buffer): { token: string } | "wrong" | "locked" {
    const key = nickname.toLowerCase();
    const t = this.now();
    const recent = (this.fails.get(key) ?? []).filter((x) => t - x < LOCK_WINDOW_MS);
    this.fails.set(key, recent);
    if (recent.length >= MAX_FAILS) return "locked";
    const row = this.db.prepare("SELECT id, secret_hash FROM accounts WHERE nickname = ?").get(nickname) as
      | { id: string; secret_hash: string }
      | undefined;
    const given = Buffer.from(sha256(secret), "hex");
    const ok = row ? timingSafeEqual(given, Buffer.from(row.secret_hash, "hex")) : false;
    if (!row || !ok) {
      recent.push(t);
      return "wrong";
    }
    this.fails.delete(key);
    return { token: this.newSession(row.id) };
  }

  /** Conta dona do token, se a sessão existir e não tiver expirado. */
  accountFor(token: string): string | null {
    const row = this.db.prepare("SELECT account_id, expires_at FROM sessions WHERE token_hash = ?").get(sha256(token)) as
      | { account_id: string; expires_at: number }
      | undefined;
    if (!row || row.expires_at <= this.now()) return null;
    return row.account_id;
  }

  setPublicKey(accountId: string, pk: Buffer) {
    this.db.prepare("UPDATE accounts SET public_key = ? WHERE id = ?").run(pk, accountId);
  }

  publicKey(accountId: string): Buffer | null {
    const row = this.db.prepare("SELECT public_key FROM accounts WHERE id = ?").get(accountId) as { public_key: Uint8Array | null } | undefined;
    return row?.public_key ? Buffer.from(row.public_key) : null;
  }

  private newSession(accountId: string): string {
    const token = randomBytes(32).toString("base64url");
    const t = this.now();
    this.db
      .prepare("INSERT INTO sessions(token_hash,account_id,created_at,expires_at) VALUES(?,?,?,?)")
      .run(sha256(token), accountId, t, t + SESSION_MS);
    return token;
  }
}
