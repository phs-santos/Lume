/** Imagens que entram no texto: reduzidas antes de ir para o banco. */
export const MAX_SIDE = 1600;
export const MAX_BYTES = 10 * 1024 * 1024;
/** Capa e contracapa: maiores, para ficarem nítidas no EPUB. */
export const COVER_SIDE = 2560;
/** Acima disso a imagem é recusada (protege contra imagens "bomba" que explodem a memória). */
export const MAX_PIXELS = 100_000_000;

/**
 * Imagens de um colar. Word e Pages mandam uma imagem do trecho junto com o texto: nesse caso
 * vale o texto (devolve lista vazia).
 */
export function pastedImages(files: File[], types: readonly string[]): File[] {
  if (types.includes("text/plain") || types.includes("text/html")) return [];
  return files.filter((f) => f.type.startsWith("image/"));
}

/** Novo tamanho com o lado maior até `max`, mantendo a proporção. */
export function fitWithin(w: number, h: number, max = MAX_SIDE): { width: number; height: number } {
  const scale = Math.min(1, max / Math.max(w, h));
  return { width: Math.round(w * scale), height: Math.round(h * scale) };
}

/** Lê, reduz e recodifica: JPEG 0,85, ou PNG quando a imagem tem transparência. */
export async function prepareImage(file: Blob, maxSide = MAX_SIDE): Promise<{ mime: string; bytes: Uint8Array; width: number; height: number }> {
  if (!file.type.startsWith("image/")) throw new Error("Esse arquivo não é uma imagem.");
  if (file.size > MAX_BYTES) throw new Error("Imagem grande demais (máx. 10 MB).");
  let bmp: ImageBitmap;
  try {
    bmp = await createImageBitmap(file);
  } catch {
    throw new Error("Não consegui abrir essa imagem.");
  }
  if (bmp.width * bmp.height > MAX_PIXELS) {
    bmp.close();
    throw new Error("Imagem grande demais (máx. 100 megapixels).");
  }
  const { width, height } = fitWithin(bmp.width, bmp.height, maxSide);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(bmp, 0, 0, width, height);
  const alpha = file.type !== "image/jpeg" && hasTransparency(ctx, width, height);
  const mime = alpha ? "image/png" : "image/jpeg";
  const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, mime, 0.85));
  if (!blob) throw new Error("Não consegui abrir essa imagem.");
  return { mime, bytes: new Uint8Array(await blob.arrayBuffer()), width, height };
}

function hasTransparency(ctx: CanvasRenderingContext2D, w: number, h: number): boolean {
  const data = ctx.getImageData(0, 0, w, h).data;
  for (let i = 3; i < data.length; i += 4 * 16) if (data[i] < 250) return true; // amostra 1 a cada 16 pixels
  return false;
}

/** Bytes em base64 (para mandar ao Rust). */
export function toBase64(bytes: Uint8Array): string {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}
