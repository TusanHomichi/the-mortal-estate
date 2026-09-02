import type { RoofPlacement } from "./feelTypes";
import type { GeometryData } from "./wallGeometry";

export const ROOF_OVERHANG = 0.15;
const RIDGE_HALF_WIDTH = 0.06;
const STRIP_HALF_HEIGHT = 0.04;
const EAVE_HALF_WIDTH = 0.055;
const BARGEBOARD_HALF_WIDTH = 0.045;

export type RoofMaterial =
  | "shingle_slope"
  | "shingle_ridge"
  | "shingle_eave"
  | "plaster"
  | "post";

export interface RoofGeometryPart {
  label: string;
  material: RoofMaterial;
  geometry: GeometryData;
}

export function mergeGeometryData(
  geometries: readonly GeometryData[],
): GeometryData {
  const merged = emptyGeometry();
  for (const geometry of geometries) {
    const vertexOffset = merged.positions.length / 3;
    merged.positions.push(...geometry.positions);
    merged.uvs.push(...geometry.uvs);
    merged.indices.push(...geometry.indices.map((index) => index + vertexOffset));
  }
  return merged;
}

type Vertex = [number, number, number];

function emptyGeometry(): GeometryData {
  return { positions: [], uvs: [], indices: [] };
}

function pushQuad(
  geometry: GeometryData,
  vertices: readonly [Vertex, Vertex, Vertex, Vertex],
  uLength: number,
  vLength: number,
): void {
  const offset = geometry.positions.length / 3;
  for (const vertex of vertices) geometry.positions.push(...vertex);
  geometry.uvs.push(0, 0, uLength, 0, uLength, vLength, 0, vLength);
  geometry.indices.push(offset, offset + 1, offset + 2, offset, offset + 2, offset + 3);
}

function pushTriangle(
  geometry: GeometryData,
  vertices: readonly [Vertex, Vertex, Vertex],
  uLength: number,
  vLength: number,
): void {
  const offset = geometry.positions.length / 3;
  for (const vertex of vertices) geometry.positions.push(...vertex);
  geometry.uvs.push(0, 0, uLength, 0, uLength / 2, vLength);
  geometry.indices.push(offset, offset + 1, offset + 2);
}

function cuboid(
  a0: number,
  a1: number,
  b0: number,
  b1: number,
  y0: number,
  y1: number,
  toWorld: (a: number, y: number, b: number) => Vertex,
): GeometryData {
  const result = emptyGeometry();
  const length = a1 - a0;
  const width = b1 - b0;
  pushQuad(result, [toWorld(a0, y0, b1), toWorld(a1, y0, b1), toWorld(a1, y1, b1), toWorld(a0, y1, b1)], length, y1 - y0);
  pushQuad(result, [toWorld(a1, y0, b0), toWorld(a0, y0, b0), toWorld(a0, y1, b0), toWorld(a1, y1, b0)], length, y1 - y0);
  pushQuad(result, [toWorld(a1, y0, b1), toWorld(a1, y0, b0), toWorld(a1, y1, b0), toWorld(a1, y1, b1)], width, y1 - y0);
  pushQuad(result, [toWorld(a0, y0, b0), toWorld(a0, y0, b1), toWorld(a0, y1, b1), toWorld(a0, y1, b0)], width, y1 - y0);
  pushQuad(result, [toWorld(a0, y1, b1), toWorld(a1, y1, b1), toWorld(a1, y1, b0), toWorld(a0, y1, b0)], length, width);
  pushQuad(result, [toWorld(a0, y0, b0), toWorld(a1, y0, b0), toWorld(a1, y0, b1), toWorld(a0, y0, b1)], length, width);
  return result;
}

function rakeStrip(
  a: number,
  b0: number,
  y0: number,
  b1: number,
  y1: number,
  toWorld: (a: number, y: number, b: number) => Vertex,
): GeometryData {
  const result = emptyGeometry();
  const dy = y1 - y0;
  const db = b1 - b0;
  const length = Math.hypot(dy, db);
  const offsetY = (-db / length) * BARGEBOARD_HALF_WIDTH;
  const offsetB = (dy / length) * BARGEBOARD_HALF_WIDTH;
  pushQuad(
    result,
    [
      toWorld(a, y0 + offsetY, b0 + offsetB),
      toWorld(a, y1 + offsetY, b1 + offsetB),
      toWorld(a, y1 - offsetY, b1 - offsetB),
      toWorld(a, y0 - offsetY, b0 - offsetB),
    ],
    length,
    BARGEBOARD_HALF_WIDTH * 2,
  );
  return result;
}

export function buildRoofGeometry(roof: RoofPlacement): RoofGeometryPart[] {
  const { footprint } = roof;
  const x0 = footprint.i0 - 0.5 - ROOF_OVERHANG;
  const x1 = footprint.i1 + 0.5 + ROOF_OVERHANG;
  const z0 = footprint.j0 - 0.5 - ROOF_OVERHANG;
  const z1 = footprint.j1 + 0.5 + ROOF_OVERHANG;
  const along0 = roof.ridge_axis === "x" ? x0 : z0;
  const along1 = roof.ridge_axis === "x" ? x1 : z1;
  const across0 = roof.ridge_axis === "x" ? z0 : x0;
  const across1 = roof.ridge_axis === "x" ? z1 : x1;
  const ridge = (across0 + across1) / 2;
  const toWorld = roof.ridge_axis === "x"
    ? (a: number, y: number, b: number): Vertex => [a, y, b]
    : (a: number, y: number, b: number): Vertex => [b, y, a];
  const alongLength = along1 - along0;
  const slopeLength = Math.hypot(
    ridge - across0,
    roof.ridge_height - roof.eave_height,
  );

  const lowerSlope = emptyGeometry();
  pushQuad(
    lowerSlope,
    [
      toWorld(along0, roof.eave_height, across0),
      toWorld(along1, roof.eave_height, across0),
      toWorld(along1, roof.ridge_height, ridge),
      toWorld(along0, roof.ridge_height, ridge),
    ],
    alongLength,
    slopeLength,
  );
  const upperSlope = emptyGeometry();
  pushQuad(
    upperSlope,
    [
      toWorld(along0, roof.ridge_height, ridge),
      toWorld(along1, roof.ridge_height, ridge),
      toWorld(along1, roof.eave_height, across1),
      toWorld(along0, roof.eave_height, across1),
    ],
    alongLength,
    slopeLength,
  );

  const gableStart = emptyGeometry();
  pushTriangle(
    gableStart,
    [
      toWorld(along0, roof.eave_height, across1),
      toWorld(along0, roof.eave_height, across0),
      toWorld(along0, roof.ridge_height, ridge),
    ],
    across1 - across0,
    roof.ridge_height - roof.eave_height,
  );
  const gableEnd = emptyGeometry();
  pushTriangle(
    gableEnd,
    [
      toWorld(along1, roof.eave_height, across0),
      toWorld(along1, roof.eave_height, across1),
      toWorld(along1, roof.ridge_height, ridge),
    ],
    across1 - across0,
    roof.ridge_height - roof.eave_height,
  );

  const parts: RoofGeometryPart[] = [
    { label: "slope-a", material: "shingle_slope", geometry: lowerSlope },
    { label: "slope-b", material: "shingle_slope", geometry: upperSlope },
    {
      label: "ridge",
      material: "shingle_ridge",
      geometry: cuboid(
        along0,
        along1,
        ridge - RIDGE_HALF_WIDTH,
        ridge + RIDGE_HALF_WIDTH,
        roof.ridge_height - STRIP_HALF_HEIGHT,
        roof.ridge_height + STRIP_HALF_HEIGHT,
        toWorld,
      ),
    },
    {
      label: "eave-a",
      material: "shingle_eave",
      geometry: cuboid(
        along0,
        along1,
        across0 - EAVE_HALF_WIDTH,
        across0 + EAVE_HALF_WIDTH,
        roof.eave_height - STRIP_HALF_HEIGHT,
        roof.eave_height + STRIP_HALF_HEIGHT,
        toWorld,
      ),
    },
    {
      label: "eave-b",
      material: "shingle_eave",
      geometry: cuboid(
        along0,
        along1,
        across1 - EAVE_HALF_WIDTH,
        across1 + EAVE_HALF_WIDTH,
        roof.eave_height - STRIP_HALF_HEIGHT,
        roof.eave_height + STRIP_HALF_HEIGHT,
        toWorld,
      ),
    },
    { label: "gable-start", material: "plaster", geometry: gableStart },
    { label: "gable-end", material: "plaster", geometry: gableEnd },
  ];
  for (const [endLabel, along] of [["start", along0], ["end", along1]] as const) {
    parts.push(
      {
        label: `barge-${endLabel}-a`,
        material: "post",
        geometry: rakeStrip(
          along,
          across0,
          roof.eave_height,
          ridge,
          roof.ridge_height,
          toWorld,
        ),
      },
      {
        label: `barge-${endLabel}-b`,
        material: "post",
        geometry: rakeStrip(
          along,
          ridge,
          roof.ridge_height,
          across1,
          roof.eave_height,
          toWorld,
        ),
      },
    );
  }
  return parts;
}
