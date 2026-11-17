import { afterEach, describe, expect, it, vi } from "vitest";
import { login, postEnvelope } from "../src/api";
import type { Pending } from "../src/queue";

const reply = (status: number, body: unknown) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

describe("api do celular", () => {
  afterEach(() => vi.unstubAllGlobals());

  it.each(["audio/mp4", "audio/webm;codecs=opus"])("postEnvelope manda kind e mime %s", async (mime) => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ id: "1" }), { status: 201 }));
    vi.stubGlobal("fetch", fetchMock);
    const p: Pending = { id: "x", kind: "audio", mime, body: new Uint8Array([1, 2]), createdAt: 1, attempts: 0 };
    await postEnvelope("tok", p);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/inbox");
    expect(init.headers["x-lume-kind"]).toBe("audio");
    expect(init.headers["content-type"]).toBe(mime);
    expect(init.headers.authorization).toBe("Bearer tok");
  });

  it("postEnvelope com erro do servidor lança", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(reply(413, { error: "A ideia passou de 25 MB." })));
    const p: Pending = { id: "x", kind: "text", mime: "text/plain", body: new Uint8Array([1]), createdAt: 1, attempts: 0 };
    await expect(postEnvelope("tok", p)).rejects.toThrow("A ideia passou de 25 MB.");
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
});
