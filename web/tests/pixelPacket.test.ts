import { describe, it, expect } from "vitest";
import promotion from "../../content/lands/first-expedition/promotion.json";
import geography from "../../content/lands/first-expedition/generated/workbench_projection.json";
import { validatePixelManifest, bindPixelSpace, type PixelManifest } from "../src/play/pixelPacket";
import { projectPixel, unprojectPixel, TEMPLE_PROJECTION, pixelDirection, pixelCellBounds, pixelGridEdges, pixelSpriteRect } from "../src/play/pixelGeometry";
import type { Snapshot } from "../src/authoritative/state";

function manifest(): PixelManifest {
  const file = { file:"synthetic.png",sha256:"a".repeat(64),body_height:92,foot:{x:48,y:94} };
  const rotations = Object.fromEntries(["north","east","south","west"].map(direction=>[direction,file]));
  const walk = Object.fromEntries(Object.keys(rotations).map(direction=>[direction,[file]]));
  const projection={origin:{x:0,y:0},step:{x:32,y:24}};
  const entrances=geography.members.find(m=>m.member==="arrival")!.transitions.map(edge=>({transition:edge.id,bounds:{x:edge.access.x*32-8,y:edge.access.y*24-8,width:16,height:16}}));
  return {schema_version:4,geography_master_sha256:promotion.master.sha256,status:"candidate",geography_review_manifest_sha256:promotion.reviewed_encoding.review_manifest_sha256,
    exterior:{background:file,width:1024,height:1024,projection,entrances,foreground:[{...file,id:"temple",x:0,y:0,depth:100}]},
    temple_foreground:[{x:20,y:20,width:50,height:50,depth:70}],
    effects:{scenes:{arrival:{material:file,normals:file,lights:[],shadows:[]},temple:{material:file,normals:file,lights:[],shadows:[]}},
      profiles:{day:{ambient:[1,1,1],fog:0,rain:0,wind:1,light:1,grade:file}},default_profile:"day"},
    background:file,figures:Object.fromEntries(["traveler","tomas","maude"].map(name=>[name,{rotations,walk,size:96}])),
    actor_figures:{tomas:"tomas",balm_seller:"maude"}};
}
function snapshot(): Snapshot {
  const member=geography.members.find(member=>member.member==="temple")!;
  return {envelope:{frame:{observation_center:{realm:"first_expedition",level:"temple",position:{x:3,y:6}}},
    static_scene_context:{visual_manifest_digest:promotion.master.sha256,site:{realm:"first_expedition",level:"temple"},
      bounds:{min:{x:0,y:0},max:{x:6,y:7}},walkable_mask:member.cells.filter(c=>c.passable).map(c=>({x:c.x,y:c.y}))}}} as Snapshot;
}
describe("pixel study boundaries",()=>{
  it("keeps feet on the same room point across differently padded poses",()=>{
    const point=projectPixel({x:3,y:4},TEMPLE_PROJECTION);
    for (const frame of [{body_height:108,foot:{x:48,y:111}},{body_height:101,foot:{x:65,y:114}},
      {body_height:143,foot:{x:90,y:164}}]) {
      for (const height of [72,96]) {
        const rect=pixelSpriteRect(point,frame,180,height);
        expect(Math.abs(rect.x+frame.foot.x*rect.scale-point.x)).toBeLessThanOrEqual(.5);
        expect(Math.abs(rect.y+frame.foot.y*rect.scale-point.y)).toBeLessThanOrEqual(.5);
        expect(frame.body_height*rect.scale).toBeCloseTo(height);
      }
    }
  });
  it("draws shared walkable boundaries once and switches picking on the same boundary",()=>{
    const projection={origin:{x:0,y:0},step:{x:64,y:44}};
    const tiles=[{position:{x:0,y:0},passable:true},{position:{x:1,y:0},passable:true},
      {position:{x:2,y:0},passable:false},{position:{x:0,y:1}}];
    const edges=pixelGridEdges(tiles,projection);
    expect(edges).toHaveLength(7);
    expect(pixelGridEdges(tiles.slice(2),projection)).toEqual([]);
    const bounds=pixelCellBounds({x:0,y:0},projection);
    expect(unprojectPixel({x:bounds.right-.01,y:0},projection)).toEqual({x:0,y:0});
    expect(unprojectPixel({x:bounds.right,y:0},projection)).toEqual({x:1,y:0});
    expect(unprojectPixel({x:0,y:bounds.bottom-.01},projection)).toEqual({x:0,y:0});
    expect(unprojectPixel({x:0,y:bounds.bottom},projection)).toEqual({x:0,y:1});
  });
  it("preserves shared room edges across observers and visibility subsets",()=>{
    const tiles=[{position:{x:2,y:3},passable:true},{position:{x:3,y:3},passable:true},
      {position:{x:4,y:3},passable:true}];
    const normalized=(rows:typeof tiles)=>pixelGridEdges(rows,TEMPLE_PROJECTION).map(edge=>JSON.stringify(edge)).sort();
    expect(normalized([...tiles].reverse())).toEqual(normalized(tiles));
    const firstObserver=normalized(tiles.slice(0,2)),secondObserver=normalized(tiles.slice(1));
    for (const sharedEdge of normalized([tiles[1]!])) {
      expect(firstObserver).toContain(sharedEdge);
      expect(secondObserver).toContain(sharedEdge);
    }
    // Drawing a player, resident or creature never contributes grid coordinates.
    const sharedSquare={x:3,y:3};
    expect(projectPixel(sharedSquare,TEMPLE_PROJECTION)).toEqual({x:265,y:242});
  });
  it("refuses stale geography, external image URLs, absent directions and invalid anchors",()=>{
    expect(()=>validatePixelManifest(manifest())).not.toThrow();
    const stale=manifest();stale.geography_review_manifest_sha256="0".repeat(64);
    expect(()=>validatePixelManifest(stale)).toThrow();
    const external=manifest();external.background={file:"https://example.com/art.png",sha256:"a".repeat(64)};
    expect(()=>validatePixelManifest(external)).toThrow();
    const missing=manifest();delete missing.figures.traveler!.walk.north;
    expect(()=>validatePixelManifest(missing)).toThrow();
    const partialDiagonal=manifest();partialDiagonal.figures.traveler!.walk["north-east"]=partialDiagonal.figures.traveler!.walk.north!;
    expect(()=>validatePixelManifest(partialDiagonal)).toThrow();
    const anchor=manifest();anchor.figures.traveler!.rotations.south!.foot.y=200;
    expect(()=>validatePixelManifest(anchor)).toThrow();
    const padding=manifest();padding.figures.traveler!.rotations.south!.body_height=128;
    expect(()=>validatePixelManifest(padding)).toThrow();
    const retired=manifest();retired.schema_version=1;
    expect(()=>validatePixelManifest(retired)).toThrow();
    retired.schema_version=2;expect(()=>validatePixelManifest(retired)).toThrow();
    retired.schema_version=3;expect(()=>validatePixelManifest(retired)).toThrow();
    const badScenery=manifest();badScenery.exterior.foreground[0]!.x=-1;
    expect(()=>validatePixelManifest(badScenery)).toThrow(/calibration/);
    const shiftedDoor=manifest();shiftedDoor.exterior.entrances[0]!.bounds.x+=100;
    expect(()=>validatePixelManifest(shiftedDoor)).toThrow(/doorway/);
    const duplicateDoor=manifest();duplicateDoor.exterior.entrances[0]=duplicateDoor.exterior.entrances[1]!;
    expect(()=>validatePixelManifest(duplicateDoor)).toThrow(/entrance/);
    const missingDay=manifest();delete missingDay.effects.profiles.day;
    expect(()=>validatePixelManifest(missingDay)).toThrow(/effects/);
    const badLight=manifest();badLight.effects.scenes.arrival.lights=[{x:0,y:0,height:20,radius:Infinity,strength:1}];
    expect(()=>validatePixelManifest(badLight)).toThrow(/light/);
    const badPatch=manifest();badPatch.temple_foreground[0]!.width=513;
    expect(()=>validatePixelManifest(badPatch)).toThrow(/foreground/);
    const traversal=manifest();traversal.background={file:"../art.png",sha256:"a".repeat(64)};
    expect(()=>validatePixelManifest(traversal)).toThrow();
  });
  it("uses diagonal artwork only for figures with the complete diagonal gait",()=>{
    expect(pixelDirection({x:1,y:1},{x:2,y:2},true)).toBe("south-east");
    expect(pixelDirection({x:1,y:1},{x:0,y:0},true)).toBe("north-west");
    expect(pixelDirection({x:1,y:1},{x:2,y:2})).toBe("south");
    expect(pixelDirection({x:1,y:1},{x:2,y:1},true)).toBe("east");
  });
  it("rejects altered bounds, duplicate floor rows and another world's master",()=>{
    expect(bindPixelSpace(snapshot())).toBe("temple");
    const bad=snapshot();const context=bad.envelope.static_scene_context as {walkable_mask:{x:number;y:number}[];visual_manifest_digest:string;bounds:{max:{x:number}}};
    context.walkable_mask[0]={...context.walkable_mask[1]!};expect(()=>bindPixelSpace(bad)).toThrow();
    context.visual_manifest_digest="0".repeat(64);expect(()=>bindPixelSpace(bad)).toThrow();
    const shifted=snapshot();(shifted.envelope.static_scene_context as typeof context).bounds.max.x=7;
    expect(()=>bindPixelSpace(shifted)).toThrow();
  });
  it("binds moving exterior windows and refuses missing or out-of-window floor rows",()=>{
    const member=geography.members.find(member=>member.member==="arrival")!;
    for(const x of [5,8]) {
      const bounds={min:{x,y:27},max:{x:x+8,y:35}};
      const mask=member.cells.filter(c=>c.passable&&c.x>=bounds.min.x&&c.x<=bounds.max.x&&
        c.y>=bounds.min.y&&c.y<=bounds.max.y).map(c=>({x:c.x,y:c.y}));
      expect(mask.length).toBeGreaterThan(1);
      const context={visual_manifest_digest:promotion.master.sha256,site:{realm:"first_expedition",level:"arrival"},
        bounds,walkable_mask:mask};
      const state={envelope:{frame:{observation_center:{...context.site,position:{x:9,y:31}}},
        static_scene_context:context}} as Snapshot;
      expect(bindPixelSpace(state)).toBe("arrival");
      context.walkable_mask=mask.slice(1);
      expect(()=>bindPixelSpace(state)).toThrow(/floor mask/);
      context.walkable_mask=[...mask.slice(1),{x:bounds.max.x+1,y:bounds.max.y}];
      expect(()=>bindPixelSpace(state)).toThrow(/floor mask/);
      context.walkable_mask=[...mask.slice(1),mask[1]!];
      expect(()=>bindPixelSpace(state)).toThrow(/floor mask/);
      context.walkable_mask=mask;context.bounds.min.x=-1;
      expect(()=>bindPixelSpace(state)).toThrow(/geography/);
    }
  });
  it("maps every authored square to one stable pixel target, including the stair and entrance edges",()=>{
    for(let x=0;x<7;x++)for(let y=0;y<8;y++){
      const p=projectPixel({x,y},TEMPLE_PROJECTION);
      expect(unprojectPixel(p,TEMPLE_PROJECTION)).toEqual({x,y});
      expect(unprojectPixel({x:p.x+30,y:p.y+20},TEMPLE_PROJECTION)).toEqual({x,y});
    }
  });
});
