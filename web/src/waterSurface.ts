import type { FeelSpace } from "./feelTypes";
import type { TerrainSurface } from "./terrainSurface";
import { sampleCoastalProfile } from "./coastalProfile";

/** Dense near the authored land, coarse offshore; one connected mesh. */
function axisKnots(extent: number): number[] {
  const values=[-200,-60,-24,-12];
  for(let v=-8;v<=extent+8;v+=.25) values.push(v);
  values.push(extent+12,extent+24,extent+60,extent+200);
  return values;
}
export function buildWaterSurface(space: FeelSpace, surface: TerrainSurface): {
  positions: number[]; shore: number[]; exposure: number[]; indices: number[];
} {
  const material=new Map(space.cells.map(c=>[`${c.i},${c.j}`,c.material]));
  const dry=space.cells.filter(c=>c.material!=="water"&&c.material!=="floor_planks");
  const coast=dry.filter(c=>[[1,0],[-1,0],[0,1],[0,-1]].some(([x,z])=>{
    const m=material.get(`${c.i+x!},${c.j+z!}`);return m===undefined||m==="water"||m==="floor_planks";
  }));
  const xs=axisKnots(space.grid_extents.i),zs=axisKnots(space.grid_extents.j);
  const positions:number[]=[],shore:number[]=[],exposure:number[]=[],indices:number[]=[];
  for(const z of zs) for(const x of xs){
    let distance=200;
    for(const c of coast) distance=Math.min(distance,Math.hypot(x-c.i,z-c.j));
    positions.push(x,0,z);
    // Water samples the bank underneath a deck, never the deck contact datum.
    shore.push(surface.sample(x,z,"water").height,distance);
    exposure.push(sampleCoastalProfile(space.coastal_profile,x,z,distance).exposure);
  }
  for(let z=0;z<zs.length-1;z++) for(let x=0;x<xs.length-1;x++){
    const a=z*xs.length+x,b=a+xs.length;
    indices.push(a,b,b+1,a,b+1,a+1);
  }
  return {positions,shore,exposure,indices};
}
