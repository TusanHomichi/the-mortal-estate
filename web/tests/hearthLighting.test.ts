import { describe, expect, it } from "vitest";
import { hearthFireFragmentShader, hearthFlicker } from "../src/shaders";

describe("hearth light presentation", () => {
  it("renders the fire card as emissive colour without a scene-light term", () => {
    expect(hearthFireFragmentShader).toContain("emissiveColour * flicker");
    expect(hearthFireFragmentShader).not.toContain("ambientColour");
    expect(hearthFireFragmentShader).not.toContain("keyColour");
  });

  it("keeps the shared fire flicker inside a twelve-percent envelope", () => {
    const samples = Array.from({ length: 12_001 }, (_, index) => hearthFlicker(index / 120));

    expect(Math.min(...samples)).toBeGreaterThanOrEqual(0.88);
    expect(Math.min(...samples)).toBeLessThan(0.89);
    expect(Math.max(...samples)).toBeLessThanOrEqual(1.12);
    expect(Math.max(...samples)).toBeGreaterThan(1.11);
  });
});
