/** Capa gerada pelo Lume: título e autor sobre um tecido de encadernação. Formato do Kindle: 1600 × 2560. */
export const COVER_W = 1600;
export const COVER_H = 2560;

/** Quebra o título em linhas de até `maxChars`, equilibradas e sem cortar palavras. */
export function wrapTitle(title: string, maxChars: number): string[] {
  const words = title.trim().split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const len = (i: number, j: number) => words.slice(i, j).join(" ").length; // palavras i..j-1
  const n = words.length;
  // menor número de linhas que cabe (uma palavra longa demais fica sozinha, inteira)
  const fits = (i: number, j: number) => j - i === 1 || len(i, j) <= maxChars;
  // best[k][i]: melhor divisão das palavras i.. em k linhas → [maior linha, soma dos quadrados, cortes]
  type Best = { max: number; sq: number; cuts: number[] } | null;
  for (let k = 1; k <= n; k++) {
    const memo = new Map<string, Best>();
    const solve = (i: number, lines: number): Best => {
      const key = `${i}:${lines}`;
      if (memo.has(key)) return memo.get(key)!;
      let best: Best = null;
      if (lines === 1) {
        best = fits(i, n) ? { max: len(i, n), sq: len(i, n) ** 2, cuts: [] } : null;
      } else {
        for (let j = i + 1; j <= n - (lines - 1); j++) {
          if (!fits(i, j)) break;
          const rest = solve(j, lines - 1);
          if (!rest) continue;
          const cand = { max: Math.max(len(i, j), rest.max), sq: len(i, j) ** 2 + rest.sq, cuts: [j, ...rest.cuts] };
          if (!best || cand.max < best.max || (cand.max === best.max && cand.sq < best.sq)) best = cand;
        }
      }
      memo.set(key, best);
      return best;
    };
    const r = solve(0, k);
    if (r) {
      const bounds = [0, ...r.cuts, n];
      return bounds.slice(0, -1).map((b, idx) => words.slice(b, bounds[idx + 1]).join(" "));
    }
  }
  return [words.join(" ")];
}

/** Desenha a capa e devolve o PNG. Usa as fontes do Lume (Literata e Courier Prime). */
export async function generateCover(o: { title: string; author: string; color: string }): Promise<Uint8Array> {
  await Promise.all([document.fonts.load("400 160px Literata"), document.fonts.load("400 64px 'Courier Prime'")]).catch(() => {});
  const c = document.createElement("canvas");
  c.width = COVER_W;
  c.height = COVER_H;
  const x = c.getContext("2d")!;

  // tecido: a cor do livro com um leve degradê e fios horizontais
  const bg = x.createLinearGradient(0, 0, COVER_W, COVER_H);
  bg.addColorStop(0, shade(o.color, 0.12));
  bg.addColorStop(1, shade(o.color, -0.18));
  x.fillStyle = bg;
  x.fillRect(0, 0, COVER_W, COVER_H);
  x.globalAlpha = 0.05;
  x.fillStyle = "#000";
  for (let y = 0; y < COVER_H; y += 6) x.fillRect(0, y, COVER_W, 2);
  x.globalAlpha = 1;

  // lombada
  const spine = x.createLinearGradient(0, 0, 140, 0);
  spine.addColorStop(0, "rgba(0,0,0,0.35)");
  spine.addColorStop(1, "rgba(0,0,0,0)");
  x.fillStyle = spine;
  x.fillRect(0, 0, 140, COVER_H);

  // moldura fina dupla
  x.strokeStyle = "rgba(243,241,234,0.55)";
  x.lineWidth = 4;
  x.strokeRect(150, 150, COVER_W - 300, COVER_H - 300);
  x.lineWidth = 2;
  x.strokeRect(180, 180, COVER_W - 360, COVER_H - 360);

  // título
  x.fillStyle = "#f3f1ea";
  x.textAlign = "center";
  x.textBaseline = "alphabetic";
  const lines = wrapTitle(o.title, 12);
  const size = lines.length > 3 ? 120 : 150;
  x.font = `400 ${size}px Literata, Georgia, serif`;
  const lineH = size * 1.18;
  const startY = 1000 - ((lines.length - 1) * lineH) / 2;
  lines.forEach((l, i) => x.fillText(l, COVER_W / 2, startY + i * lineH));

  // fita vermelha, a marca do Lume
  x.fillStyle = "#b42318";
  x.fillRect(COVER_W / 2 - 70, startY + (lines.length - 1) * lineH + 90, 140, 8);

  // autor
  if (o.author.trim()) {
    x.fillStyle = "rgba(243,241,234,0.85)";
    x.font = "400 64px 'Courier Prime', 'Courier New', monospace";
    x.fillText(o.author.trim(), COVER_W / 2, COVER_H - 330);
  }

  const blob = await new Promise<Blob | null>((r) => c.toBlob(r, "image/png"));
  if (!blob) throw new Error("Não consegui desenhar a capa.");
  return new Uint8Array(await blob.arrayBuffer());
}

/** Clareia (amount > 0) ou escurece (amount < 0) uma cor #rrggbb. */
function shade(hex: string, amount: number): string {
  const n = parseInt(hex.slice(1), 16);
  const ch = (v: number) => Math.max(0, Math.min(255, Math.round(amount > 0 ? v + (255 - v) * amount : v * (1 + amount))));
  const r = ch(n >> 16), g = ch((n >> 8) & 255), b = ch(n & 255);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}
