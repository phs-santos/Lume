import { clear, createStore } from "idb-keyval";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { enqueue, flush, pending } from "../src/queue";

const item = (n: number) => ({ kind: "text" as const, mime: "text/plain", body: new Uint8Array([n]), createdAt: n });

describe("fila offline", () => {
  beforeEach(async () => clear(createStore("lume-fila", "ideias")));

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
    expect(await flush(send)).toEqual({ sent: 2, left: 0 });
    expect(await pending()).toEqual([]);
  });

  it("envio falhando mantém o item e conta a tentativa; depois envia", async () => {
    await enqueue(item(1));
    expect(await flush(vi.fn().mockRejectedValue(new Error("sem rede")))).toEqual({ sent: 0, left: 1 });
    expect((await pending())[0].attempts).toBe(1);
    expect(await flush(vi.fn().mockResolvedValue(undefined))).toEqual({ sent: 1, left: 0 });
  });

  it("envia na ordem em que foi gravado", async () => {
    await enqueue(item(3));
    await enqueue(item(1));
    await enqueue(item(2));
    const order: number[] = [];
    await flush(async (p) => void order.push(p.body[0]));
    expect(order).toEqual([1, 2, 3]);
  });

  it("dois flush ao mesmo tempo não enviam o mesmo item duas vezes", async () => {
    await enqueue(item(1));
    const send = vi.fn().mockImplementation(() => new Promise((r) => setTimeout(r, 10)));
    await Promise.all([flush(send), flush(send)]);
    expect(send).toHaveBeenCalledTimes(1);
  });
});
