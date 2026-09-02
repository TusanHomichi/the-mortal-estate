import { describe, expect, it } from "vitest";
import type { WallRun } from "../src/feelTypes";
import { occludingRuns } from "../src/walk/wallOcclusion";

const runX: WallRun = {
  axis: "x",
  start: [3.5, 6.5],
  cells: 4,
  door_interval: null,
};

describe("camera-space wall occlusion", () => {
  it("selects the tile just behind a run", () => {
    expect(occludingRuns([runX], { i: 5, j: 6 }, 5.4)).toEqual([runX]);
  });

  it("refuses a tile beyond the run's projected cover", () => {
    expect(occludingRuns([runX], { i: 5, j: 1 }, 5.4)).toEqual([]);
  });

  it("refuses a tile in front of the run", () => {
    expect(occludingRuns([runX], { i: 5, j: 7 }, 5.4)).toEqual([]);
  });

  it("includes the slack tile beside a run's end", () => {
    expect(occludingRuns([runX], { i: 8, j: 6 }, 5.4)).toEqual([runX]);
  });

  it("handles a run on the other axis symmetrically", () => {
    const runZ: WallRun = {
      axis: "z",
      start: [8.5, 2.5],
      cells: 4,
      door_interval: null,
    };
    expect(occludingRuns([runZ], { i: 8, j: 4 }, 5.4)).toEqual([runZ]);
  });
});
