// Gera test-fixtures/crypto.json com libsodium (a mesma biblioteca do celular).
// O teste em Rust usa este arquivo para provar que o desktop abre o que o celular fecha.
import sodium from "libsodium-wrappers-sumo";

await sodium.ready;
const b64 = (b) => Buffer.from(b).toString("base64");
const password = "lume-teste-ção";
const salt = new Uint8Array(16).map((_, i) => i * 7 + 3);
const login_secret = sodium.crypto_pwhash(32, password, salt, 3, 64 * 1024 * 1024, sodium.crypto_pwhash_ALG_ARGON2ID13);
const kp = sodium.crypto_box_seed_keypair(new Uint8Array(32).fill(42));
const plaintext = new TextEncoder().encode("Ideia: Marina abre a carta na serra.");
const sealed = sodium.crypto_box_seal(plaintext, kp.publicKey);
console.log(JSON.stringify({
  password, salt: b64(salt), login_secret: b64(login_secret),
  secret_key: b64(kp.privateKey), public_key: b64(kp.publicKey),
  plaintext: b64(plaintext), sealed: b64(sealed),
}, null, 2));
