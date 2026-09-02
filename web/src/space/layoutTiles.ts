import type { WallRun } from "../feelTypes";

export interface GridCell {
  i: number;
  j: number;
}

export function gridCellKey(cell: GridCell): string {
  return `${cell.i},${cell.j}`;
}

function tileUnderRun(run: WallRun, panel: number): GridCell {
  return run.axis === "x"
    ? { i: run.start[0] + panel + 0.5, j: run.start[1] - 0.5 }
    : { i: run.start[0] - 0.5, j: run.start[1] + panel + 0.5 };
}

function panelIsDoor(run: WallRun, panel: number): boolean {
  if (run.door_interval === null) return false;
  const centre = panel + 0.5;
  return centre >= run.door_interval[0] && centre <= run.door_interval[1];
}

function meetingCorner(left: WallRun, right: WallRun): GridCell | null {
  if (left.axis === right.axis) return null;
  const runX = left.axis === "x" ? left : right;
  const runZ = left.axis === "z" ? left : right;
  const intersectionX = runZ.start[0];
  const intersectionZ = runX.start[1];
  const liesOnXRun =
    intersectionX >= runX.start[0] && intersectionX <= runX.start[0] + runX.cells;
  const liesOnZRun =
    intersectionZ >= runZ.start[1] && intersectionZ <= runZ.start[1] + runZ.cells;
  return liesOnXRun && liesOnZRun
    ? { i: intersectionX - 0.5, j: intersectionZ - 0.5 }
    : null;
}

export function wallAndDoorTiles(
  wallRuns: readonly WallRun[],
  cells: ReadonlySet<string>,
): { wallTiles: Set<string>; doorTiles: Set<string> } {
  const wallTiles = new Set<string>();
  const doorCandidates = new Set<string>();
  for (const run of wallRuns) {
    for (let panel = 0; panel < run.cells; panel += 1) {
      const key = gridCellKey(tileUnderRun(run, panel));
      if (!cells.has(key)) continue;
      if (panelIsDoor(run, panel)) doorCandidates.add(key);
      else wallTiles.add(key);
    }
  }
  for (let left = 0; left < wallRuns.length; left += 1) {
    for (let right = left + 1; right < wallRuns.length; right += 1) {
      const corner = meetingCorner(wallRuns[left]!, wallRuns[right]!);
      if (corner === null) continue;
      const key = gridCellKey(corner);
      if (cells.has(key)) wallTiles.add(key);
    }
  }
  return {
    wallTiles,
    doorTiles: new Set([...doorCandidates].filter((key) => !wallTiles.has(key))),
  };
}
