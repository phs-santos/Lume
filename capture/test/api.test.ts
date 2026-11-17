import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, fetchPublicKey, login, logout, postEnvelope } from "../src/api";
import type { Pending } from "../src/queue";

const reply = (status: number, body: unknown) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

describe("api do celular", () => {
  afterEach(() => vi.unstubAllGlobals());

  it.each(["audio/mp4", "audio/webm;codecs=opus"])("postEnvelope manda kind e mime %s", async (mime) => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ id: "1" }), { status: 201 }));
    vi.stubGlobal("fetch", fetchMock);
    const p: Pending = { id: "x", kind: "audio", mime, body: new Uint8Array([1, 2]), createdAt: 1, attempts: 0, nickname: "henri" };
    await postEnvelope("tok", p);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/inbox");
    expect(init.headers["x-lume-kind"]).toBe("audio");
    expect(init.headers["content-type"]).toBe(mime);
    expect(init.headers.authorization).toBe("Bearer tok");
  });

  it("postEnvelope com erro do servidor lança", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(reply(413, { error: "A ideia passou de 25 MB." })));
    const p: Pending = { id: "x", kind: "text", mime: "text/plain", body: new Uint8Array([1]), createdAt: 1, attempts: 0, nickname: "henri" };
    const err = await postEnvelope("tok", p).catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(413);
    expect(err.message).toBe("A ideia passou de 25 MB.");
  });

  it("login com 401 lança a mensagem do servidor", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValueOnce(reply(200, { salt: "AwoRGB8mLTQ7QklQV15lbA==" })).mockResolvedValueOnce(reply(401, { error: "Apelido ou senha incorretos" })),
    );
    await expect(login("henri", "errada")).rejects.toThrow("Apelido ou senha incorretos");
  });

  it("login sem computador conectado avisa", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(reply(200, { salt: "AwoRGB8mLTQ7QklQV15lbA==" }))
        .mockResolvedValueOnce(reply(200, { token: "t" }))
        .mockResolvedValueOnce(reply(404, { error: "Este computador ainda não foi conectado." })),
    );
    await expect(login("henri", "certa")).rejects.toThrow("Este computador ainda não foi conectado.");
  });

  it("login pede sessão de celular (escopo capture)", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(reply(200, { salt: "AwoRGB8mLTQ7QklQV15lbA==" }))
      .mockResolvedValueOnce(reply(200, { token: "t" }))
      .mockResolvedValueOnce(reply(200, { public_key: "AAAA" }));
    vi.stubGlobal("fetch", fetchMock);
    await login("henri", "certa");
    expect(JSON.parse(fetchMock.mock.calls[1][1].body).scope).toBe("capture");
  });

  it("fetchPublicKey devolve a chave atual do servidor", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(reply(200, { public_key: "AQID" })));
    expect(await fetchPublicKey("t")).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("logout encerra a sessão no servidor", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);
    await logout("t");
    expect(fetchMock.mock.calls[0][0]).toBe("/api/sessions/current");
    expect(fetchMock.mock.calls[0][1].method).toBe("DELETE");
  });
});
