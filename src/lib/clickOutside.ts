/**
 * Fecha algo aberto (painel, menu) com um clique fora dele ou com Esc.
 * `inside` lista as áreas onde o clique NÃO fecha: o próprio painel e, em geral,
 * o botão que o abre (para o botão continuar alternando sozinho).
 * Devolve a função que desliga.
 */
export function onClickOutside(
  isOpen: () => boolean,
  inside: () => (Element | null | undefined)[],
  close: () => void,
): () => void {
  const onPointer = (e: Event) => {
    if (!isOpen()) return;
    const t = e.target as Node | null;
    if (t && inside().some((el) => el?.contains(t))) return;
    close();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && isOpen()) close();
  };
  document.addEventListener("pointerdown", onPointer, true);
  document.addEventListener("keydown", onKey);
  return () => {
    document.removeEventListener("pointerdown", onPointer, true);
    document.removeEventListener("keydown", onKey);
  };
}
