import type { DatabaseSync } from "node:sqlite";
import { Hono } from "hono";
import { Accounts, WRONG } from "./accounts.js";

type Env = { Variables: { accountId: string } };

/** Decodifica base64 e confere o tamanho; `null` se inválido. */
export function b64(value: unknown, len?: number): Buffer | null {
  if (typeof value !== "string" || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) return null;
  const b = Buffer.from(value, "base64");
  return len === undefined || b.length === len ? b : null;
}

const NICK = /^[\p{L}\p{N}._-]{2,32}$/u;

export function createApp(db: DatabaseSync, opts: { inviteCode: string; now?: () => number }) {
  const now = opts.now ?? Date.now;
  const accounts = new Accounts(db, opts.inviteCode, now);
  const app = new Hono<Env>();

  const auth = async (c: any, next: () => Promise<void>) => {
    const h = c.req.header("authorization") ?? "";
    const id = h.startsWith("Bearer ") ? accounts.accountFor(h.slice(7)) : null;
    if (!id) return c.json({ error: "Entre de novo." }, 401);
    c.set("accountId", id);
    await next();
  };

  app.post("/api/accounts", async (c) => {
    const body = await c.req.json().catch(() => ({}));
    const salt = b64(body.salt, 16);
    const secret = b64(body.secret, 32);
    if (typeof body.nickname !== "string" || !NICK.test(body.nickname) || !salt || !secret || typeof body.invite !== "string")
      return c.json({ error: "Dados inválidos." }, 400);
    const r = accounts.register(body.nickname, body.invite, salt, secret);
    if (r === "forbidden") return c.json({ error: "Código de convite incorreto." }, 403);
    if (r === "taken") return c.json({ error: "Esse apelido já está em uso." }, 409);
    return c.json(r, 201);
  });

  app.get("/api/accounts/:nickname/salt", (c) => c.json({ salt: accounts.salt(c.req.param("nickname")).toString("base64") }));

  app.post("/api/sessions", async (c) => {
    const body = await c.req.json().catch(() => ({}));
    const secret = b64(body.secret, 32);
    if (typeof body.nickname !== "string" || !secret) return c.json({ error: WRONG }, 401);
    const r = accounts.login(body.nickname, secret);
    if (r === "locked") return c.json({ error: "Muitas tentativas. Espere 15 minutos." }, 429);
    if (r === "wrong") return c.json({ error: WRONG }, 401);
    return c.json(r);
  });

  app.put("/api/accounts/me/public-key", auth, async (c) => {
    const body = await c.req.json().catch(() => ({}));
    const pk = b64(body.public_key, 32);
    if (!pk) return c.json({ error: "Chave inválida." }, 400);
    accounts.setPublicKey(c.get("accountId"), pk);
    return c.body(null, 204);
  });

  app.get("/api/accounts/me/public-key", auth, (c) => {
    const pk = accounts.publicKey(c.get("accountId"));
    if (!pk) return c.json({ error: "Este computador ainda não foi conectado." }, 404);
    return c.json({ public_key: pk.toString("base64") });
  });

  return app;
}
