import type { Cell } from "./layoutPassability";

export interface FootprintPoint {
  x: number;
  z: number;
}

export interface FootprintPair {
  pathIndex: number;
  angle: number;
  lead: "left" | "right";
  left: FootprintPoint;
  right: FootprintPoint;
}

const LATERAL_OFFSET = 0.06;
const STRIDE_OFFSET = 0.035;

function directionInto(path: readonly Cell[], index: number): Cell {
  const from = path[index - 1]!;
  const to = path[index]!;
  const length = Math.hypot(to.i - from.i, to.j - from.j) || 1;
  return { i: (to.i - from.i) / length, j: (to.j - from.j) / length };
}

export function footprintsFromPath(path: readonly Cell[]): FootprintPair[] {
  return path.slice(1).map((cell, enteredIndex) => {
    const pathIndex = enteredIndex + 1;
    const direction = directionInto(path, pathIndex);
    const perpendicular = { i: -direction.j, j: direction.i };
    const lead: FootprintPair["lead"] = enteredIndex % 2 === 0 ? "left" : "right";
    const leftStride = lead === "left" ? STRIDE_OFFSET : -STRIDE_OFFSET;
    const rightStride = -leftStride;
    return {
      pathIndex,
      angle: Math.atan2(direction.i, direction.j),
      lead,
      left: {
        x: cell.i + perpendicular.i * LATERAL_OFFSET + direction.i * leftStride,
        z: cell.j + perpendicular.j * LATERAL_OFFSET + direction.j * leftStride,
      },
      right: {
        x: cell.i - perpendicular.i * LATERAL_OFFSET + direction.i * rightStride,
        z: cell.j - perpendicular.j * LATERAL_OFFSET + direction.j * rightStride,
      },
    };
  });
}
