import { randomBytes } from "node:crypto";
import { createApp } from "../src/app.js";
import { openDb } from "../src/db.js";

export const INVITE = "convite-teste";
export const b64 = (b: Uint8Array) => Buffer.from(b).toString("base64");

export function setup(now = { t: 1_000_000 }) {
  const db = openDb(":memory:");
  const app = createApp(db, { inviteCode: INVITE, now: () => now.t });
  return { db, app, now };
}

export const json = (body: unknown, token?: string): RequestInit => ({
  method: "POST",
  headers: { "content-type": "application/json", ...(token ? { authorization: `Bearer ${token}` } : {}) },
  body: JSON.stringify(body),
});

/** Cadastra uma conta e devolve token + segredo usado. */
export async function register(app: ReturnType<typeof setup>["app"], nickname = "henri") {
  const salt = randomBytes(16);
  const secret = randomBytes(32);
  const res = await app.request("/api/accounts", json({ nickname, invite: INVITE, salt: b64(salt), secret: b64(secret) }));
  const { token } = (await res.json()) as { token: string };
  return { token, salt, secret };
}
