import { describe, expect, it } from "vitest";
import {
  describeView,
  parsePresets,
  parseZoomStep,
  presetsFromUrl,
  windPresetSettings,
  zoomStepFromUrl,
} from "../src/presets";

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

  it("reads a whole-number zoom step, clamps it, and treats anything else as the ruled frame", () => {
    expect(parseZoomStep(null)).toBe(0);
    expect(parseZoomStep("-1")).toBe(-1);
    expect(parseZoomStep("+2")).toBe(2);
    expect(parseZoomStep(" -9 ")).toBe(-3);
    expect(parseZoomStep("1.5")).toBe(0);
    expect(parseZoomStep("out")).toBe(0);
    expect(zoomStepFromUrl(new URL("https://example.invalid/?preset=night&zoom=-1"))).toBe(-1);
    expect(zoomStepFromUrl(new URL("https://example.invalid/?preset=night"))).toBe(0);
    expect(describeView("night", 0)).toBe("night");
    expect(describeView("night", -1)).toBe("night · zoom \u22121");
    expect(describeView("INTERIOR", 2)).toBe("INTERIOR · zoom +2");
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
