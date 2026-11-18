import { describe, expect, it } from "vitest";
import { fitWithin, prepareImage } from "./imageFile";

describe("fitWithin", () => {
  it("reduz mantendo a proporção", () => expect(fitWithin(3000, 2000)).toEqual({ width: 1600, height: 1067 }));
  it("imagem em pé usa a altura como lado maior", () => expect(fitWithin(1000, 4000)).toEqual({ width: 400, height: 1600 }));
  it("imagem pequena fica como está", () => expect(fitWithin(800, 600)).toEqual({ width: 800, height: 600 }));
});

describe("prepareImage", () => {
  it("recusa arquivo maior que 10 MB", async () => {
    const big = new Blob([new Uint8Array(10 * 1024 * 1024 + 1)], { type: "image/jpeg" });
    await expect(prepareImage(big)).rejects.toThrow("Imagem grande demais (máx. 10 MB).");
  });
  it("recusa o que não é imagem", async () => {
    await expect(prepareImage(new Blob(["oi"], { type: "text/plain" }))).rejects.toThrow("Esse arquivo não é uma imagem.");
  });
});
