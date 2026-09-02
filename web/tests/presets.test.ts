import { describe, expect, it } from "vitest";
import { parsePresets, presetsFromUrl } from "../src/presets";

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
});
