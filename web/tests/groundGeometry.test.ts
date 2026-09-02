import { describe, expect, it } from "vitest";
import { buildGroundGeometry } from "../src/groundGeometry";

describe("batched ground geometry", () => {
  it("carries each cell origin on all four of that cell's vertices", () => {
    const geometry = buildGroundGeometry([
      { i: 2, j: 3, material: "grass" },
      { i: 8, j: 5, material: "grass" },
    ]);
    expect(geometry.positions).toHaveLength(24);
    expect(geometry.uvs).toHaveLength(16);
    expect(geometry.indices).toEqual([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    expect(geometry.cellOrigins).toEqual([
      2, 3, 2, 3, 2, 3, 2, 3,
      8, 5, 8, 5, 8, 5, 8, 5,
    ]);
  });
});
