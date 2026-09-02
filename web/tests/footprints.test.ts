import { describe, expect, it } from "vitest";
import { footprintsFromPath } from "../src/walk/footprints";

describe("walk footprint placement", () => {
  it("starts on the first cell stepped into and alternates the leading foot", () => {
    const pairs = footprintsFromPath([
      { i: 0, j: 0 },
      { i: 1, j: 0 },
      { i: 2, j: 0 },
    ]);
    expect(pairs.map((pair) => pair.pathIndex)).toEqual([1, 2]);
    expect(pairs.map((pair) => pair.lead)).toEqual(["left", "right"]);
    expect(pairs[0]!.left.x).toBeGreaterThan(pairs[0]!.right.x);
    expect(pairs[1]!.right.x).toBeGreaterThan(pairs[1]!.left.x);
  });

  it("orients each pair along the direction of travel into its cell", () => {
    const pairs = footprintsFromPath([
      { i: 0, j: 0 },
      { i: 1, j: 0 },
      { i: 1, j: 1 },
    ]);
    expect(pairs[0]!.angle).toBeCloseTo(Math.PI / 2);
    expect(pairs[1]!.angle).toBeCloseTo(0);
  });
});
