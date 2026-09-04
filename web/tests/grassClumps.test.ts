import { describe, expect, it } from "vitest";
import { scatterGrassClumps } from "../src/grassClumps";
import type { FeelSpace } from "../src/feelTypes";

function space(weather = true): FeelSpace {
  return {
    grid_extents: { i: 4, j: 1 },
    cells: [
      { i: 0, j: 0, material: "grass" },
      { i: 1, j: 0, material: "grass" },
      { i: 2, j: 0, material: "grass" },
      { i: 3, j: 0, material: "stone" },
    ],
    wall_runs: [],
    roofs: [],
    props: [],
    fixtures: [],
    structures: [],
    light_sources: { lantern_glass: null, candles: [] },
    weather,
    portals: [],
  };
}

describe("grass clump scatter", () => {
  it("is deterministic, grass-only, and denser within two tiles of a lane", () => {
    const first = scatterGrassClumps(space());
    expect(scatterGrassClumps(space())).toEqual(first);
    expect(first.filter((clump) => clump.x < 0.5)).toHaveLength(1);
    expect(first.filter((clump) => clump.x >= 0.5 && clump.x < 1.5).length).toBeGreaterThanOrEqual(3);
    expect(first.every((clump) => clump.x < 3.5)).toBe(true);
    expect(first.every((clump) => clump.scale >= 0.8 && clump.scale <= 1.2)).toBe(true);
  });

  it("honours the instance cap and emits none for an interior", () => {
    expect(scatterGrassClumps(space(), 4)).toHaveLength(4);
    expect(scatterGrassClumps(space(false))).toEqual([]);
  });
});
