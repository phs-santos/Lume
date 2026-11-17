import { describe, expect, it } from "vitest";
import { register, setup } from "./helpers.js";

const post = (token: string, body: Uint8Array, kind = "audio", mime = "audio/webm;codecs=opus"): RequestInit => ({
  method: "POST",
  headers: { authorization: `Bearer ${token}`, "x-lume-kind": kind, "content-type": mime },
  body,
});
const auth = (token: string, extra: RequestInit = {}): RequestInit => ({ ...extra, headers: { authorization: `Bearer ${token}` } });

describe("caixa de entrada", () => {
  it("posta, lista e baixa os mesmos bytes com o mime original", async () => {
    const { app } = setup();
    const { token } = await register(app);
    const bytes = new Uint8Array([1, 2, 3, 250]);
    const res = await app.request("/api/inbox", post(token, bytes, "audio", "audio/mp4"));
    expect(res.status).toBe(201);
    const { id } = (await res.json()) as { id: string };
    const list = (await (await app.request("/api/inbox", auth(token))).json()) as any[];
    expect(list).toEqual([{ id, kind: "audio", mime: "audio/mp4", size: 4, created_at: expect.any(Number) }]);
    const got = await app.request(`/api/inbox/${id}`, auth(token));
    expect(got.headers.get("x-lume-kind")).toBe("audio");
    expect(got.headers.get("x-lume-mime")).toBe("audio/mp4");
    expect(new Uint8Array(await got.arrayBuffer())).toEqual(bytes);
  });

  it("lista do mais antigo para o mais novo", async () => {
    const { app, now } = setup();
    const { token } = await register(app);
    const a = (await (await app.request("/api/inbox", post(token, new Uint8Array([1]), "text", "text/plain"))).json()) as any;
    now.t += 1000;
    const b = (await (await app.request("/api/inbox", post(token, new Uint8Array([2]), "text", "text/plain"))).json()) as any;
    const list = (await (await app.request("/api/inbox", auth(token))).json()) as any[];
    expect(list.map((x) => x.id)).toEqual([a.id, b.id]);
  });

  it("apagar remove da lista", async () => {
    const { app } = setup();
    const { token } = await register(app);
    const { id } = (await (await app.request("/api/inbox", post(token, new Uint8Array([1])))).json()) as any;
    expect((await app.request(`/api/inbox/${id}`, auth(token, { method: "DELETE" }))).status).toBe(204);
    expect(await (await app.request("/api/inbox", auth(token))).json()).toEqual([]);
  });

  it("uma conta não vê, não baixa e não apaga envelope de outra", async () => {
    const { app } = setup();
    const a = await register(app, "ana");
    const b = await register(app, "bia");
    const { id } = (await (await app.request("/api/inbox", post(a.token, new Uint8Array([9])))).json()) as any;
    expect(await (await app.request("/api/inbox", auth(b.token))).json()).toEqual([]);
    expect((await app.request(`/api/inbox/${id}`, auth(b.token))).status).toBe(404);
    expect((await app.request(`/api/inbox/${id}`, auth(b.token, { method: "DELETE" }))).status).toBe(404);
    expect((await app.request(`/api/inbox/${id}`, auth(a.token))).status).toBe(200);
  });

  it("acima de 25 MB é 413", async () => {
    const { app } = setup();
    const { token } = await register(app);
    const res = await app.request("/api/inbox", post(token, new Uint8Array(25 * 1024 * 1024 + 1)));
    expect(res.status).toBe(413);
  });

  it("sem token é 401; tipo inválido é 400; corpo vazio é 400", async () => {
    const { app } = setup();
    const { token } = await register(app);
    expect((await app.request("/api/inbox", { method: "POST", body: new Uint8Array([1]) })).status).toBe(401);
    expect((await app.request("/api/inbox", post(token, new Uint8Array([1]), "video"))).status).toBe(400);
    expect((await app.request("/api/inbox", post(token, new Uint8Array([])))).status).toBe(400);
  });

  it("health responde ok", async () => {
    const { app } = setup();
    expect(await (await app.request("/api/health")).json()).toEqual({ ok: true });
  });
});
