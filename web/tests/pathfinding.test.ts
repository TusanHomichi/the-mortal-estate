import { describe, expect, it } from "vitest";
import type { FeelLayout, WallRun } from "../src/feelTypes";
import { passabilityFrom } from "../src/walk/layoutPassability";
import { findPath } from "../src/walk/pathfinding";

function field(walls: WallRun[] = [], blocked: FeelLayout["props"] = []): FeelLayout {
  return {
    grid_extents: { i: 5, j: 5 },
    cells: Array.from({ length: 25 }, (_, index) => ({
      i: index % 5,
      j: Math.floor(index / 5),
      material: "ground",
    })),
    wall_runs: walls,
    props: blocked,
    light_sources: { lantern_glass: [0, 0, 0], candles: [] },
  };
}

describe("deterministic eight-neighbour pathfinding", () => {
  it("the shortest path over an open field is the Chebyshev distance", () => {
    const path = findPath(passabilityFrom(field()), { i: 0, j: 0 }, { i: 4, j: 3 });
    expect(path).not.toBeNull();
    expect(path!.length - 1).toBe(4);
  });

  it("a walled cell is routed around", () => {
    const wall: WallRun = {
      axis: "x",
      start: [-0.5, 0.5],
      cells: 1,
      door_interval: null,
    };
    const path = findPath(passabilityFrom(field([wall])), { i: 0, j: 0 }, { i: 2, j: 2 });
    expect(path).not.toBeNull();
    expect(path).not.toContainEqual({ i: 1, j: 1 });
    expect(path!.length - 1).toBeGreaterThan(2);
  });

  it("an unreachable destination returns null", () => {
    const tree = { kind: "tree", cell_anchor: [2, 2] as [number, number], nominal_height: 2, sway: true };
    expect(findPath(passabilityFrom(field([], [tree])), { i: 0, j: 0 }, { i: 2, j: 2 })).toBeNull();
  });

  it("identical inputs produce identical paths", () => {
    const passability = passabilityFrom(field());
    const first = findPath(passability, { i: 0, j: 1 }, { i: 4, j: 3 });
    const second = findPath(passability, { i: 0, j: 1 }, { i: 4, j: 3 });
    expect(second).toEqual(first);
  });
});
