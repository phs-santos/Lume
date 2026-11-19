import { describe, expect, it } from "vitest";
import { errorMessage } from "./errors";

describe("errorMessage", () => {
  it("INVALID do Rust vira a frase, com maiúscula", () => {
    expect(errorMessage("INVALID: proposta já resolvida")).toBe("Proposta já resolvida.");
  });
  it("códigos conhecidos continuam", () => expect(errorMessage("LOCKED")).toBe("O cofre está trancado. Abra com sua senha."));
});
