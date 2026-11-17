/** Conta palavras: sequências de letras/números (hífen e apóstrofo internos fazem parte). */
export function countWords(text: string): number {
  return text.match(/[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu)?.length ?? 0;
}
