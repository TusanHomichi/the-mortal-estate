import type { Cell } from "./layoutPassability";

/** A presentation heading on the ground plane, independent of screen axes. */
export interface FigureFacing {
  i: number;
  j: number;
}

/** Quantize a look target or route segment to the nearest of eight headings. */
export function facingBetween(from: Cell, to: Cell): FigureFacing | null {
  const di = to.i - from.i;
  const dj = to.j - from.j;
  if (Math.hypot(di, dj) < 1e-9) return null;
  const angle = Math.round(Math.atan2(di, dj) / (Math.PI / 4)) * Math.PI / 4;
  return { i: Math.round(Math.sin(angle)), j: Math.round(Math.cos(angle)) };
}

/** The rig's authored forward axis is +z. */
export function facingYaw(direction: FigureFacing): number {
  if (![direction.i, direction.j].every((v) => Number.isInteger(v) && Math.abs(v) <= 1) ||
      (direction.i === 0 && direction.j === 0)) {
    throw new Error("a figure heading must be one of the eight ground directions");
  }
  return Math.atan2(direction.i, direction.j);
}
