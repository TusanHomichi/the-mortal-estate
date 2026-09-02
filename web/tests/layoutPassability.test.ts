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
  start: [-0.5, 0.5],
  cells: 3,
  door_interval: null,
};

describe("packet-layout passability", () => {
  it("a wall blocks its crossing", () => {
    const passability = passabilityFrom(layout([WALL]));
    expect(canStep(passability, { i: 0, j: 0 }, { i: 0, j: 1 })).toBe(false);
  });

  it("the door interval opens exactly one crossing", () => {
    const passability = passabilityFrom(
      layout([{ ...WALL, door_interval: [1.4, 1.6] }]),
    );
    expect(canStep(passability, { i: 1, j: 0 }, { i: 1, j: 1 })).toBe(true);
    expect(canStep(passability, { i: 0, j: 0 }, { i: 0, j: 1 })).toBe(false);
    expect(canStep(passability, { i: 2, j: 0 }, { i: 2, j: 1 })).toBe(false);
  });

  it("a diagonal past a wall corner is refused", () => {
    const passability = passabilityFrom(layout([WALL]));
    expect(canStep(passability, { i: 0, j: 0 }, { i: 1, j: 1 })).toBe(false);
  });

  it("a diagonal cannot cut the far end corner of a wall", () => {
    const passability = passabilityFrom(
      layout([{ axis: "x", start: [1.5, 1.5], cells: 1, door_interval: null }]),
    );
    expect(canStep(passability, { i: 1, j: 1 }, { i: 2, j: 2 })).toBe(false);
  });

  it("prop cells are blocked", () => {
    const passability = passabilityFrom(
      layout([], [{ kind: "tree", cell_anchor: [1.2, 0.8], nominal_height: 2, sway: true }]),
    );
    expect(canStep(passability, { i: 0, j: 1 }, { i: 1, j: 1 })).toBe(false);
  });
});
