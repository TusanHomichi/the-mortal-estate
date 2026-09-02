import {
  canStep,
  cellKey,
  sameCell,
  type Cell,
  type LayoutPassability,
} from "./layoutPassability";

const NEIGHBOURS: readonly Cell[] = [
  { i: -1, j: -1 },
  { i: -1, j: 0 },
  { i: -1, j: 1 },
  { i: 0, j: -1 },
  { i: 0, j: 1 },
  { i: 1, j: -1 },
  { i: 1, j: 0 },
  { i: 1, j: 1 },
];

interface SearchNode {
  cell: Cell;
  cost: number;
  estimate: number;
  order: number;
}

function octileDistance(from: Cell, to: Cell): number {
  const deltaI = Math.abs(to.i - from.i);
  const deltaJ = Math.abs(to.j - from.j);
  const diagonal = Math.min(deltaI, deltaJ);
  return Math.max(deltaI, deltaJ) + (Math.SQRT2 - 1) * diagonal;
}

function compareNodes(left: SearchNode, right: SearchNode): number {
  const leftTotal = left.cost + left.estimate;
  const rightTotal = right.cost + right.estimate;
  return (
    leftTotal - rightTotal ||
    left.estimate - right.estimate ||
    left.cell.i - right.cell.i ||
    left.cell.j - right.cell.j ||
    left.order - right.order
  );
}

function rebuildPath(cameFrom: ReadonlyMap<string, Cell>, destination: Cell): Cell[] {
  const path = [{ ...destination }];
  let cursor = destination;
  for (;;) {
    const previous = cameFrom.get(cellKey(cursor));
    if (previous === undefined) break;
    path.push({ ...previous });
    cursor = previous;
  }
  return path.reverse();
}

export function findPath(
  passability: LayoutPassability,
  from: Cell,
  to: Cell,
): Cell[] | null {
  if (
    !passability.cells.has(cellKey(from)) ||
    !passability.cells.has(cellKey(to)) ||
    passability.blocked.has(cellKey(from)) ||
    passability.blocked.has(cellKey(to))
  ) {
    return null;
  }
  if (sameCell(from, to)) return [{ ...from }];

  let order = 0;
  const open: SearchNode[] = [
    { cell: { ...from }, cost: 0, estimate: octileDistance(from, to), order: order++ },
  ];
  const bestCost = new Map([[cellKey(from), 0]]);
  const cameFrom = new Map<string, Cell>();

  while (open.length > 0) {
    open.sort(compareNodes);
    const current = open.shift()!;
    if (current.cost !== bestCost.get(cellKey(current.cell))) continue;
    if (sameCell(current.cell, to)) return rebuildPath(cameFrom, current.cell);

    for (const offset of NEIGHBOURS) {
      const neighbour = { i: current.cell.i + offset.i, j: current.cell.j + offset.j };
      if (!canStep(passability, current.cell, neighbour)) continue;
      const stepCost = offset.i !== 0 && offset.j !== 0 ? Math.SQRT2 : 1;
      const cost = current.cost + stepCost;
      const key = cellKey(neighbour);
      const known = bestCost.get(key);
      if (known !== undefined && cost >= known) continue;
      bestCost.set(key, cost);
      cameFrom.set(key, current.cell);
      open.push({
        cell: neighbour,
        cost,
        estimate: octileDistance(neighbour, to),
        order: order++,
      });
    }
  }
  return null;
}
