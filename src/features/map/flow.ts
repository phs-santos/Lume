/** Converte o mapa guardado no cofre para o formato do Vue Flow. */
import { MarkerType, type Edge, type Node } from "@vue-flow/core";
import type { MapCard, MapLink } from "../../lib/types";

/** Tamanho de referência de um cartão (para enquadrar). */
export const CARD_W = 240;
export const CARD_H = 120;

export interface CardData { card: MapCard }

export const toNodes = (cards: MapCard[]): Node<CardData>[] =>
  cards.map((card) => ({ id: card.id, type: "card", position: { x: card.x, y: card.y }, data: { card } }));

export const toEdges = (links: MapLink[]): Edge[] =>
  links.map((l) => ({ id: l.id, source: l.from_card, target: l.to_card, label: l.label, markerEnd: MarkerType.ArrowClosed }));

/** Retângulo que cobre os cartões indicados (para mostrar os que acabaram de entrar). */
export function newCardsBounds(cards: MapCard[], ids: string[]) {
  const sel = cards.filter((c) => ids.includes(c.id));
  if (!sel.length) return null;
  const x = Math.min(...sel.map((c) => c.x)), y = Math.min(...sel.map((c) => c.y));
  const x2 = Math.max(...sel.map((c) => c.x)) + CARD_W, y2 = Math.max(...sel.map((c) => c.y)) + CARD_H;
  return { x, y, width: x2 - x, height: y2 - y };
}
