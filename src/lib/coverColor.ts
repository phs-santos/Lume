/** Cores de capa: tons de tecido de encadernação, todos legíveis com texto claro. */
export const COVER_COLORS = ["#1c2333", "#b42318", "#2f5d50", "#5a3d5c", "#8a5a1f", "#3d4a5c", "#7a2e3b", "#24504f"];

/** Cor estável por livro (hash simples do id). */
export function coverColor(id: string): string {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.codePointAt(0)!) >>> 0;
  return COVER_COLORS[h % COVER_COLORS.length];
}
