import { b64, deriveLoginSecret } from "./crypto";
import type { Pending } from "./queue";

async function check(res: Response): Promise<Response> {
  if (res.ok) return res;
  const msg = await res
    .json()
    .then((j: { error?: string }) => j.error)
    .catch(() => null);
  throw new Error(msg ?? `Erro ${res.status}`);
}

/** Entra com apelido e senha (a mesma do cofre do computador). A senha nunca sai do aparelho. */
export async function login(nickname: string, password: string): Promise<{ token: string; publicKey: Uint8Array }> {
  const { salt } = await (await check(await fetch(`/api/accounts/${encodeURIComponent(nickname)}/salt`))).json();
  const secret = await deriveLoginSecret(password, b64.decode(salt));
  const res = await check(
    await fetch("/api/sessions", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ nickname, secret: b64.encode(secret) }),
    }),
  );
  const { token } = await res.json();
  const pk = await (await check(await fetch("/api/accounts/me/public-key", { headers: { authorization: `Bearer ${token}` } }))).json();
  return { token, publicKey: b64.decode(pk.public_key) };
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
