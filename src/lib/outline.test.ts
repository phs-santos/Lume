import { describe, expect, it } from "vitest";
import { resolveDrop } from "./outline";
import type { Outline } from "./types";

const scene = (id: string, chapter_id: string, position: number) => ({ id, chapter_id, title: id, word_count: 0, position });
const outline: Outline = {
  book: { id: "b", title: "L", ai_mode: "conselheira", created_at: 0 },
  chapters: [
    { chapter: { id: "c1", book_id: "b", title: "1", position: 0 }, scenes: [scene("A", "c1", 0), scene("B", "c1", 1), scene("C", "c1", 2)] },
    { chapter: { id: "c2", book_id: "b", title: "2", position: 1 }, scenes: [scene("D", "c2", 0)] },
    { chapter: { id: "c3", book_id: "b", title: "3", position: 2 }, scenes: [] },
  ],
};

describe("resolveDrop", () => {
  it("mesmo capítulo, índice novo", () => {
    expect(resolveDrop(outline, "C", "c1", 0)).toEqual({ toChapterId: "c1", toIndex: 0 });
  });
  it("outro capítulo", () => {
    expect(resolveDrop(outline, "B", "c2", 1)).toEqual({ toChapterId: "c2", toIndex: 1 });
  });
  it("capítulo vazio, índice 0", () => {
    expect(resolveDrop(outline, "A", "c3", 0)).toEqual({ toChapterId: "c3", toIndex: 0 });
  });
  it("mesma posição de antes não move", () => {
    expect(resolveDrop(outline, "B", "c1", 1)).toBeNull();
  });
  it("índice além do fim vai para o fim", () => {
    expect(resolveDrop(outline, "A", "c2", 99)).toEqual({ toChapterId: "c2", toIndex: 1 });
    expect(resolveDrop(outline, "A", "c1", 99)).toEqual({ toChapterId: "c1", toIndex: 2 });
  });
  it("cena desconhecida não move", () => {
    expect(resolveDrop(outline, "Z", "c1", 0)).toBeNull();
  });
});
