// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { onClickOutside } from "./clickOutside";

const el = (html: string) => {
  const d = document.createElement("div");
  d.innerHTML = html;
  document.body.appendChild(d);
  return d;
};
const press = (t: Element) => t.dispatchEvent(new Event("pointerdown", { bubbles: true }));
afterEach(() => (document.body.innerHTML = ""));

describe("onClickOutside", () => {
  it("clique fora fecha", () => {
    const root = el('<aside id="p"><button id="dentro"></button></aside><p id="fora"></p>');
    const close = vi.fn();
    const off = onClickOutside(() => true, () => [root.querySelector("#p")], close);
    press(root.querySelector("#fora")!);
    expect(close).toHaveBeenCalledTimes(1);
    off();
  });

  it("clique dentro não fecha", () => {
    const root = el('<aside id="p"><button id="dentro"></button></aside>');
    const close = vi.fn();
    const off = onClickOutside(() => true, () => [root.querySelector("#p")], close);
    press(root.querySelector("#dentro")!);
    expect(close).not.toHaveBeenCalled();
    off();
  });

  it("clique no botão que abre (área ignorada) não fecha, para o próprio botão alternar", () => {
    const root = el('<nav id="barra"><button id="abre"></button></nav><aside id="p"></aside>');
    const close = vi.fn();
    const off = onClickOutside(() => true, () => [root.querySelector("#p"), root.querySelector("#barra")], close);
    press(root.querySelector("#abre")!);
    expect(close).not.toHaveBeenCalled();
    off();
  });

  it("fechado não faz nada", () => {
    const root = el('<p id="fora"></p>');
    const close = vi.fn();
    const off = onClickOutside(() => false, () => [], close);
    press(root.querySelector("#fora")!);
    expect(close).not.toHaveBeenCalled();
    off();
  });

  it("Esc fecha", () => {
    const close = vi.fn();
    const off = onClickOutside(() => true, () => [], close);
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(close).toHaveBeenCalledTimes(1);
    off();
  });

  it("depois de desligar, não reage mais", () => {
    const root = el('<p id="fora"></p>');
    const close = vi.fn();
    onClickOutside(() => true, () => [], close)();
    press(root.querySelector("#fora")!);
    expect(close).not.toHaveBeenCalled();
  });
});
