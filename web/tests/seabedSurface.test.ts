import { expect, it } from "vitest";
import { buildWaterSurface } from "../src/waterSurface";
import { buildSeabedSurface } from "../src/seabedSurface";
import { createTerrainSurface } from "../src/terrainSurface";
import type { FeelSpace } from "../src/feelTypes";

it("keeps the submerged bank below the ground and stitches refinement without cracks", () => {
  const space: FeelSpace = {
    grid_extents: { i: 3, j: 2 }, weather: true,
    cells: Array.from({ length: 6 }, (_, n) => ({ i: n % 3, j: Math.floor(n / 3),
      material: n % 3 === 0 ? "grass" : n % 3 === 1 ? "floor_planks" : "water", walkable: n % 3 < 2 })),
    structures: [], props: [], fixtures: [], wall_runs: [], roofs: [], portals: [],
    light_sources: { lantern_glass: null, candles: [] },
  };
  const surface = createTerrainSurface(space), water = buildWaterSurface(space, surface);
  const original = structuredClone(water), bottom = buildSeabedSurface(water, surface);
  expect(water).toEqual(original);
  expect(bottom.positions.every(Number.isFinite)).toBe(true);
  const edges = new Map<string, number>(), p = bottom.positions;
  const minX = Math.min(...p.filter((_, i) => i % 3 === 0)), maxX = Math.max(...p.filter((_, i) => i % 3 === 0));
  const minZ = Math.min(...p.filter((_, i) => i % 3 === 2)), maxZ = Math.max(...p.filter((_, i) => i % 3 === 2));
  let bankTriangles = 0;
  for (let i = 0; i < bottom.indices.length; i += 3) {
    const ids = bottom.indices.slice(i, i + 3);
    const x = ids.reduce((sum, v) => sum + p[v * 3]!, 0) / 3;
    const y = ids.reduce((sum, v) => sum + p[v * 3 + 1]!, 0) / 3;
    const z = ids.reduce((sum, v) => sum + p[v * 3 + 2]!, 0) / 3;
    const sample = surface.sample(x, z, "water");
    if (sample.land > .05 && sample.land < .95) {
      expect(y).toBeCloseTo(sample.height - .014, 8);
      bankTriangles++;
    }
    for (let j = 0; j < 3; j++) {
      const a = ids[j]!, b = ids[(j + 1) % 3]!, key = a < b ? `${a},${b}` : `${b},${a}`;
      edges.set(key, (edges.get(key) ?? 0) + 1);
    }
  }
  expect(bankTriangles).toBeGreaterThan(0);
  for (const [key, count] of edges) {
    const [a, b] = key.split(",").map(Number) as [number, number];
    const boundary = [minX, maxX].some(x => p[a * 3] === x && p[b * 3] === x)
      || [minZ, maxZ].some(z => p[a * 3 + 2] === z && p[b * 3 + 2] === z);
    expect(count).toBe(boundary ? 1 : 2);
  }
});
