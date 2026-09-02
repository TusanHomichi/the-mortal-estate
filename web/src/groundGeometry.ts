import type { CellPlan } from "./feelTypes";

export interface GroundGeometryData {
  positions: number[];
  uvs: number[];
  cellOrigins: number[];
  indices: number[];
}

export function buildGroundGeometry(cells: readonly CellPlan[]): GroundGeometryData {
  const positions: number[] = [];
  const uvs: number[] = [];
  const cellOrigins: number[] = [];
  const indices: number[] = [];

  for (const cell of cells) {
    const vertex = positions.length / 3;
    positions.push(
      cell.i - 0.5, -0.006, cell.j - 0.5,
      cell.i - 0.5, -0.006, cell.j + 0.5,
      cell.i + 0.5, -0.006, cell.j + 0.5,
      cell.i + 0.5, -0.006, cell.j - 0.5,
    );
    uvs.push(0, 0, 0, 1, 1, 1, 1, 0);
    for (let corner = 0; corner < 4; corner += 1) {
      cellOrigins.push(cell.i, cell.j);
    }
    indices.push(vertex, vertex + 1, vertex + 2, vertex, vertex + 2, vertex + 3);
  }

  return { positions, uvs, cellOrigins, indices };
}
