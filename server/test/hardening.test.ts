import { randomBytes } from "node:crypto";
import { describe, expect, it } from "vitest";
import { createApp } from "../src/app.js";
import { openDb } from "../src/db.js";
import { b64, INVITE, json, register, setup } from "./helpers.js";

const bearer = (t: string, extra: Record<string, string> = {}) => ({ authorization: `Bearer ${t}`, ...extra });

describe("limites de corpo nas rotas JSON (I1)", () => {
  it.each(["/api/sessions", "/api/accounts"])("%s recusa corpo acima de 4 KB com 413", async (path) => {
    const { app } = setup();
    const res = await app.request(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ x: "a".repeat(10_000) }) });
    expect(res.status).toBe(413);
  });
});

describe("tabela de tentativas não cresce sem limite (I2)", () => {
  it("apelido inválido não entra na tabela", async () => {
    const { app } = setup();
    await app.request("/api/sessions", json({ nickname: "x".repeat(5000), secret: b64(randomBytes(32)) }));
    await app.request("/api/sessions", json({ nickname: "", secret: b64(randomBytes(32)) }));
    expect((app as any).accounts.tracked()).toBe(0);
  });

  it("entradas velhas são descartadas", async () => {
    const { app, now } = setup();
    for (let i = 0; i < 50; i++) await app.request("/api/sessions", json({ nickname: `fantasma${i}`, secret: b64(randomBytes(32)) }));
    expect((app as any).accounts.tracked()).toBe(50);
    now.t += 16 * 60 * 1000;
    await app.request("/api/sessions", json({ nickname: "outro", secret: b64(randomBytes(32)) }));
    expect((app as any).accounts.tracked()).toBe(1);
  });
});

describe("sessões do celular têm poder limitado e podem sair (I5)", () => {
  async function phone(app: ReturnType<typeof setup>["app"]) {
    const { secret, token: full } = await register(app);
    const res = await app.request("/api/sessions", json({ nickname: "henri", secret: b64(secret), scope: "capture" }));
    return { full, phone: ((await res.json()) as { token: string }).token };
  }

  it("token do celular envia ideia e lê a chave pública", async () => {
    const { app } = setup();
    const { full, phone: t } = await phone(app);
    await app.request("/api/accounts/me/public-key", { method: "PUT", headers: bearer(full, { "content-type": "application/json" }), body: JSON.stringify({ public_key: b64(randomBytes(32)) }) });
    expect((await app.request("/api/accounts/me/public-key", { headers: bearer(t) })).status).toBe(200);
    const post = await app.request("/api/inbox", { method: "POST", headers: bearer(t, { "x-lume-kind": "text", "content-type": "text/plain" }), body: new Uint8Array([1]) });
    expect(post.status).toBe(201);
  });

  it("token do celular não troca a chave, não lista e não apaga (403)", async () => {
    const { app } = setup();
    const { full, phone: t } = await phone(app);
    const { id } = (await (await app.request("/api/inbox", { method: "POST", headers: bearer(full, { "x-lume-kind": "text", "content-type": "text/plain" }), body: new Uint8Array([1]) })).json()) as any;
    expect((await app.request("/api/accounts/me/public-key", { method: "PUT", headers: bearer(t, { "content-type": "application/json" }), body: JSON.stringify({ public_key: b64(randomBytes(32)) }) })).status).toBe(403);
    expect((await app.request("/api/inbox", { headers: bearer(t) })).status).toBe(403);
    expect((await app.request(`/api/inbox/${id}`, { headers: bearer(t) })).status).toBe(403);
    expect((await app.request(`/api/inbox/${id}`, { method: "DELETE", headers: bearer(t) })).status).toBe(403);
  });

  it("escopo desconhecido é tratado como celular", async () => {
    const { app } = setup();
    const { secret } = await register(app);
    const t = ((await (await app.request("/api/sessions", json({ nickname: "henri", secret: b64(secret), scope: "root" }))).json()) as any).token;
    expect((await app.request("/api/inbox", { headers: bearer(t) })).status).toBe(403);
  });

  it("sair encerra a sessão no servidor", async () => {
    const { app } = setup();
    const { phone: t } = await phone(app);
    expect((await app.request("/api/sessions/current", { method: "DELETE", headers: bearer(t) })).status).toBe(204);
    expect((await app.request("/api/accounts/me/public-key", { headers: bearer(t) })).status).toBe(401);
  });
});

describe("cota por conta (I8)", () => {
  it("passou do número máximo de ideias guardadas: 507", async () => {
    const db = openDb(":memory:");
    const app = createApp(db, { inviteCode: INVITE, quota: { count: 2, bytes: 1000 } });
    const { token } = await register(app as any);
    const post = () => app.request("/api/inbox", { method: "POST", headers: bearer(token, { "x-lume-kind": "text", "content-type": "text/plain" }), body: new Uint8Array(10) });
    expect((await post()).status).toBe(201);
    expect((await post()).status).toBe(201);
    expect((await post()).status).toBe(507);
  });

  it("passou do total de bytes: 507", async () => {
    const app = createApp(openDb(":memory:"), { inviteCode: INVITE, quota: { count: 100, bytes: 15 } });
    const { token } = await register(app as any);
    const post = () => app.request("/api/inbox", { method: "POST", headers: bearer(token, { "x-lume-kind": "text", "content-type": "text/plain" }), body: new Uint8Array(10) });
    expect((await post()).status).toBe(201);
    expect((await post()).status).toBe(507);
  });
});

describe("rotas da API inexistentes (M8)", () => {
  it("devolvem 404 em JSON", async () => {
    const { app } = setup();
    const res = await app.request("/api/nao-existe");
    expect(res.status).toBe(404);
    expect(await res.json()).toEqual({ error: "Não encontrado." });
  });
});
