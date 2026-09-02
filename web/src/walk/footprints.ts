import type { Cell } from "./layoutPassability";

export interface FootprintPoint {
  x: number;
  z: number;
}

export interface FootprintPlacement {
  printIndex: number;
  pathIndex: number;
  foot: "left" | "right";
  angle: number;
  position: FootprintPoint;
}

const PRINTS_PER_SQUARE = 2;
const LATERAL_OFFSET = 0.07;

function directionAlong(from: Cell, to: Cell): FootprintPoint {
  const length = Math.hypot(to.i - from.i, to.j - from.j) || 1;
  return { x: (to.i - from.i) / length, z: (to.j - from.j) / length };
}

/**
 * Places one alternating stride along a locally authored route.
 *
 * The route is presentation-only evidence in the feel experiment. These marks
 * explain that route; they neither supply nor imply authoritative walkability.
 */
export function footprintsFromPath(path: readonly Cell[]): FootprintPlacement[] {
  const placements: FootprintPlacement[] = [];

  for (let pathIndex = 1; pathIndex < path.length; pathIndex += 1) {
    const from = path[pathIndex - 1]!;
    const to = path[pathIndex]!;
    const direction = directionAlong(from, to);
    const left = { x: -direction.z, z: direction.x };

    for (let halfStep = 1; halfStep <= PRINTS_PER_SQUARE; halfStep += 1) {
      const printIndex = placements.length;
      const foot: FootprintPlacement["foot"] = printIndex % 2 === 0 ? "left" : "right";
      const progress = halfStep / PRINTS_PER_SQUARE;
      const side = foot === "left" ? 1 : -1;
      placements.push({
        printIndex,
        pathIndex,
        foot,
        angle: Math.atan2(direction.x, direction.z),
        position: {
          x: from.i + (to.i - from.i) * progress + left.x * LATERAL_OFFSET * side,
          z: from.j + (to.j - from.j) * progress + left.z * LATERAL_OFFSET * side,
        },
      });
    }
  }

  return placements;
}
