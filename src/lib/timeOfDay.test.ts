import { describe, expect, it } from "vitest";
import { timeOfDay } from "./timeOfDay";

describe("timeOfDay", () => {
  it("5h às 11h é manhã", () => {
    expect(timeOfDay(5)).toBe("manha");
    expect(timeOfDay(11)).toBe("manha");
  });
  it("12h às 17h é tarde", () => {
    expect(timeOfDay(12)).toBe("tarde");
    expect(timeOfDay(17)).toBe("tarde");
  });
  it("18h às 4h é noite", () => {
    expect(timeOfDay(18)).toBe("noite");
    expect(timeOfDay(23)).toBe("noite");
    expect(timeOfDay(0)).toBe("noite");
    expect(timeOfDay(4)).toBe("noite");
  });
});
