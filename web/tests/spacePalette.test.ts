import { describe, expect, it } from "vitest";
import { paletteFor } from "../src/space/palette";

describe("space-local preset palette", () => {
  it("ignores exterior presets in an interior and keeps it darker than night outside", () => {
    const interior = paletteFor(["dusk", "rain", "wind"], false);
    const plainInterior = paletteFor(["night"], false);
    const nightExterior = paletteFor(["night"], true);

    expect(interior.background.getHex()).toBe(plainInterior.background.getHex());
    expect(interior.key.getHex()).toBe(plainInterior.key.getHex());
    expect(interior.ambientIntensity).toBeLessThan(nightExterior.ambientIntensity);
    expect(interior.lanternIntensity).toBe(0);
  });
});
