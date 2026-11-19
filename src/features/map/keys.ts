/**
 * Backspace/Delete só apaga o que está selecionado no mapa quando o foco está num cartão ou seta
 * do próprio mapa. Depois de terminar de digitar num cartão o foco sai do mapa, e um Backspace
 * de reflexo não apaga nada.
 */
export function isMapDeleteKey(e: { key: string; target: EventTarget | null }, map: HTMLElement | null): boolean {
  if (e.key !== "Backspace" && e.key !== "Delete") return false;
  const t = e.target as HTMLElement | null;
  if (!map || !t || !map.contains(t)) return false;
  return !t.closest("input, textarea, select, [contenteditable=''], [contenteditable='true']");
}
