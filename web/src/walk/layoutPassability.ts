/**
 * Presentation-only passability for the browser feel scene's walk experiment.
 *
 * This guesses walkability from the candidate packet's cells, wall runs, door
 * interval, and prop placement. It is local, non-authoritative, and must never
 * be used by the real client, which may present only walkability received from
 * authority (docs/boundary-map.md#15-fail-closed-terrain-composition).
 */
import type { FeelLayout, WallRun } from "../feelTypes";

export interface Cell {
  i: number;
  j: number;
}

export interface LayoutPassability {
  readonly cells: ReadonlySet<string>;
  readonly blocked: ReadonlySet<string>;
  readonly wallRuns: readonly WallRun[];
}

const NON_WALKABLE_PROPS = new Set([
  "tree",
  "lantern_post",
  "shrine_table",
  "grave_marker",
]);

export function cellKey(cell: Cell): string {
  return `${cell.i},${cell.j}`;
}

export function sameCell(left: Cell, right: Cell): boolean {
  return left.i === right.i && left.j === right.j;
}

export function passabilityFrom(layout: FeelLayout): LayoutPassability {
  const cells = new Set(layout.cells.map((cell) => cellKey(cell)));
  const blocked = new Set<string>();
  for (const prop of layout.props) {
    if (!NON_WALKABLE_PROPS.has(prop.kind)) continue;
    const occupied = {
      i: Math.round(prop.cell_anchor[0]),
      j: Math.round(prop.cell_anchor[1]),
    };
    if (cells.has(cellKey(occupied))) blocked.add(cellKey(occupied));
  }
  return { cells, blocked, wallRuns: layout.wall_runs };
}

function doorContains(run: WallRun, runPosition: number): boolean {
  if (run.door_interval === null) return false;
  const local = runPosition - (run.axis === "x" ? run.start[0] : run.start[1]);
  return local >= run.door_interval[0] && local <= run.door_interval[1];
}

function wallBlocksOrthogonalStep(run: WallRun, from: Cell, to: Cell): boolean {
  if (run.axis === "x" && from.i === to.i && from.j !== to.j) {
    const boundary = (from.j + to.j) / 2;
    const runEnd = run.start[0] + run.cells;
    return (
      boundary === run.start[1] &&
      from.i >= run.start[0] &&
      from.i <= runEnd &&
      !doorContains(run, from.i)
    );
  }
  if (run.axis === "z" && from.j === to.j && from.i !== to.i) {
    const boundary = (from.i + to.i) / 2;
    const runEnd = run.start[1] + run.cells;
    return (
      boundary === run.start[0] &&
      from.j >= run.start[1] &&
      from.j <= runEnd &&
      !doorContains(run, from.j)
    );
  }
  return false;
}

function canStepOrthogonally(passability: LayoutPassability, from: Cell, to: Cell): boolean {
  if (!passability.cells.has(cellKey(from)) || !passability.cells.has(cellKey(to))) return false;
  if (passability.blocked.has(cellKey(from)) || passability.blocked.has(cellKey(to))) return false;
  return !passability.wallRuns.some((run) => wallBlocksOrthogonalStep(run, from, to));
}

export function canStep(passability: LayoutPassability, from: Cell, to: Cell): boolean {
  const deltaI = to.i - from.i;
  const deltaJ = to.j - from.j;
  if (
    (deltaI === 0 && deltaJ === 0) ||
    Math.abs(deltaI) > 1 ||
    Math.abs(deltaJ) > 1 ||
    !passability.cells.has(cellKey(from)) ||
    !passability.cells.has(cellKey(to)) ||
    passability.blocked.has(cellKey(from)) ||
    passability.blocked.has(cellKey(to))
  ) {
    return false;
  }
  if (deltaI === 0 || deltaJ === 0) return canStepOrthogonally(passability, from, to);

  const acrossI = { i: to.i, j: from.j };
  const acrossJ = { i: from.i, j: to.j };
  return (
    canStepOrthogonally(passability, from, acrossI) &&
    canStepOrthogonally(passability, acrossI, to) &&
    canStepOrthogonally(passability, from, acrossJ) &&
    canStepOrthogonally(passability, acrossJ, to)
  );
}
