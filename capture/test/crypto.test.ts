import sodium from "libsodium-wrappers-sumo";
import { describe, expect, it } from "vitest";
import fixture from "../../test-fixtures/crypto.json";
import { b64, deriveLoginSecret, seal } from "../src/crypto";

describe("crypto do celular", () => {
  it("deriveLoginSecret bate com a fixture (Argon2id m=64MiB t=3 p=1)", async () => {
    const out = await deriveLoginSecret(fixture.password, b64.decode(fixture.salt));
    expect(b64.encode(out)).toBe(fixture.login_secret);
  });

  it("seal produz envelope que a chave privada abre", async () => {
    await sodium.ready;
    const msg = new TextEncoder().encode("ideia secreta");
    const sealed = await seal(msg, b64.decode(fixture.public_key));
    const opened = sodium.crypto_box_seal_open(sealed, b64.decode(fixture.public_key), b64.decode(fixture.secret_key));
    expect(new TextDecoder().decode(opened)).toBe("ideia secreta");
  });

  it("b64 ida e volta", () => {
    const b = new Uint8Array([0, 255, 128, 7]);
    expect(b64.decode(b64.encode(b))).toEqual(b);
  });
});
