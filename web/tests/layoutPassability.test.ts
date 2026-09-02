import { describe, expect, it } from "vitest";
import type { FeelSpace, WallRun } from "../src/feelTypes";
import { canStep, passabilityFrom } from "../src/walk/layoutPassability";

function layout(wallRuns: WallRun[] = [], props: FeelSpace["props"] = []): FeelSpace {
  return {
    grid_extents: { i: 3, j: 3 },
    cells: Array.from({ length: 9 }, (_, index) => ({
      i: index % 3,
      j: Math.floor(index / 3),
      material: "ground",
    })),
    wall_runs: wallRuns,
    roofs: [],
    props,
    light_sources: { lantern_glass: null, candles: [] },
    weather: false,
    portals: [],
  };
}

const WALL: WallRun = {
  axis: "x",
  start: [-0.5, 1.5],
  cells: 3,
  door_interval: null,
};

describe("packet-layout passability", () => {
  it("every tile under each wall run is blocked", () => {
    const passability = passabilityFrom(
      layout([WALL, { axis: "z", start: [1.5, -0.5], cells: 3, door_interval: null }]),
    );
    expect(passability.wallTiles).toEqual(
      new Set(["0,1", "1,1", "2,1", "1,0", "1,2"]),
    );
    for (const key of passability.wallTiles) expect(passability.blocked.has(key)).toBe(true);
  });

  it("the door tile is passable from both sides", () => {
    const passability = passabilityFrom(
      layout([{ ...WALL, door_interval: [1.4, 1.6] }]),
    );
    expect(passability.doorTiles).toEqual(new Set(["1,1"]));
    expect(passability.blocked.has("1,1")).toBe(false);
    expect(canStep(passability, { i: 1, j: 2 }, { i: 1, j: 1 })).toBe(true);
    expect(canStep(passability, { i: 1, j: 0 }, { i: 1, j: 1 })).toBe(true);

    const acrossZRun = passabilityFrom(
      layout([
        { axis: "z", start: [1.5, -0.5], cells: 3, door_interval: [1.4, 1.6] },
      ]),
    );
    expect(acrossZRun.doorTiles).toEqual(new Set(["1,1"]));
    expect(canStep(acrossZRun, { i: 2, j: 1 }, { i: 1, j: 1 })).toBe(true);
    expect(canStep(acrossZRun, { i: 0, j: 1 }, { i: 1, j: 1 })).toBe(true);
  });

  it("a diagonal past a wall corner is refused", () => {
    const passability = passabilityFrom(layout([WALL]));
    expect(canStep(passability, { i: 0, j: 2 }, { i: 1, j: 1 })).toBe(false);
  });

  it("a diagonal cannot cut the far end corner of a wall", () => {
    const passability = passabilityFrom(
      layout([{ axis: "x", start: [1.5, 1.5], cells: 1, door_interval: null }]),
    );
    expect(canStep(passability, { i: 1, j: 1 }, { i: 2, j: 2 })).toBe(false);
  });

  it("the meeting corner tile belongs to the wall", () => {
    const passability = passabilityFrom(
      layout([
        { axis: "x", start: [0.5, 0.5], cells: 2, door_interval: null },
        { axis: "z", start: [0.5, 0.5], cells: 2, door_interval: null },
      ]),
    );
    expect(passability.wallTiles.has("0,0")).toBe(true);
    expect(passability.blocked.has("0,0")).toBe(true);
  });

  it("the tile behind a wall tile remains walkable", () => {
    const passability = passabilityFrom(layout([WALL]));
    expect(passability.blocked.has("0,0")).toBe(false);
    expect(canStep(passability, { i: 0, j: 0 }, { i: 1, j: 0 })).toBe(true);
  });

  it("every tree-prefixed prop kind blocks its cell", () => {
    const passability = passabilityFrom(
      layout(
        [],
        [
          {
            kind: "tree_broad",
            cell_anchor: [1.2, 0.8],
            nominal_height: 2,
            sway: true,
            mirror: true,
            facing: "view",
          },
        ],
      ),
    );
    expect(canStep(passability, { i: 0, j: 1 }, { i: 1, j: 1 })).toBe(false);
  });

  it("every roof-footprint tile blocks except its portal tile", () => {
    const roofed = layout([
      { ...WALL, door_interval: [1.4, 1.6] },
    ]);
    roofed.roofs = [{
      footprint: { i0: 0, j0: 0, i1: 2, j1: 1 },
      ridge_axis: "x",
      eave_height: 2,
      ridge_height: 3,
      material: "shingle",
    }];
    roofed.portals = [{ cell: [1, 1], to: { space: "room", cell: [1, 2] } }];

    const passability = passabilityFrom(roofed);
    expect(passability.roofTiles).toEqual(new Set(["0,0", "1,0", "2,0", "0,1", "1,1", "2,1"]));
    expect(passability.blocked.has("0,0")).toBe(true);
    expect(passability.blocked.has("1,1")).toBe(false);
  });
});
