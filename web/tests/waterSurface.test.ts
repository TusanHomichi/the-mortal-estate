import { describe, expect, it } from "vitest";
import { buildWaterSurface } from "../src/waterSurface";
import { createTerrainSurface } from "../src/terrainSurface";
import type { FeelSpace } from "../src/feelTypes";

describe("coastal water geometry",()=>{
  it("uses the actual bank beneath decks, preserves movement, and stays one bounded indexed mesh",()=>{
    const space:FeelSpace={grid_extents:{i:3,j:2},cells:Array.from({length:6},(_,n)=>({i:n%3,j:Math.floor(n/3),material:n%3===0?"grass":n%3===1?"floor_planks":"water",walkable:n%3<2})),
      weather:true,structures:[],props:[],fixtures:[],wall_runs:[],roofs:[],portals:[],light_sources:{lantern_glass:null,candles:[]}};
    const original=structuredClone(space),surface=createTerrainSurface(space),mesh=buildWaterSurface(space,surface);
    expect(space).toEqual(original);
    expect(mesh.positions.every(Number.isFinite)).toBe(true);
    expect(mesh.shore.every(Number.isFinite)).toBe(true);
    expect(Math.max(...mesh.indices)).toBeLessThan(mesh.positions.length/3);
    const index=mesh.positions.findIndex((x,n)=>n%3===0&&x===1&&mesh.positions[n+2]===0)/3;
    expect(index).toBeGreaterThanOrEqual(0);
    expect(surface.heightAt(1,0)).toBe(.04);
    expect(mesh.shore[index*2]).toBeLessThan(0);
    expect(mesh.positions.length/3).toBeLessThan(10000);
  });
});
