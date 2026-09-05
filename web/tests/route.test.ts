import { describe, expect, it } from "vitest";
import type { FeelSpace, WallRun } from "../src/feelTypes";
import { canStep, passabilityFrom } from "../src/walk/layoutPassability";
import { authorRoute } from "../src/walk/route";

function field(walls: WallRun[] = []): FeelSpace {
  return {
    grid_extents: { i: 7, j: 7 },
    cells: Array.from({ length: 49 }, (_, index) => ({
      i: index % 7,
      j: Math.floor(index / 7),
      material: "ground",
    })),
    wall_runs: walls,
    roofs: [],
    props: [],
    fixtures: [],
    structures: [],
    light_sources: { lantern_glass: null, candles: [] },
    weather: false,
    portals: [],
  };
}

describe("shortest route authoring", () => {
  it("keeps open-ground routes direct and finds legal detours around walls", () => {
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
    const blocked = passabilityFrom(field([blockingWall]));
    const detour = authorRoute(blocked, { i: 1, j: 1 }, { i: 3, j: 3 });
    expect(detour).not.toBeNull();
    expect(detour![detour!.length - 1]).toEqual({ i: 3, j: 3 });
    expect(detour!.length - 1).toBeLessThanOrEqual(3);
    expect(detour!.slice(1).every((cell, index) => canStep(blocked, detour![index]!, cell))).toBe(true);
  });

  it("routes around a table without cutting its blocked corners", () => {
    const open = passabilityFrom(field());
    const table = { ...open, blocked: new Set(["3,3"]) };
    const from = { i: 3, j: 4 };
    const to = { i: 2, j: 2 };
    const route = authorRoute(table, from, to);
    expect(route).toEqual([from, { i: 2, j: 4 }, { i: 2, j: 3 }, to]);
    expect(route!.slice(1).every((cell, index) => canStep(table, route![index]!, cell))).toBe(true);
    // The directly opposite tile needs four legal steps, outside one move.
    expect(authorRoute(table, from, { i: 3, j: 2 })).toBeNull();
  });

  it("a target beyond three squares cannot be authored", () => {
    expect(
      authorRoute(passabilityFrom(field()), { i: 1, j: 1 }, { i: 5, j: 1 }),
    ).toBeNull();
  });

  it("a route ending on a wall tile cannot be authored", () => {
    const wall: WallRun = {
      axis: "x",
      start: [0.5, 2.5],
      cells: 3,
      door_interval: null,
    };
    expect(
      authorRoute(passabilityFrom(field([wall])), { i: 2, j: 1 }, { i: 2, j: 2 }),
    ).toBeNull();
  });
});
