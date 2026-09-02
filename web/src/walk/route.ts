/**
 * Direct-route authoring for the local, non-authoritative walk experiment.
 *
 * The target supplies the route. Each unit step closes both differing axes
 * first, then the remaining axis. An obstruction refuses that authored route;
 * this module never searches for another one.
 */
import { WALK_MAX_ROUTE_SQUARES } from "./beat";
import {
  canStep,
  sameCell,
  type Cell,
  type LayoutPassability,
} from "./layoutPassability";

function direction(delta: number): number {
  return Math.sign(delta);
}

export function authorRoute(
  passability: LayoutPassability,
  from: Cell,
  to: Cell,
): Cell[] | null {
  const distance = Math.max(Math.abs(to.i - from.i), Math.abs(to.j - from.j));
  if (distance === 0 || distance > WALK_MAX_ROUTE_SQUARES) return null;

  const route: Cell[] = [{ ...from }];
  let current = { ...from };
  while (!sameCell(current, to)) {
    const next = {
      i: current.i + direction(to.i - current.i),
      j: current.j + direction(to.j - current.j),
    };
    if (!canStep(passability, current, next)) return null;
    route.push(next);
    current = next;
  }
  return route;
}
