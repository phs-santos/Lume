import { randomBytes } from "node:crypto";
import { describe, expect, it } from "vitest";
import { b64, INVITE, json, register, setup } from "./helpers.js";

describe("contas", () => {
  it("cadastro com convite certo devolve token", async () => {
    const { app } = setup();
    const res = await app.request("/api/accounts", json({ nickname: "henri", invite: INVITE, salt: b64(randomBytes(16)), secret: b64(randomBytes(32)) }));
    expect(res.status).toBe(201);
    expect(((await res.json()) as { token: string }).token.length).toBeGreaterThan(20);
  });

  it("convite errado é 403", async () => {
    const { app } = setup();
    const res = await app.request("/api/accounts", json({ nickname: "henri", invite: "x", salt: b64(randomBytes(16)), secret: b64(randomBytes(32)) }));
    expect(res.status).toBe(403);
  });

  it("apelido repetido, mesmo com outra caixa, é 409", async () => {
    const { app } = setup();
    await register(app, "Henri");
    const res = await app.request("/api/accounts", json({ nickname: "henri", invite: INVITE, salt: b64(randomBytes(16)), secret: b64(randomBytes(32)) }));
    expect(res.status).toBe(409);
  });

  it("dados malformados são 400", async () => {
    const { app } = setup();
    const res = await app.request("/api/accounts", json({ nickname: "", invite: INVITE, salt: "x", secret: "y" }));
    expect(res.status).toBe(400);
  });
});

describe("sal", () => {
  it("conta existente devolve o sal cadastrado", async () => {
    const { app } = setup();
    const { salt } = await register(app);
    const res = await app.request("/api/accounts/henri/salt");
    expect(((await res.json()) as { salt: string }).salt).toBe(b64(salt));
  });

  it("apelido inexistente tem sal falso estável de 16 bytes", async () => {
    const { app } = setup();
    const a = (await (await app.request("/api/accounts/fantasma/salt")).json()) as { salt: string };
    const b = (await (await app.request("/api/accounts/fantasma/salt")).json()) as { salt: string };
    expect(a.salt).toBe(b.salt);
    expect(Buffer.from(a.salt, "base64").length).toBe(16);
  });
});

describe("login", () => {
  it("segredo certo devolve token", async () => {
    const { app } = setup();
    const { secret } = await register(app);
    const res = await app.request("/api/sessions", json({ nickname: "henri", secret: b64(secret) }));
    expect(res.status).toBe(200);
  });

  it("segredo errado e apelido inexistente dão a mesma resposta", async () => {
    const { app } = setup();
    await register(app);
    const wrong = await app.request("/api/sessions", json({ nickname: "henri", secret: b64(randomBytes(32)) }));
    const ghost = await app.request("/api/sessions", json({ nickname: "fantasma", secret: b64(randomBytes(32)) }));
    expect(wrong.status).toBe(401);
    expect(ghost.status).toBe(401);
    expect(await wrong.json()).toEqual(await ghost.json());
    expect(await (await app.request("/api/sessions", json({ nickname: "henri", secret: b64(randomBytes(32)) }))).json()).toEqual({
      error: "Apelido ou senha incorretos",
    });
  });

  it("11ª tentativa errada em 15 min é 429; depois de 15 min volta", async () => {
    const { app, now } = setup();
    const { secret } = await register(app);
    for (let i = 0; i < 10; i++) await app.request("/api/sessions", json({ nickname: "henri", secret: b64(randomBytes(32)) }));
    expect((await app.request("/api/sessions", json({ nickname: "henri", secret: b64(secret) }))).status).toBe(429);
    now.t += 15 * 60 * 1000 + 1;
    expect((await app.request("/api/sessions", json({ nickname: "henri", secret: b64(secret) }))).status).toBe(200);
  });
});

describe("chave pública", () => {
  it("sem token é 401", async () => {
    const { app } = setup();
    expect((await app.request("/api/accounts/me/public-key")).status).toBe(401);
  });

  it("grava e lê com token; sem chave ainda é 404", async () => {
    const { app } = setup();
    const { token } = await register(app);
    const auth = { authorization: `Bearer ${token}` };
    expect((await app.request("/api/accounts/me/public-key", { headers: auth })).status).toBe(404);
    const pk = b64(randomBytes(32));
    const put = await app.request("/api/accounts/me/public-key", { method: "PUT", headers: { ...auth, "content-type": "application/json" }, body: JSON.stringify({ public_key: pk }) });
    expect(put.status).toBe(204);
    expect(await (await app.request("/api/accounts/me/public-key", { headers: auth })).json()).toEqual({ public_key: pk });
  });

  it("sessão expirada (90 dias) é 401", async () => {
    const { app, now } = setup();
    const { token } = await register(app);
    now.t += 91 * 24 * 3600 * 1000;
    expect((await app.request("/api/accounts/me/public-key", { headers: { authorization: `Bearer ${token}` } })).status).toBe(401);
  });
});

describe("banco", () => {
  it("não guarda o segredo nem o token em claro", async () => {
    const { app, db } = setup();
    const { token, secret } = await register(app);
    const dump = JSON.stringify([db.prepare("SELECT * FROM accounts").all(), db.prepare("SELECT * FROM sessions").all()]);
    expect(dump).not.toContain(b64(secret));
    expect(dump).not.toContain(token);
  });
});
