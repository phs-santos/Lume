import type { ErrorCode } from "./types";

const messages: Record<ErrorCode, string> = {
  WRONG_PASSWORD: "Senha incorreta.",
  INVALID_PHRASE: "Essas palavras não abrem este cofre.",
  NO_VAULT: "Ainda não existe um cofre neste computador.",
  VAULT_EXISTS: "Já existe um cofre neste computador.",
  LOCKED: "O cofre está trancado. Abra com sua senha.",
  CHAPTER_NOT_EMPTY: "Mova as cenas antes de apagar o capítulo.",
};

/** Converte o erro vindo do Rust em uma frase para a pessoa. */
export function errorMessage(e: unknown): string {
  const s = String(e);
  return (messages as Record<string, string>)[s] ?? `Algo deu errado: ${s}`;
}
