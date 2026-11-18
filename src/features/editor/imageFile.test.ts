import { describe, expect, it } from "vitest";
import { fitWithin, pastedImages, prepareImage } from "./imageFile";

describe("pastedImages", () => {
  const png = new File([new Uint8Array([1])], "a.png", { type: "image/png" });
  it("imagem sozinha entra como imagem", () => expect(pastedImages([png], ["Files"])).toEqual([png]));
  it("colar do Word (texto + imagem do trecho) fica como texto", () => {
    expect(pastedImages([png], ["text/plain", "text/html", "Files"])).toEqual([]);
  });
  it("ignora arquivos que não são imagem", () => {
    expect(pastedImages([new File(["x"], "a.txt", { type: "text/plain" })], ["Files"])).toEqual([]);
  });
});

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
