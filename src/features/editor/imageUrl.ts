import { imageGet, imageMime } from "../../lib/api";

/** URLs de exibição das imagens do texto, uma por id (os bytes vêm cifrados do banco). */
const cache = new Map<string, Promise<string>>();

export function imageUrl(id: string): Promise<string> {
  let p = cache.get(id);
  if (!p) {
    p = Promise.all([imageGet(id), imageMime(id)]).then(([buf, mime]) => URL.createObjectURL(new Blob([buf], { type: mime })));
    p.catch(() => cache.delete(id));
    cache.set(id, p);
  }
  return p;
}
