import { clear, createStore } from "idb-keyval";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiError } from "../src/api";
import { AuthExpired, enqueue, failed, flush, pending } from "../src/queue";

const item = (n: number, nickname = "henri") => ({ kind: "text" as const, mime: "text/plain", body: new Uint8Array([n]), createdAt: n, nickname });

describe("fila offline", () => {
  beforeEach(async () => {
    await clear(createStore("lume-fila", "ideias"));
    await clear(createStore("lume-falhas", "ideias"));
  });

  it("enqueue e pending devolvem o item", async () => {
    await enqueue(item(1));
    const p = await pending();
    expect(p).toHaveLength(1);
    expect(p[0].body).toEqual(new Uint8Array([1]));
    expect(p[0].attempts).toBe(0);
  });

  it("flush com envio OK esvazia a fila", async () => {
    await enqueue(item(1));
    await enqueue(item(2));
    const send = vi.fn().mockResolvedValue(undefined);
    expect(await flush("henri", send)).toEqual({ sent: 2, left: 0 });
    expect(await pending()).toEqual([]);
  });

  it("envio falhando mantém o item e conta a tentativa; depois envia", async () => {
    await enqueue(item(1));
    expect(await flush("henri", vi.fn().mockRejectedValue(new Error("sem rede")))).toEqual({ sent: 0, left: 1 });
    expect((await pending())[0].attempts).toBe(1);
    expect(await flush("henri", vi.fn().mockResolvedValue(undefined))).toEqual({ sent: 1, left: 0 });
  });

  it("envia na ordem em que foi gravado", async () => {
    await enqueue(item(3));
    await enqueue(item(1));
    await enqueue(item(2));
    const order: number[] = [];
    await flush("henri", async (p) => void order.push(p.body[0]));
    expect(order).toEqual([1, 2, 3]);
  });

  it("dois flush ao mesmo tempo não enviam o mesmo item duas vezes", async () => {
    await enqueue(item(1));
    const send = vi.fn().mockImplementation(() => new Promise((r) => setTimeout(r, 10)));
    await Promise.all([flush("henri", send), flush("henri", send)]);
    expect(send).toHaveBeenCalledTimes(1);
  });

  it("sessão expirada (401) para a fila, mantém o item e avisa", async () => {
    await enqueue(item(1));
    await expect(flush("henri", vi.fn().mockRejectedValue(new ApiError(401, "Entre de novo.")))).rejects.toBeInstanceOf(AuthExpired);
    expect(await pending()).toHaveLength(1);
  });

  it("recusa definitiva (413) tira o item da frente e segue com os outros", async () => {
    await enqueue(item(1));
    await enqueue(item(2));
    const send = vi.fn().mockRejectedValueOnce(new ApiError(413, "A ideia passou de 25 MB.")).mockResolvedValue(undefined);
    expect(await flush("henri", send)).toEqual({ sent: 1, left: 0 });
    const f = await failed();
    expect(f).toHaveLength(1);
    expect(f[0].reason).toBe("A ideia passou de 25 MB.");
  });

  it("erro 5xx é tratado como passageiro (fica na fila)", async () => {
    await enqueue(item(1));
    expect(await flush("henri", vi.fn().mockRejectedValue(new ApiError(502, "Bad Gateway")))).toEqual({ sent: 0, left: 1 });
    expect(await failed()).toEqual([]);
  });

  it("só envia ideias de quem está logado", async () => {
    await enqueue(item(1, "ana"));
    await enqueue(item(2, "henri"));
    const send = vi.fn().mockResolvedValue(undefined);
    expect(await flush("henri", send)).toEqual({ sent: 1, left: 0 });
    expect(send).toHaveBeenCalledTimes(1);
    expect((await pending()).map((p) => p.nickname)).toEqual(["ana"]);
  });
});
