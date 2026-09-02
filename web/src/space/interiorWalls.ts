import type { FeelSpace, WallRun } from "../feelTypes";
import { gridCellKey } from "./layoutTiles";

/**
 * Finds wall runs whose camera-facing side has no floor. With the ruled camera
 * at +x/+z, those are the room edges that should stop at the sill.
 */
export function nearWallRunIndices(space: FeelSpace): Set<number> {
  if (space.roofs.length > 0) return new Set();
  const cells = new Set(space.cells.map(gridCellKey));
  const near = new Set<number>();
  space.wall_runs.forEach((run, runIndex) => {
    if (!runHasFloorOnCameraSide(run, cells)) near.add(runIndex);
  });
  return near;
}

function runHasFloorOnCameraSide(run: WallRun, cells: ReadonlySet<string>): boolean {
  for (let panel = 0; panel < run.cells; panel += 1) {
    const cell = run.axis === "x"
      ? { i: run.start[0] + panel + 0.5, j: run.start[1] + 0.5 }
      : { i: run.start[0] + 0.5, j: run.start[1] + panel + 0.5 };
    if (cells.has(gridCellKey(cell))) return true;
  }
  return false;
}
