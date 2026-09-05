/** Shortest-step routing for the local, non-authoritative walk experiment. */
import { WALK_MAX_ROUTE_SQUARES } from "./movement";
import { canStep, cellKey, sameCell, type Cell, type LayoutPassability } from "./layoutPassability";

const NEIGHBOURS = [
  [-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1],
] as const;

function distanceSquared(from: Cell, to: Cell): number {
  return (to.i - from.i) ** 2 + (to.j - from.j) ** 2;
}

export function authorRoute(passability: LayoutPassability, from: Cell, to: Cell): Cell[] | null {
  const distance = Math.max(Math.abs(to.i - from.i), Math.abs(to.j - from.j));
  if (distance === 0 || distance > WALK_MAX_ROUTE_SQUARES) return null;
  if (!passability.cells.has(cellKey(to)) || passability.blocked.has(cellKey(to))) return null;

  // Breadth-first search minimizes charged steps. The target-facing neighbour
  // order makes equal-length choices stable and keeps open-ground routes direct.
  const queue: Cell[][] = [[{ ...from }]];
  const seen = new Set([cellKey(from)]);
  for (let head = 0; head < queue.length; head += 1) {
    const route = queue[head]!;
    if (route.length - 1 >= WALK_MAX_ROUTE_SQUARES) continue;
    const current = route[route.length - 1]!;
    const neighbours = NEIGHBOURS.map(([di, dj]) => ({ i: current.i + di, j: current.j + dj }))
      .sort((a, b) => distanceSquared(a, to) - distanceSquared(b, to));
    for (const next of neighbours) {
      if (seen.has(cellKey(next)) || !canStep(passability, current, next)) continue;
      const candidate = [...route, next];
      if (sameCell(next, to)) return candidate;
      seen.add(cellKey(next));
      queue.push(candidate);
    }
  }
  return null;
}
