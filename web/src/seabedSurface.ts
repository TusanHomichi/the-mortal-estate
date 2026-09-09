import type { TerrainSurface } from "./terrainSurface";

interface WaterMesh { positions: number[]; shore: number[]; indices: number[] }

/** Match the bank's finer lattice without refining the entire offshore sea.
 * Shared edge midpoints also split adjoining coarse triangles, so the transition
 * has no T junctions. The animated water geometry is never changed here.
 */
export function buildSeabedSurface(water: WaterMesh, surface: TerrainSurface): {
  positions: number[]; indices: number[];
} {
  const positions = [...water.positions], indices: number[] = [];
  const land: boolean[] = [];
  for (let i = 0; i < positions.length / 3; i++) {
    positions[i * 3 + 1] = water.shore[i * 2]! - .014;
    land.push(surface.sample(positions[i * 3]!, positions[i * 3 + 2]!, "water").land > 0);
  }
  const edgeKey = (a: number, b: number) => a < b ? `${a},${b}` : `${b},${a}`;
  const midpoints = new Map<string, number>();
  function vertex(x: number, z: number): number {
    const id = positions.length / 3;
    positions.push(x, surface.sample(x, z, "water").height - .014, z);
    return id;
  }
  function split(a: number, b: number): void {
    const key = edgeKey(a, b);
    if (!midpoints.has(key)) midpoints.set(key, vertex(
      (positions[a * 3]! + positions[b * 3]!) / 2,
      (positions[a * 3 + 2]! + positions[b * 3 + 2]!) / 2,
    ));
  }
  for (let i = 0; i < water.indices.length; i += 3) {
    const a = water.indices[i]!, b = water.indices[i + 1]!, c = water.indices[i + 2]!;
    if (land[a] || land[b] || land[c]) { split(a, b); split(b, c); split(c, a); }
  }
  for (let i = 0; i < water.indices.length; i += 3) {
    const a = water.indices[i]!, b = water.indices[i + 1]!, c = water.indices[i + 2]!;
    const ab = midpoints.get(edgeKey(a, b)), bc = midpoints.get(edgeKey(b, c)), ca = midpoints.get(edgeKey(c, a));
    if (ab !== undefined && bc !== undefined && ca !== undefined) {
      // Four triangles have exactly the terrain lattice's diagonal.
      indices.push(a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca);
    } else if (ab === undefined && bc === undefined && ca === undefined) {
      indices.push(a, b, c);
    } else {
      const centre = vertex((positions[a * 3]! + positions[b * 3]! + positions[c * 3]!) / 3,
        (positions[a * 3 + 2]! + positions[b * 3 + 2]! + positions[c * 3 + 2]!) / 3);
      const ring = [a, ab, b, bc, c, ca].filter((v): v is number => v !== undefined);
      for (let j = 0; j < ring.length; j++) indices.push(centre, ring[j]!, ring[(j + 1) % ring.length]!);
    }
  }
  return { positions, indices };
}
