import { b64, deriveLoginSecret } from "./crypto";
import type { Pending } from "./queue";

/** Erro com o código HTTP, para a fila saber se é passageiro (rede, 5xx) ou definitivo (4xx). */
export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function check(res: Response): Promise<Response> {
  if (res.ok) return res;
  const msg = await res
    .json()
    .then((j: { error?: string }) => j.error)
    .catch(() => null);
  throw new ApiError(res.status, msg ?? `Erro ${res.status}`);
}

/** Chave pública atual da conta (a do computador). */
export async function fetchPublicKey(token: string): Promise<Uint8Array> {
  const pk = await (await check(await fetch("/api/accounts/me/public-key", { headers: { authorization: `Bearer ${token}` } }))).json();
  return b64.decode(pk.public_key);
}

/** Entra com apelido e senha (a mesma do cofre do computador). A senha nunca sai do aparelho. */
export async function login(nickname: string, password: string): Promise<{ token: string; publicKey: Uint8Array }> {
  const { salt } = await (await check(await fetch(`/api/accounts/${encodeURIComponent(nickname)}/salt`))).json();
  const secret = await deriveLoginSecret(password, b64.decode(salt));
  const res = await check(
    await fetch("/api/sessions", {
      method: "POST",
      headers: { "content-type": "application/json" },
      // sessão de celular: só envia ideias; não lê nem apaga a caixa de entrada
      body: JSON.stringify({ nickname, secret: b64.encode(secret), scope: "capture" }),
    }),
  );
  const { token } = await res.json();
  return { token, publicKey: await fetchPublicKey(token) };
}

/** Encerra a sessão no servidor (o token deixa de valer). */
export async function logout(token: string): Promise<void> {
  await fetch("/api/sessions/current", { method: "DELETE", headers: { authorization: `Bearer ${token}` } }).catch(() => {});
}

export async function postEnvelope(token: string, p: Pending): Promise<void> {
  await check(
    await fetch("/api/inbox", {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "x-lume-kind": p.kind, "content-type": p.mime },
      body: p.body as Uint8Array<ArrayBuffer>,
    }),
  );
}
