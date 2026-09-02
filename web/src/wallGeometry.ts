import type { WallAxis, WallRun } from "./feelTypes";

export const WALL_PROFILE = Object.freeze({
  thickness: 0.22,
  plinthTop: 0.3,
  sillTop: 0.42,
  capBottom: 1.98,
  capTop: 2.2,
  postWidth: 0.11,
  cornerPostWidth: 0.11 * 1.3,
  doorWidth: 0.7,
  doorHeight: 1.6,
  lintelTop: 1.74,
  doorLintelInset: 0.07,
});

export type WallMaterial =
  | "plinth"
  | "plaster"
  | "sill"
  | "post"
  | "door"
  | "cap_front"
  | "cap_top";

export interface GeometryData {
  positions: number[];
  uvs: number[];
  indices: number[];
}

export interface WallGeometryPart {
  label: string;
  material: WallMaterial;
  geometry: GeometryData;
}

interface Bounds {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  minZ: number;
  maxZ: number;
}

function emptyGeometry(): GeometryData {
  return { positions: [], uvs: [], indices: [] };
}

function pushFace(
  result: GeometryData,
  vertices: readonly [number, number, number][],
  u0: number,
  u1: number,
  v0: number,
  v1: number,
): void {
  const offset = result.positions.length / 3;
  for (const vertex of vertices) result.positions.push(...vertex);
  result.uvs.push(u0, v0, u1, v0, u1, v1, u0, v1);
  result.indices.push(offset, offset + 1, offset + 2, offset, offset + 2, offset + 3);
}

function cuboid(
  bounds: Bounds,
  u0: number,
  u1: number,
  v0: number,
  v1: number,
  continuousAxis: WallAxis = "x",
): GeometryData {
  const result = emptyGeometry();
  const { minX: x0, maxX: x1, minY: y0, maxY: y1, minZ: z0, maxZ: z1 } = bounds;
  const xFaceU = continuousAxis === "x" ? [u0, u1] as const : [0, 1] as const;
  const zFaceU = continuousAxis === "z" ? [u0, u1] as const : [0, 1] as const;
  pushFace(result, [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], xFaceU[0], xFaceU[1], v0, v1);
  pushFace(result, [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]], xFaceU[0], xFaceU[1], v0, v1);
  pushFace(result, [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]], zFaceU[0], zFaceU[1], v0, v1);
  pushFace(result, [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], zFaceU[0], zFaceU[1], v0, v1);
  pushFace(result, [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], u0, u1, 0, 1);
  pushFace(result, [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], u0, u1, 0, 1);
  return result;
}

function runBounds(
  run: WallRun,
  along0: number,
  along1: number,
  y0: number,
  y1: number,
  thickness: number = WALL_PROFILE.thickness,
): Bounds {
  const [startX, startZ] = run.start;
  return run.axis === "x"
    ? {
        minX: startX + along0,
        maxX: startX + along1,
        minY: y0,
        maxY: y1,
        minZ: startZ - thickness,
        maxZ: startZ,
      }
    : {
        minX: startX - thickness,
        maxX: startX,
        minY: y0,
        maxY: y1,
        minZ: startZ + along0,
        maxZ: startZ + along1,
      };
}

function addRunCuboid(
  parts: WallGeometryPart[],
  run: WallRun,
  label: string,
  material: WallMaterial,
  along0: number,
  along1: number,
  y0: number,
  y1: number,
  thickness: number = WALL_PROFILE.thickness,
): void {
  if (along1 <= along0) return;
  parts.push({
    label,
    material,
    geometry: cuboid(
      runBounds(run, along0, along1, y0, y1, thickness),
      along0 / 4,
      along1 / 4,
      y0 / 2.2,
      y1 / 2.2,
      run.axis,
    ),
  });
}

function transformGeometry(
  geometry: GeometryData,
  transform: (x: number, y: number, z: number) => [number, number, number],
): GeometryData {
  const positions: number[] = [];
  for (let index = 0; index < geometry.positions.length; index += 3) {
    positions.push(
      ...transform(
        geometry.positions[index]!,
        geometry.positions[index + 1]!,
        geometry.positions[index + 2]!,
      ),
    );
  }
  return { positions, uvs: [...geometry.uvs], indices: [...geometry.indices] };
}

function addBrace(parts: WallGeometryPart[], run: WallRun, panel: number): void {
  const heightSpan = WALL_PROFILE.capBottom - WALL_PROFILE.sillTop - 0.12;
  const alongSpan = 0.76;
  const length = Math.hypot(alongSpan, heightSpan);
  const angle = Math.atan2(alongSpan, heightSpan);
  const local = cuboid(
    run.axis === "x"
      ? { minX: -0.065, maxX: 0.065, minY: -length / 2, maxY: length / 2, minZ: -0.016, maxZ: 0.016 }
      : { minX: -0.016, maxX: 0.016, minY: -length / 2, maxY: length / 2, minZ: -0.065, maxZ: 0.065 },
    0,
    1,
    0,
    1,
  );
  const centreAlong = panel + 0.5;
  const centreY = (WALL_PROFILE.sillTop + WALL_PROFILE.capBottom) / 2;
  const [startX, startZ] = run.start;
  const front = 0.018;
  const geometry = transformGeometry(local, (x, y, z) => {
    if (run.axis === "x") {
      const rotatedX = x * Math.cos(-angle) - y * Math.sin(-angle);
      const rotatedY = x * Math.sin(-angle) + y * Math.cos(-angle);
      return [startX + centreAlong + rotatedX, centreY + rotatedY, startZ + front + z];
    }
    const rotatedY = y * Math.cos(angle) - z * Math.sin(angle);
    const rotatedZ = y * Math.sin(angle) + z * Math.cos(angle);
    return [startX + front + x, centreY + rotatedY, startZ + centreAlong + rotatedZ];
  });
  parts.push({ label: `brace-${run.axis}-${panel}`, material: "post", geometry });
}

function panelIsDoor(run: WallRun, panel: number): boolean {
  if (run.door_interval === null) return false;
  const centre = panel + 0.5;
  return centre >= run.door_interval[0] && centre <= run.door_interval[1];
}

function addCapTop(parts: WallGeometryPart[], run: WallRun): void {
  const [startX, startZ] = run.start;
  const thickness = WALL_PROFILE.thickness;
  const y = WALL_PROFILE.capTop + 0.001;
  const geometry = emptyGeometry();
  if (run.axis === "x") {
    pushFace(
      geometry,
      [[startX, y, startZ], [startX + run.cells, y, startZ], [startX + run.cells, y, startZ - thickness], [startX, y, startZ - thickness]],
      0,
      run.cells / 4,
      0,
      1,
    );
  } else {
    pushFace(
      geometry,
      [[startX - thickness, y, startZ], [startX, y, startZ], [startX, y, startZ + run.cells], [startX - thickness, y, startZ + run.cells]],
      0,
      run.cells / 4,
      0,
      1,
    );
  }
  parts.push({ label: `cap-top-${run.axis}`, material: "cap_top", geometry });
}

function addDoor(parts: WallGeometryPart[], run: WallRun, interval: [number, number]): void {
  const [u0, u1] = interval;
  const centre = (u0 + u1) / 2;
  const [startX, startZ] = run.start;
  const front = 0.002;
  const geometry = emptyGeometry();
  if (run.axis === "x") {
    pushFace(
      geometry,
      [[startX + u0, 0, startZ + front], [startX + u1, 0, startZ + front], [startX + u1, WALL_PROFILE.doorHeight, startZ + front], [startX + u0, WALL_PROFILE.doorHeight, startZ + front]],
      0,
      1,
      0,
      1,
    );
  } else {
    pushFace(
      geometry,
      [[startX + front, 0, startZ + u1], [startX + front, 0, startZ + u0], [startX + front, WALL_PROFILE.doorHeight, startZ + u0], [startX + front, WALL_PROFILE.doorHeight, startZ + u1]],
      0,
      1,
      0,
      1,
    );
  }
  parts.push({ label: `door-${run.axis}`, material: "door", geometry });
  const lintel0 = Math.floor(centre) + WALL_PROFILE.doorLintelInset;
  const lintel1 = Math.floor(centre) + 1 - WALL_PROFILE.doorLintelInset;
  addRunCuboid(
    parts,
    run,
    `door-lintel-${run.axis}`,
    "cap_front",
    lintel0,
    lintel1,
    WALL_PROFILE.doorHeight,
    WALL_PROFILE.lintelTop,
    WALL_PROFILE.thickness + 0.025,
  );
}

function segmentsWithoutDoor(run: WallRun): [number, number][] {
  return run.door_interval === null
    ? [[0, run.cells]]
    : [
        [0, run.door_interval[0]],
        [run.door_interval[1], run.cells],
      ];
}

export function buildWallProfile(runs: readonly WallRun[]): WallGeometryPart[] {
  const parts: WallGeometryPart[] = [];
  for (const run of runs) {
    for (const [u0, u1] of segmentsWithoutDoor(run)) {
      addRunCuboid(parts, run, `plinth-${run.axis}-${u0}`, "plinth", u0, u1, 0, WALL_PROFILE.plinthTop);
      addRunCuboid(parts, run, `plaster-${run.axis}-${u0}`, "plaster", u0, u1, WALL_PROFILE.plinthTop, WALL_PROFILE.capBottom);
      addRunCuboid(parts, run, `sill-${run.axis}-${u0}`, "sill", u0, u1, WALL_PROFILE.plinthTop, WALL_PROFILE.sillTop, WALL_PROFILE.thickness + 0.014);
    }
    if (run.door_interval !== null) {
      addRunCuboid(parts, run, `plaster-over-door-${run.axis}`, "plaster", run.door_interval[0], run.door_interval[1], WALL_PROFILE.doorHeight, WALL_PROFILE.capBottom);
      addDoor(parts, run, run.door_interval);
    }
    addRunCuboid(parts, run, `cap-front-${run.axis}`, "cap_front", 0, run.cells, WALL_PROFILE.capBottom, WALL_PROFILE.capTop);
    addCapTop(parts, run);
    for (let boundary = 1; boundary <= run.cells; boundary += 1) {
      addRunCuboid(
        parts,
        run,
        `post-${run.axis}-${boundary}`,
        "post",
        boundary - WALL_PROFILE.postWidth / 2,
        boundary + WALL_PROFILE.postWidth / 2,
        WALL_PROFILE.sillTop,
        WALL_PROFILE.capBottom,
        WALL_PROFILE.thickness + 0.018,
      );
    }
    for (let panel = 0; panel < run.cells; panel += 1) {
      if (panel % 3 === 1 && !panelIsDoor(run, panel)) addBrace(parts, run, panel);
    }
  }
  const first = runs[0];
  if (first !== undefined) {
    const [x, z] = first.start;
    const width = WALL_PROFILE.cornerPostWidth;
    parts.push({
      label: "corner-post",
      material: "post",
      geometry: cuboid(
        {
          minX: x - width,
          maxX: x,
          minY: WALL_PROFILE.sillTop,
          maxY: WALL_PROFILE.capBottom,
          minZ: z - width,
          maxZ: z,
        },
        0,
        1,
        0,
        1,
      ),
    });
  }
  return parts;
}

export function wallRunPosition(axis: WallAxis, start: [number, number], along: number): [number, number] {
  return axis === "x" ? [start[0] + along, start[1]] : [start[0], start[1] + along];
}
