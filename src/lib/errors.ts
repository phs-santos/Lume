import type { ErrorCode } from "./types";

const messages: Record<ErrorCode, string> = {
  WRONG_PASSWORD: "Senha incorreta.",
  INVALID_PHRASE: "Essas palavras não abrem este cofre.",
  NO_VAULT: "Ainda não existe um cofre neste computador.",
  VAULT_EXISTS: "Já existe um cofre neste computador.",
  LOCKED: "O cofre está trancado. Abra com sua senha.",
  CHAPTER_NOT_EMPTY: "Mova as cenas antes de apagar o capítulo.",
  DB_EXISTS: "Já existe um livro cifrado neste computador. Não vou criar um cofre novo por cima dele.",
  WRONG_KEY: "Este cofre não abre o livro guardado neste computador. Os arquivos podem ter sido trocados.",
  NOT_FOUND: "Não encontrei isso. Talvez já tenha sido apagado.",
  NOT_CONNECTED: "Este computador ainda não está conectado ao celular.",
  NOT_TEXT: "Só ideias escritas viram cena por enquanto. Áudio vai ganhar transcrição em breve.",
};

/** Converte o erro vindo do Rust em uma frase para a pessoa. */
export function errorMessage(e: unknown): string {
  const s = String(e);
  if (s.startsWith("NETWORK")) return "Sem conexão com o servidor. Confira a internet e tente de novo.";
  if (s.startsWith("SYNC_PARTIAL")) return "Algumas ideias não chegaram. Vou tentar de novo daqui a pouco.";
  return (messages as Record<string, string>)[s] ?? (/^[A-ZÁ-Ú]/.test(s) && !s.includes("_") ? s : `Algo deu errado: ${s}`);
}
