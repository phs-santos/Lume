// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { isMapDeleteKey } from "./keys";

describe("isMapDeleteKey", () => {
  const map = document.createElement("section");
  const node = document.createElement("div");
  const ta = document.createElement("textarea");
  map.append(node, ta);
  document.body.append(map);
  const ev = (key: string, target: EventTarget) => ({ key, target });

  it("Backspace ou Delete com o foco num cartão do mapa apagam", () => {
    expect(isMapDeleteKey(ev("Backspace", node), map)).toBe(true);
    expect(isMapDeleteKey(ev("Delete", node), map)).toBe(true);
  });
  it("foco fora do mapa (depois de terminar de digitar) não apaga", () => {
    expect(isMapDeleteKey(ev("Backspace", document.body), map)).toBe(false);
  });
  it("digitando num campo não apaga", () => expect(isMapDeleteKey(ev("Backspace", ta), map)).toBe(false));
  it("outras teclas não apagam", () => expect(isMapDeleteKey(ev("a", node), map)).toBe(false));
});
