/** Shortest-step routing for the local, non-authoritative walk experiment. */
import { WALK_MAX_ROUTE_SQUARES } from "./movement";
import { canStep, cellKey, type Cell, type LayoutPassability } from "./layoutPassability";
import { shortestRoute } from "./shortestRoute";

export function authorRoute(passability: LayoutPassability, from: Cell, to: Cell): Cell[] | null {
  if (!passability.cells.has(cellKey(to)) || passability.blocked.has(cellKey(to))) return null;
  return shortestRoute(from, to, WALK_MAX_ROUTE_SQUARES, (a, b) => canStep(passability, a, b));
}
