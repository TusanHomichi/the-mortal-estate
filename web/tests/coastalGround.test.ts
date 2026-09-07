import { createFeelCamera } from "../src/camera";
import { describe, expect, it } from "vitest";
import { Group, Mesh, ShaderMaterial, Texture } from "three";
import { addCoastalGround } from "../src/space/coastalGround";
import { createTerrainSurface } from "../src/terrainSurface";
import { paletteFor } from "../src/space/palette";
import type { FeelSpace } from "../src/feelTypes";
import type { SpaceSceneOptions } from "../src/space/SpaceScene";
import type { DecodedTexture } from "../src/space/textures";

describe("coastal material ownership", () => {
  it("renders authored stone and plank cells without a model or an unused lane asset", () => {
    const materials=["water","grass","stone","floor_planks"];
    const space: FeelSpace = { grid_extents:{i:4,j:1}, cells:materials.map((material,i)=>({i,j:0,material,walkable:i>0})),
      structures:[],props:[],wall_runs:[],roofs:[],fixtures:[],portals:[],weather:true,light_sources:{lantern_glass:null,candles:[]} };
    const textures=new Map<string,DecodedTexture>();
    for (const name of [...materials,"earth"]) textures.set(`terrain/${name}`,{texture:new Texture(),width:1,height:1,pixels:null});
    const options={space,textures,presets:[],anisotropy:1,camera:createFeelCamera(1280,800,{i:0,j:0})} as unknown as SpaceSceneOptions;
    const group=new Group(), surface=createTerrainSurface(space);
    addCoastalGround(group,options,paletteFor([],true),{value:0},surface);
    for (const name of ["stone","floor_planks"]) {
      const mesh=group.getObjectByName(`Ground_${name}`) as Mesh;
      expect(mesh).toBeInstanceOf(Mesh);
      expect((mesh.material as ShaderMaterial).uniforms.swatch!.value).toBe(textures.get(`terrain/${name}`)!.texture);
      const position=mesh.geometry.getAttribute("position");
      for (let n=0;n<position.count;n++) expect(position.getY(n)).toBeCloseTo(surface.sample(position.getX(n),position.getZ(n),name).height-.006,6);
    }
    const deck = (group.getObjectByName("Ground_floor_planks") as Mesh).geometry.getAttribute("position");
    for (let n=0;n<deck.count;n++) expect(deck.getY(n)).toBeCloseTo(.034,6);
    expect(group.getObjectByName("SeaBackdrop")).toBeInstanceOf(Mesh);
    expect((group.getObjectByName("Ground_coast") as Mesh).material).toBeInstanceOf(ShaderMaterial);
  });
});
