import type { Outline, SceneSummary } from "./types";

export type DropTarget = { toChapterId: string; toIndex: number };

/**
 * Converte o resultado de um arrastar (lista de destino + índice final) num movimento.
 * `null` quando nada muda ou a cena não existe.
 */
export function resolveDrop(outline: Outline, sceneId: string, toChapterId: string, newIndex: number): DropTarget | null {
  const from = outline.chapters.find((c) => c.scenes.some((s) => s.id === sceneId));
  const to = outline.chapters.find((c) => c.chapter.id === toChapterId);
  if (!from || !to) return null;
  const others = to.scenes.filter((s) => s.id !== sceneId).length;
  const toIndex = Math.max(0, Math.min(newIndex, others));
  const sameChapter = from.chapter.id === toChapterId;
  if (sameChapter && from.scenes.findIndex((s) => s.id === sceneId) === toIndex) return null;
  return { toChapterId, toIndex };
}

/** A única cena de um capítulo, quando ela tem o mesmo nome dele: aí o sumário mostra uma linha só. */
export function singleScene(c: Outline["chapters"][number]): SceneSummary | null {
  if (c.scenes.length !== 1) return null;
  const [s] = c.scenes;
  return s.title.trim() === c.chapter.title.trim() ? s : null;
}
