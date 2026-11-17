/** Áudio recebido do celular como Blob com o tipo original (mp4 do iPhone, webm do Android). */
export function audioBlob(body: Uint8Array | number[], mime: string): Blob {
  const bytes = body instanceof Uint8Array ? body : Uint8Array.from(body);
  return new Blob([bytes as Uint8Array<ArrayBuffer>], { type: mime });
}
