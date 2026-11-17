import { reactive } from "vue";
import { b64 } from "./crypto";

const KEY = "lume-sessao";

/** Token e chave pública ficam no aparelho; a senha nunca. */
export const session = reactive<{ nickname: string; token: string; publicKey: Uint8Array | null }>({
  nickname: "",
  token: "",
  publicKey: null,
});

export function loadSession() {
  try {
    const s = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (s) Object.assign(session, { nickname: s.nickname, token: s.token, publicKey: b64.decode(s.publicKey) });
  } catch {
    /* sem sessão */
  }
}

export function saveSession(nickname: string, token: string, publicKey: Uint8Array) {
  Object.assign(session, { nickname, token, publicKey });
  try {
    localStorage.setItem(KEY, JSON.stringify({ nickname, token, publicKey: b64.encode(publicKey) }));
  } catch {
    /* navegador sem armazenamento: vale só nesta aba */
  }
}

export function clearSession() {
  Object.assign(session, { nickname: "", token: "", publicKey: null });
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* nada */
  }
}
