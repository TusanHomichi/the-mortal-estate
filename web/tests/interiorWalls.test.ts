import { describe, expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { nearWallRunIndices } from "../src/space/interiorWalls";

function room(): FeelSpace {
  return {
    grid_extents: { i: 9, j: 5 },
    cells: Array.from({ length: 45 }, (_, index) => ({
      i: index % 9,
      j: Math.floor(index / 9),
      material: "floor_planks",
    })),
    wall_runs: [
      { axis: "x", start: [0.5, 0.5], cells: 8, door_interval: null },
      { axis: "z", start: [0.5, 0.5], cells: 4, door_interval: null },
      { axis: "x", start: [0.5, 4.5], cells: 8, door_interval: [3.15, 3.85] },
      { axis: "z", start: [8.5, 0.5], cells: 4, door_interval: null },
    ],
    roofs: [],
    props: [],
    light_sources: { lantern_glass: null, candles: [] },
    weather: false,
    portals: [],
  };
}

describe("interior near-wall selection", () => {
  it("derives the south and east runs from absent camera-side floor", () => {
    expect(nearWallRunIndices(room())).toEqual(new Set([2, 3]));
  });

  it("never cuts down exterior walls", () => {
    const exterior = room();
    exterior.roofs.push({
      footprint: { i0: 0, j0: 0, i1: 8, j1: 4 },
      ridge_axis: "x",
      eave_height: 2.2,
      ridge_height: 3.2,
      material: "shingle",
    });
    expect(nearWallRunIndices(exterior)).toEqual(new Set());
  });
});
