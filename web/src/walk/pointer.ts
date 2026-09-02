import type { Cell } from "./layoutPassability";

interface MatrixLike {
  readonly elements: ArrayLike<number>;
}

export interface OrthographicCameraLike {
  readonly left: number;
  readonly right: number;
  readonly top: number;
  readonly bottom: number;
  readonly zoom: number;
  readonly matrixWorld: MatrixLike;
}

interface CanvasLike {
  getBoundingClientRect(): {
    left: number;
    top: number;
    width: number;
    height: number;
  };
}

export interface GridExtents {
  i: number;
  j: number;
}

function transformedPoint(elements: ArrayLike<number>, x: number, y: number): [number, number, number] {
  return [
    elements[0]! * x + elements[4]! * y + elements[12]!,
    elements[1]! * x + elements[5]! * y + elements[13]!,
    elements[2]! * x + elements[6]! * y + elements[14]!,
  ];
}

export function cellUnderPointer(
  camera: OrthographicCameraLike,
  canvas: CanvasLike,
  clientX: number,
  clientY: number,
  extents: GridExtents,
): Cell | null {
  const bounds = canvas.getBoundingClientRect();
  if (bounds.width <= 0 || bounds.height <= 0) return null;
  const ndcX = ((clientX - bounds.left) / bounds.width) * 2 - 1;
  const ndcY = 1 - ((clientY - bounds.top) / bounds.height) * 2;
  const localX = (camera.left + camera.right) / 2 + (ndcX * (camera.right - camera.left)) / (2 * camera.zoom);
  const localY = (camera.bottom + camera.top) / 2 + (ndcY * (camera.top - camera.bottom)) / (2 * camera.zoom);
  const elements = camera.matrixWorld.elements;
  const origin = transformedPoint(elements, localX, localY);
  const direction = [-elements[8]!, -elements[9]!, -elements[10]!] as const;
  if (Math.abs(direction[1]) < Number.EPSILON) return null;
  const distance = -origin[1] / direction[1];
  if (distance < 0) return null;
  const cell = {
    i: Math.round(origin[0] + direction[0] * distance),
    j: Math.round(origin[2] + direction[2] * distance),
  };
  return cell.i >= 0 && cell.i < extents.i && cell.j >= 0 && cell.j < extents.j
    ? cell
    : null;
}
