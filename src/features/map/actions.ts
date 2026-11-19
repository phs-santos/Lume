import type { InjectionKey } from "vue";
import type { BibleEntry, CardColor } from "../../lib/types";

/** O que um cartão pede ao mapa (o mapa fala com o cofre). */
export interface MapActions {
  save(id: string, text: string, color: CardColor): Promise<void>;
  remove(id: string): Promise<void>;
  toBible(id: string, kind: BibleEntry["kind"]): Promise<void>;
  kindOf(entryId: string): string | null;
  /** O cartão recém-criado já abre em edição. */
  consumeEditRequest(id: string): boolean;
}

export const MAP_ACTIONS: InjectionKey<MapActions> = Symbol("map-actions");
