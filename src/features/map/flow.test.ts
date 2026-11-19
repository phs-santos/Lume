import { describe, expect, it } from "vitest";
import { MarkerType } from "@vue-flow/core";
import type { MapCard, MapLink } from "../../lib/types";
import { newCardsBounds, toEdges, toNodes } from "./flow";

const card = (id: string, x: number, y: number): MapCard => ({
  id, book_id: "b", text: id, color: "papel", x, y, bible_entry_id: null, created_at: 0, updated_at: 0,
});

describe("flow", () => {
  it("toNodes keeps position and card data", () => {
    const [n] = toNodes([card("a", 10, 20)]);
    expect(n).toMatchObject({ id: "a", type: "card", position: { x: 10, y: 20 } });
    expect(n.data?.card.id).toBe("a");
  });

  it("toEdges maps label and arrow marker", () => {
    const l: MapLink = { id: "l", book_id: "b", from_card: "a", to_card: "z", label: "por quê?" };
    const [e] = toEdges([l]);
    expect(e).toMatchObject({ id: "l", source: "a", target: "z", label: "por quê?" });
    expect(e.markerEnd).toBe(MarkerType.ArrowClosed);
  });

  it("newCardsBounds covers only given ids", () => {
    const cards = [card("a", 0, 0), card("b", 400, 100), card("c", 700, 300)];
    expect(newCardsBounds(cards, ["b", "c"])).toEqual({ x: 400, y: 100, width: 300 + 240, height: 200 + 120 });
    expect(newCardsBounds(cards, [])).toBeNull();
  });
});
