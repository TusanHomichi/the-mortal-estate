import { describe, expect, it } from "vitest";
import { parsePresets, presetsFromUrl, windPresetSettings } from "../src/presets";

describe("URL feel presets", () => {
  it("defaults to night", () => {
    expect(parsePresets(null)).toEqual(["night"]);
  });

  it("normalises, deduplicates, sorts, and refuses unknown presets", () => {
    expect(parsePresets(" WIND,night,rain,wind,legacy ")).toEqual(["night", "rain", "wind"]);
  });

  it("reads only the preset query value", () => {
    expect(presetsFromUrl(new URL("https://example.invalid/?preset=dusk,fog&other=rain"))).toEqual([
      "dusk",
      "fog",
    ]);
  });

  it("keeps a faint idle wind and strengthens wind and rain presets", () => {
    const calm = windPresetSettings(["night"], true);
    const rain = windPresetSettings(["rain"], true);
    const wind = windPresetSettings(["wind"], true);
    expect(calm.strength).toBeGreaterThan(0);
    expect(rain.strength).toBeGreaterThan(calm.strength);
    expect(wind.strength).toBeGreaterThan(rain.strength);
    expect(wind.gustPeriod).toBe(9);
    expect(windPresetSettings(["wind", "rain"], false).strength).toBe(calm.strength);
  });
});
