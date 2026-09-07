import { describe, expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { createTerrainSurface } from "../src/terrainSurface";
import { buildTacticalGridGeometry, createTacticalGrid } from "../src/space/tacticalGrid";

function space(): FeelSpace {
  return { weather: true, cells: [
    { i: 0, j: 0, material: "meadow", walkable: true },
    { i: 1, j: 0, material: "earth", walkable: false },
    { i: 2, j: 0, material: "water", walkable: false },
    { i: 3, j: 0, material: "void", walkable: false },
  ] } as FeelSpace;
}

describe("visible tactical grid", () => {
  it("covers authored standing surfaces regardless of legality and follows the shared relief", () => {
    const s = space(), surface = createTerrainSurface(s);
    const geometry = buildTacticalGridGeometry(s, surface);
    const p = geometry.getAttribute("position");
    expect(p.count).toBe(2 * 81);
    for (let n = 0; n < p.count; n++) {
      const cell = s.cells[n < 81 ? 0 : 1]!;
      expect(p.getY(n)).toBeCloseTo(surface.sample(p.getX(n), p.getZ(n), cell.material).height + .009, 6);
      expect(p.getX(n)).toBeLessThanOrEqual(1.5);
    }
    expect(s.cells[1]!.walkable).toBe(false);
    geometry.dispose();
  });
  it("keeps the resting grid when hover clears and never changes cell data", () => {
    const s = space(), before = JSON.stringify(s);
    const grid = createTacticalGrid(s, createTerrainSurface(s));
    grid.focus({ i: 1, j: 0 });
    expect(grid.mesh.material.uniforms.focus!.value.toArray()).toEqual([1, 0]);
    expect(grid.mesh.material.uniforms.focusEnabled!.value).toBe(1);
    grid.focus(null);
    expect(grid.mesh.material.uniforms.focusEnabled!.value).toBe(0);
    expect(grid.mesh.visible).toBe(true);
    expect(grid.mesh.material.depthTest).toBe(true);
    expect(JSON.stringify(s)).toBe(before);
    grid.mesh.geometry.dispose(); grid.mesh.material.dispose();
  });
});
