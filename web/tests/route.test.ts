import { describe, expect, it } from "vitest";
import type { FeelLayout, WallRun } from "../src/feelTypes";
import { passabilityFrom } from "../src/walk/layoutPassability";
import { authorRoute } from "../src/walk/route";

function field(walls: WallRun[] = []): FeelLayout {
  return {
    grid_extents: { i: 7, j: 7 },
    cells: Array.from({ length: 49 }, (_, index) => ({
      i: index % 7,
      j: Math.floor(index / 7),
      material: "ground",
    })),
    wall_runs: walls,
    props: [],
    light_sources: { lantern_glass: [0, 0, 0], candles: [] },
  };
}

describe("direct route authoring", () => {
  it("a route is the direct line and never routes around a wall", () => {
    const open = passabilityFrom(field());
    expect(authorRoute(open, { i: 1, j: 1 }, { i: 4, j: 3 })).toEqual([
      { i: 1, j: 1 },
      { i: 2, j: 2 },
      { i: 3, j: 3 },
      { i: 4, j: 3 },
    ]);

    const blockingWall: WallRun = {
      axis: "x",
      start: [1.5, 1.5],
      cells: 1,
      door_interval: null,
    };
    expect(
      authorRoute(
        passabilityFrom(field([blockingWall])),
        { i: 1, j: 1 },
        { i: 3, j: 3 },
      ),
    ).toBeNull();
  });

  it("a target beyond three squares cannot be authored", () => {
    expect(
      authorRoute(passabilityFrom(field()), { i: 1, j: 1 }, { i: 5, j: 1 }),
    ).toBeNull();
  });
});
