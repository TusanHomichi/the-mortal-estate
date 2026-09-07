import type { FeelSpace } from "./feelTypes";

export const SEA_HEIGHT = -0.24;
const RESOLUTION = 8;
const smooth = (a: number, b: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};
export interface SurfaceSample { height: number; land: number; lane: number; meadow: number }
export interface TerrainSurface {
  coastal: boolean;
  sample(x: number, z: number, material?: string): SurfaceSample;
  heightAt(x: number, z: number): number;
}

/** Presentation only. Cells remain the sole movement/collision authority.
 * Fine-grid vertex heights and figure/vegetation contact share this field.
 */
export function createTerrainSurface(space: FeelSpace): TerrainSurface {
  const coastal = space.weather && space.cells.some(c => c.material === "water");
  const cells = new Map(space.cells.map(c => [`${c.i},${c.j}`, c]));
  const materialAt = (i: number, j: number): string => cells.get(`${i},${j}`)?.material ?? "water";
  function weights(x: number, z: number): Omit<SurfaceSample, "height"> {
    // A small continuous perturbation breaks perfectly straight tile edges.
    const px = x + Math.sin(z * 5.3 + x * 1.7) * 0.065;
    const pz = z + Math.sin(x * 4.7 - z * 2.1) * 0.065;
    const i = Math.floor(px), j = Math.floor(pz), fx = px - i, fz = pz - j;
    let land = 0, lane = 0, meadow = 0;
    for (let dz = 0; dz < 2; dz++) for (let dx = 0; dx < 2; dx++) {
      const w = (dx ? fx : 1 - fx) * (dz ? fz : 1 - fz);
      const m = materialAt(i + dx, j + dz);
      if (m !== "water" && m !== "floor_planks") land += w;
      if (m === "lane" || m === "earth") lane += w;
      if (m === "meadow") meadow += w;
    }
    return { land, lane, meadow };
  }
  function vertex(x: number, z: number): SurfaceSample {
    const w = weights(x, z);
    if (!coastal) return { ...w, height: 0 };
    const material = materialAt(Math.round(x), Math.round(z));
    // Foundations and their apron stay level; paths settle to the same datum.
    const apron = material === "earth";
    const bank = -.43 * (1 - smooth(.22, .86, w.land));
    const rolling = (.07 + .045 * Math.sin(x * .7) * Math.cos(z * .51)) * w.meadow;
    const height = apron ? 0 : bank + rolling * (1 - smooth(.05, .55, w.lane)) * smooth(.7, 1, w.land);
    return { ...w, height };
  }
  // Cache the same lattice used to triangulate the ground. Barycentric contact
  // matches the rendered triangles, including their diagonal, not a second hill formula.
  const cache = new Map<string, SurfaceSample>();
  function at(i: number, j: number): SurfaceSample {
    const key = `${i},${j}`;
    let value = cache.get(key);
    if (!value) { value = vertex(i / RESOLUTION, j / RESOLUTION); cache.set(key, value); }
    return value;
  }
  function sample(x: number, z: number, material = materialAt(Math.round(x), Math.round(z))): SurfaceSample {
    if (!coastal) return { ...weights(x, z), height: 0 };
    // A deck is a level surface above the bank, with its own sharp edge.
    // Explicit material context keeps adjacent seabed triangles below it.
    if (material === "floor_planks") return { ...weights(x, z), height: .04 };
    const sx = x * RESOLUTION, sz = z * RESOLUTION;
    const i = Math.floor(sx), j = Math.floor(sz), u = sx - i, v = sz - j;
    const a = at(i, j), b = at(i, j + 1), c = at(i + 1, j + 1), d = at(i + 1, j);
    const mix = (key: keyof SurfaceSample): number => v >= u
      ? a[key] * (1 - v) + b[key] * (v - u) + c[key] * u
      : a[key] * (1 - u) + c[key] * v + d[key] * (u - v);
    return { height: mix("height"), land: mix("land"), lane: mix("lane"), meadow: mix("meadow") };
  }
  return { coastal, sample, heightAt: (x, z) => sample(x, z).height };
}

export const TERRAIN_SUBDIVISIONS = RESOLUTION;
