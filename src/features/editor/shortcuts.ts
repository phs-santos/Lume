/** Atalhos do editor. "mod" é Cmd no Mac e Ctrl no Windows. */
export type Action =
  | "save"
  | "newScene"
  | "newChapter"
  | "nextScene"
  | "prevScene"
  | "toggleSidebar"
  | "toggleFocus"
  | "toggleTheme"
  | "versions"
  | "ideas"
  | "assistant"
  | "export"
  | "settings"
  | "help"
  | "close";

export interface Shortcut {
  action: Action;
  key: string; // comparado sem diferenciar maiúsculas
  mod: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  group: "Escrever" | "Navegar" | "Aparência" | "Painéis";
}

export const SHORTCUTS: Shortcut[] = [
  { action: "save", key: "s", mod: true, description: "Salvar agora", group: "Escrever" },
  { action: "newScene", key: "n", mod: true, description: "Nova cena depois desta", group: "Escrever" },
  { action: "newChapter", key: "n", mod: true, shift: true, description: "Novo capítulo", group: "Escrever" },
  { action: "nextScene", key: "ArrowDown", mod: true, alt: true, description: "Próxima cena", group: "Navegar" },
  { action: "prevScene", key: "ArrowUp", mod: true, alt: true, description: "Cena anterior", group: "Navegar" },
  { action: "toggleSidebar", key: "\\", mod: true, description: "Mostrar ou esconder o sumário", group: "Navegar" },
  { action: "toggleFocus", key: "f", mod: true, shift: true, description: "Modo foco", group: "Aparência" },
  { action: "toggleTheme", key: "l", mod: true, shift: true, description: "Papel claro ou escuro", group: "Aparência" },
  { action: "assistant", key: "j", mod: true, description: "Assistente de IA", group: "Painéis" },
  { action: "ideas", key: "i", mod: true, shift: true, description: "Ideias do celular", group: "Painéis" },
  { action: "versions", key: "h", mod: true, shift: true, description: "Versões da cena", group: "Painéis" },
  { action: "export", key: "e", mod: true, shift: true, description: "Exportar o livro", group: "Painéis" },
  { action: "settings", key: ",", mod: true, description: "Ajustes", group: "Painéis" },
  { action: "help", key: "/", mod: true, description: "Ver os atalhos", group: "Painéis" },
  { action: "close", key: "Escape", mod: false, description: "Fechar o painel aberto", group: "Painéis" },
];

/** Atalhos do próprio editor de texto, só para exibir na lista. */
export const TEXT_SHORTCUTS = [
  { keys: ["i"], description: "Itálico" },
  { keys: ["b"], description: "Negrito" },
  { keys: ["z"], description: "Desfazer" },
];

type KeyEvent = { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; altKey: boolean };

export function matchShortcut(e: KeyEvent, isMac: boolean): Action | null {
  const mod = isMac ? e.metaKey : e.ctrlKey;
  const wrongMod = isMac ? e.ctrlKey : e.metaKey;
  if (wrongMod) return null;
  const hit = SHORTCUTS.find(
    (s) =>
      s.key.toLowerCase() === e.key.toLowerCase() &&
      s.mod === mod &&
      !!s.shift === e.shiftKey &&
      !!s.alt === e.altKey,
  );
  return hit?.action ?? null;
}

const KEY_NAMES: Record<string, [string, string]> = {
  ArrowDown: ["↓", "↓"],
  ArrowUp: ["↑", "↑"],
  Escape: ["Esc", "Esc"],
};

export function label(s: Pick<Shortcut, "key" | "mod" | "shift" | "alt">, isMac: boolean): string {
  const parts: string[] = [];
  if (s.mod) parts.push(isMac ? "⌘" : "Ctrl");
  if (s.alt) parts.push(isMac ? "⌥" : "Alt");
  if (s.shift) parts.push(isMac ? "⇧" : "Shift");
  const named = KEY_NAMES[s.key];
  parts.push(named ? named[isMac ? 0 : 1] : s.key.toUpperCase());
  return parts.join(" ");
}

export const isMac = () => typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);
