import sodium from "libsodium-wrappers-sumo";

/** Mesmos parâmetros do desktop: Argon2id, 64 MiB, 3 passadas, 32 bytes. */
const OPSLIMIT = 3;
const MEMLIMIT = 64 * 1024 * 1024;

export async function deriveLoginSecret(password: string, salt: Uint8Array): Promise<Uint8Array> {
  await sodium.ready;
  return sodium.crypto_pwhash(32, password, salt, OPSLIMIT, MEMLIMIT, sodium.crypto_pwhash_ALG_ARGON2ID13);
}

/** Envelope lacrado para a chave pública da conta: só a chave privada (no computador) abre. */
export async function seal(message: Uint8Array, publicKey: Uint8Array): Promise<Uint8Array> {
  await sodium.ready;
  return sodium.crypto_box_seal(message, publicKey);
}

export const b64 = {
  encode(b: Uint8Array): string {
    let s = "";
    for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode(...b.subarray(i, i + 0x8000));
    return btoa(s);
  },
  decode(s: string): Uint8Array {
    return Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
  },
};
