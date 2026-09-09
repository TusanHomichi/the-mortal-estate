import { expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { createInlandShore } from "../src/inlandShore";
import { createTerrainSurface, SEA_HEIGHT } from "../src/terrainSurface";
import { passabilityFrom } from "../src/walk/layoutPassability";

function pond(): FeelSpace {
  return { grid_extents: { i: 7, j: 7 }, weather: true,
    cells: Array.from({ length: 49 }, (_, n) => {
      const i = n % 7, j = Math.floor(n / 7), wet = i >= 2 && i <= 4 && j >= 2 && j <= 4;
      return { i, j, material: wet ? "water" : "grass", walkable: !wet };
    }), structures: [], props: [], fixtures: [], wall_runs: [], roofs: [], portals: [],
    light_sources: { lantern_glass: null, candles: [] } };
}

it("rounds an enclosed pond while preserving wet/dry standing centres and movement", () => {
  const space = pond(), original = structuredClone(space), mask = passabilityFrom(space);
  const surface = createTerrainSurface(space);
  for (const c of space.cells) {
    if (c.material === "water") expect(surface.heightAt(c.i, c.j)).toBeLessThan(SEA_HEIGHT);
    else expect(surface.heightAt(c.i, c.j)).toBeGreaterThan(SEA_HEIGHT);
  }
  // The old square's corner is now bank, while its side midpoint remains wet.
  expect(surface.heightAt(1.65, 1.65)).toBeGreaterThan(SEA_HEIGHT);
  expect(surface.heightAt(1.65, 3)).toBeLessThan(SEA_HEIGHT);
  expect(space).toEqual(original);
  expect(passabilityFrom(space)).toEqual(mask);
  expect(surface.heightAt(1.65, 3)).toBe(createTerrainSurface(space).heightAt(1.65, 3));
});

it("leaves open water alone, including a coast reached beneath a deck", () => {
  const space = pond();
  for (const c of space.cells) if (c.i === 3 && c.j < 2) { c.material = "floor_planks"; c.walkable = true; }
  const shore = createInlandShore(space);
  for (const c of space.cells) expect(shore(c.i, c.j)).toBeNull();
});

it("does not let overlapping contour bounds erase another pond or nearby open water", () => {
  const space = pond();
  for (const c of space.cells) c.material = "grass";
  for (const [i, j] of [[2, 2], [3, 3], [4, 2], [5, 2], [6, 2]]) {
    space.cells.find(c => c.i === i && c.j === j)!.material = "water";
  }
  const shore = createInlandShore(space);
  expect(shore(2, 2)).toBeLessThan(.1);
  expect(shore(3, 3)).toBeLessThan(.1);
  expect(shore(4, 2)).toBeNull();
});

it("keeps an island inside a pond dry and contours finite at diagonal touches", () => {
  const space = pond();
  space.cells.find(c => c.i === 3 && c.j === 3)!.material = "grass";
  space.cells.find(c => c.i === 2 && c.j === 2)!.material = "grass";
  const shore = createInlandShore(space);
  expect(shore(3, 3)).toBeGreaterThan(.9);
  for (let z = 1; z <= 5; z += .125) for (let x = 1; x <= 5; x += .125) {
    const land = shore(x, z);
    expect(land).not.toBeNull();
    expect(land).toBeGreaterThanOrEqual(0);
    expect(land).toBeLessThanOrEqual(1);
  }
});
