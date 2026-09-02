import { describe, expect, it } from "vitest";
import { hearthFireFragmentShader, hearthFlicker } from "../src/shaders";
import {
  HEARTH_LIGHT_DISTANCE,
  HEARTH_LIGHT_INTENSITY_MULTIPLIER,
} from "../src/space/SpaceScene";

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

  it("carries the first-pass room-reaching fire light constants", () => {
    expect(HEARTH_LIGHT_INTENSITY_MULTIPLIER).toBe(8);
    expect(HEARTH_LIGHT_DISTANCE).toBe(7.5);
  });
});
