// @vitest-environment happy-dom
import { describe, expect, it, vi } from "vitest";
import { audioBlob } from "./audioUrl";

describe("audioBlob", () => {
  it.each(["audio/mp4", "audio/webm;codecs=opus"])("mantém o tipo %s para o player saber tocar", async (mime) => {
    const b = audioBlob(new Uint8Array([1, 2, 3]), mime);
    expect(b.type).toBe(mime);
    expect(new Uint8Array(await b.arrayBuffer())).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("aceita ArrayBuffer (bytes crus vindos do Rust)", async () => {
    const b = audioBlob(new Uint8Array([7, 8]).buffer, "audio/webm");
    expect(new Uint8Array(await b.arrayBuffer())).toEqual(new Uint8Array([7, 8]));
  });

  it("aceita o corpo como lista de números", async () => {
    const b = audioBlob([4, 5], "audio/mp4");
    expect(new Uint8Array(await b.arrayBuffer())).toEqual(new Uint8Array([4, 5]));
  });
});
vi.restoreAllMocks();
