import { describe, expect, it } from "vitest";
import { assertNormalSheetsMatch, isNormalSheetKey } from "../src/space/textures";

describe("normal sheets at decode time", () => {
  it("names a normal sheet by its key suffix", () => {
    expect(isNormalSheetKey("props/caretaker/normal")).toBe(true);
    expect(isNormalSheetKey("props/caretaker")).toBe(false);
  });

  it("accepts a normal sheet the size of its colour sheet", () => {
    const textures = new Map([
      ["props/caretaker", { width: 1254, height: 1254 }],
      ["props/caretaker/normal", { width: 1254, height: 1254 }],
      ["terrain/grass", { width: 512, height: 512 }],
    ]);
    expect(() => assertNormalSheetsMatch(textures)).not.toThrow();
  });

  it("refuses a mismatched normal sheet before any space is built", () => {
    const textures = new Map([
      ["props/kit_bed_twin1", { width: 760, height: 1428 }],
      ["props/kit_bed_twin1/normal", { width: 760, height: 1400 }],
    ]);
    expect(() => assertNormalSheetsMatch(textures)).toThrow(/760x1400 but its colour sheet is 760x1428/);
  });

  it("refuses a normal sheet with no colour sheet", () => {
    const textures = new Map([["props/ghost/normal", { width: 8, height: 8 }]]);
    expect(() => assertNormalSheetsMatch(textures)).toThrow(/has no colour sheet props\/ghost/);
  });
});
