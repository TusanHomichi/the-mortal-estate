import { describe, expect, it } from "vitest";
import { createTerrainSurface, SEA_HEIGHT } from "../src/terrainSurface";
import { passabilityFrom } from "../src/walk/layoutPassability";
import type { FeelSpace } from "../src/feelTypes";

function coast(): FeelSpace {
  return {
    grid_extents: {i: 7,j: 7},
    cells: Array.from({length:49},(_,n) => ({i:n%7,j:Math.floor(n/7),material:n%7<3?"water":"meadow",walkable:n%7>=3})),
    structures:[], props:[], wall_runs:[],roofs:[], fixtures:[], portals:[], weather:true,
    light_sources:{lantern_glass:null,candles:[]},
  };
}

describe("presentation terrain contact", () => {
  it("puts dry cell centres above the sea and a sloping bank between them without changing collision", () => {
    const space=coast(), original=structuredClone(space), before=passabilityFrom(space);
    const surface=createTerrainSurface(space);
    expect(surface.heightAt(2,3)).toBeLessThan(SEA_HEIGHT);
    expect(surface.heightAt(3,3)).toBeGreaterThan(SEA_HEIGHT+.2);
    expect(surface.heightAt(2.5,3)).toBeGreaterThan(surface.heightAt(2,3));
    expect(surface.heightAt(2.5,3)).toBeLessThan(surface.heightAt(3,3));
    expect(space).toEqual(original);
    expect(passabilityFrom(space)).toEqual(before);
  });
  it("interpolates contact on the rendered triangle and stays continuous across cell boundaries", () => {
    const s=createTerrainSurface(coast());
    const a=s.heightAt(4,3),b=s.heightAt(4,3.125),c=s.heightAt(4.125,3.125);
    expect(s.heightAt(4.025,3.1)).toBeCloseTo(a*.2+b*.6+c*.2,10);
    expect(Math.abs(s.heightAt(3.5-1e-6,3)-s.heightAt(3.5+1e-6,3))).toBeLessThan(1e-5);
  });
  it("keeps foundations level, gives the dock its contact height, and retains flat interior floors", () => {
    const space=coast();
    space.cells.find(c=>c.i===4&&c.j===3)!.material="earth";
    space.cells.find(c=>c.i===2&&c.j===3)!.material="floor_planks";
    const s=createTerrainSurface(space);
    expect(s.heightAt(4,3)).toBe(0);
    expect(s.heightAt(2,3)).toBe(.04);
    space.weather=false;
    expect(createTerrainSurface(space).heightAt(2,3)).toBe(0);
  });
});
