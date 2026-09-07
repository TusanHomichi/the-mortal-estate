import { describe, expect, it } from "vitest";
import { INTERIOR_AMBIENT_INTENSITY, paletteFor } from "../src/space/palette";

describe("space-local preset palette", () => {
  it("ignores exterior presets in an interior and provides readable warm indoor light", () => {
    const interior = paletteFor(["dusk", "rain", "wind"], false);
    const plainInterior = paletteFor(["night"], false);
    const nightExterior = paletteFor(["night"], true);

    expect(interior.background.getHex()).toBe(plainInterior.background.getHex());
    expect(interior.key.getHex()).toBe(plainInterior.key.getHex());
    expect(interior.ambientIntensity).toBe(INTERIOR_AMBIENT_INTENSITY);
    expect(interior.ambientIntensity).toBeGreaterThan(nightExterior.ambientIntensity);
    expect(interior.lanternIntensity).toBe(0);
  });
});
