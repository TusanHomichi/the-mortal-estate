import { describe, expect, it } from "vitest";
import { footprintsFromPath } from "../src/walk/footprints";

describe("walk footprint placement", () => {
  it("lays one alternating stride with exactly two prints per route square", () => {
    const prints = footprintsFromPath([
      { i: 0, j: 0 },
      { i: 1, j: 0 },
      { i: 2, j: 0 },
      { i: 3, j: 0 },
    ]);

    expect(prints).toHaveLength(6);
    expect(prints.map((print) => print.foot)).toEqual([
      "left",
      "right",
      "left",
      "right",
      "left",
      "right",
    ]);
    expect(prints.map((print) => print.position.x)).toEqual([0.5, 1, 1.5, 2, 2.5, 3]);
    expect(prints.map((print) => print.position.z)).toEqual([0.07, -0.07, 0.07, -0.07, 0.07, -0.07]);
  });

  it("points every print away from the origin along its own leg", () => {
    const route = [
      { i: 0, j: 0 },
      { i: 1, j: 0 },
      { i: 2, j: 1 },
      { i: 2, j: 2 },
    ];
    const prints = footprintsFromPath(route);

    for (const print of prints) {
      const from = route[print.pathIndex - 1]!;
      const to = route[print.pathIndex]!;
      const length = Math.hypot(to.i - from.i, to.j - from.j);
      const expected = { x: (to.i - from.i) / length, z: (to.j - from.j) / length };
      const heading = { x: Math.sin(print.angle), z: Math.cos(print.angle) };
      expect(heading.x).toBeCloseTo(expected.x);
      expect(heading.z).toBeCloseTo(expected.z);
    }
  });

  it("keeps a diagonal stride half a leg apart and on opposite sides of its line", () => {
    const prints = footprintsFromPath([
      { i: 2, j: 3 },
      { i: 3, j: 4 },
    ]);
    const diagonal = Math.SQRT1_2;

    expect(prints[0]!.position.x).toBeCloseTo(2.5 - 0.07 * diagonal);
    expect(prints[0]!.position.z).toBeCloseTo(3.5 + 0.07 * diagonal);
    expect(prints[1]!.position.x).toBeCloseTo(3 + 0.07 * diagonal);
    expect(prints[1]!.position.z).toBeCloseTo(4 - 0.07 * diagonal);
  });
});
