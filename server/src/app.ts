import type { DatabaseSync } from "node:sqlite";
import { Hono } from "hono";
import { bodyLimit } from "hono/body-limit";
import { Accounts, NICK, type Scope, WRONG } from "./accounts.js";
import { DEFAULT_QUOTA, Inbox, KINDS, MAX_ENVELOPE } from "./inbox.js";

type Env = { Variables: { accountId: string; scope: Scope; token: string } };

/** Decodifica base64 e confere o tamanho; `null` se inválido. */
export function b64(value: unknown, len?: number): Buffer | null {
  if (typeof value !== "string" || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) return null;
  const b = Buffer.from(value, "base64");
  return len === undefined || b.length === len ? b : null;
}

/** Corpo das rotas JSON: nada legítimo passa de 4 KB. */
const JSON_LIMIT = 4096;

export function createApp(
  db: DatabaseSync,
  opts: { inviteCode: string; now?: () => number; quota?: { count: number; bytes: number } },
) {
  const quota = opts.quota ?? DEFAULT_QUOTA;
  const now = opts.now ?? Date.now;
  const accounts = new Accounts(db, opts.inviteCode, now);
  const inbox = new Inbox(db, now);
  const app = new Hono<Env>();
  (app as any).accounts = accounts; // acesso nos testes

  app.get("/api/health", (c) => c.json({ ok: true }));

  const small = bodyLimit({ maxSize: JSON_LIMIT, onError: (c) => c.json({ error: "Pedido grande demais." }, 413) });

  const auth = async (c: any, next: () => Promise<void>) => {
    const h = c.req.header("authorization") ?? "";
    const token = h.startsWith("Bearer ") ? h.slice(7) : "";
    const s = token ? accounts.sessionFor(token) : null;
    if (!s) return c.json({ error: "Entre de novo." }, 401);
    c.set("accountId", s.accountId);
    c.set("scope", s.scope);
    c.set("token", token);
    await next();
  };

  /** Só o computador (sessão "full") lê, apaga e troca a chave. */
  const full = async (c: any, next: () => Promise<void>) => {
    if (c.get("scope") !== "full") return c.json({ error: "Só o computador pode fazer isso." }, 403);
    await next();
  };

  app.post("/api/accounts", small, async (c) => {
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

  app.post("/api/sessions", small, async (c) => {
    const body = await c.req.json().catch(() => ({}));
    const secret = b64(body.secret, 32);
    if (typeof body.nickname !== "string" || !NICK.test(body.nickname) || !secret) return c.json({ error: WRONG }, 401);
    const scope: Scope = body.scope === "full" ? "full" : "capture";
    const r = accounts.login(body.nickname, secret, scope);
    if (r === "locked") return c.json({ error: "Muitas tentativas. Espere 15 minutos." }, 429);
    if (r === "wrong") return c.json({ error: WRONG }, 401);
    return c.json(r);
  });

  app.delete("/api/sessions/current", auth, (c) => {
    accounts.logout(c.get("token"));
    return c.body(null, 204);
  });

  app.put("/api/accounts/me/public-key", small, auth, full, async (c) => {
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

  app.post(
    "/api/inbox",
    auth,
    bodyLimit({ maxSize: MAX_ENVELOPE, onError: (c) => c.json({ error: "A ideia passou de 25 MB." }, 413) }),
    async (c) => {
      const kind = c.req.header("x-lume-kind") ?? "";
      const mime = (c.req.header("content-type") ?? "").slice(0, 120);
      if (!(KINDS as readonly string[]).includes(kind) || !mime) return c.json({ error: "Tipo inválido." }, 400);
      const body = Buffer.from(await c.req.arrayBuffer());
      if (body.length === 0) return c.json({ error: "Envelope vazio." }, 400);
      const used = inbox.usage(c.get("accountId"));
      if (used.count + 1 > quota.count || used.bytes + body.length > quota.bytes)
        return c.json({ error: "A caixa de entrada está cheia. Abra o Lume no computador para receber as ideias." }, 507);
      return c.json({ id: inbox.add(c.get("accountId"), kind, mime, body) }, 201);
    },
  );

  app.get("/api/inbox", auth, full, (c) => c.json(inbox.list(c.get("accountId"))));

  app.get("/api/inbox/:id", auth, full, (c) => {
    const e = inbox.get(c.get("accountId"), c.req.param("id"));
    if (!e) return c.json({ error: "Não encontrado." }, 404);
    return c.body(e.body as Uint8Array<ArrayBuffer>, 200, {
      "content-type": "application/octet-stream",
      "x-lume-kind": e.kind,
      "x-lume-mime": e.mime,
    });
  });

  app.delete("/api/inbox/:id", auth, full, (c) =>
    inbox.remove(c.get("accountId"), c.req.param("id")) ? c.body(null, 204) : c.json({ error: "Não encontrado." }, 404),
  );

  app.all("/api/*", (c) => c.json({ error: "Não encontrado." }, 404));

  return app;
}
