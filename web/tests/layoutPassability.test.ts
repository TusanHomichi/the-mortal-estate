import { describe, expect, it } from "vitest";
import type { FeelLayout, WallRun } from "../src/feelTypes";
import { canStep, passabilityFrom } from "../src/walk/layoutPassability";

function layout(wallRuns: WallRun[] = [], props: FeelLayout["props"] = []): FeelLayout {
  return {
    grid_extents: { i: 3, j: 3 },
    cells: Array.from({ length: 9 }, (_, index) => ({
      i: index % 3,
      j: Math.floor(index / 3),
      material: "ground",
    })),
    wall_runs: wallRuns,
    props,
    light_sources: { lantern_glass: [0, 0, 0], candles: [] },
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
          },
        ],
      ),
    );
    expect(canStep(passability, { i: 0, j: 1 }, { i: 1, j: 1 })).toBe(false);
  });
});
