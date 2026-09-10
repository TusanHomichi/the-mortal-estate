import {Bone,Skeleton,SkinnedMesh,Group,BoxGeometry,MeshBasicMaterial} from 'three';
import {disposeSkeletons} from '../src/play/dungeon/rigResources';
import {describe,it,expect,vi} from 'vitest';
import type {Frame} from '../src/authoritative/state';
import {observedDungeon,occupantAnchors,isDungeon} from '../src/play/dungeon/view';
import {wallParts} from '../src/play/dungeon/topology';
function frame():Frame {return {logical_time:'0',ready_at:'0',can_act:true,observer_actor_id:'p',observation_center:{realm:'first_expedition',level:'d1_entry',position:{x:24,y:9}},tiles:[],actors:[],corpses:[],ground_items:[],gold_piles:[]};}
describe('live dungeon presentation boundaries',()=>{
  it('routes all four floors and excludes town and service interiors',()=>{
    for(const level of ['d1_entry','d2','d3','d4'])expect(isDungeon(level)).toBe(true);
    for(const level of ['arrival','temple','bank','d5'])expect(isDungeon(level)).toBe(false);
  });
  it('removes unseen rows and occupants without changing the authoritative frame',()=>{
    const f=frame(),pos=f.observation_center;
    f.tiles=[{position:pos.position,terrain_id:'expedition_floor',passable:true},{position:{x:25,y:9}},
      {position:{x:28,y:9},terrain_id:'expedition_floor',passable:true}];
    f.actors=[{actor_id:'p',name:'Player',position:pos},{actor_id:'hidden',name:'Hidden',position:{...pos,position:{x:25,y:9}}}];
    const before=JSON.stringify(f),view=observedDungeon(f);
    expect(view.tiles).toHaveLength(1);expect(view.actors.map(a=>a.actor_id)).toEqual(['p']);expect(JSON.stringify(f)).toBe(before);
  });
  it('keeps a lone actor centered and distributes a shared square deterministically',()=>{
    const f=frame();f.actors=[{actor_id:'p',name:'Player',position:f.observation_center}];
    expect(occupantAnchors(f).get('p')).toEqual(f.observation_center.position);
    f.actors.push({...f.actors[0]!,actor_id:'other'});
    const a=occupantAnchors(f);f.actors.reverse();expect(occupantAnchors(f)).toEqual(a);
    expect(a.get('p')).not.toEqual(a.get('other'));
  });
  it('keeps a side-door axis and hinge identity through visibility changes and opening',()=>{
    const f=frame();const door={position:{x:22,y:9},terrain_id:'expedition_doorway_closed',passable:true,transition:{navigation:'door',door_open:false}};
    f.tiles=[door];const closed=wallParts(f);expect(closed).toHaveLength(1);expect(closed[0]!.axis).toBe('y');
    f.observation_center.position={x:21,y:9};door.transition.door_open=true;
    const opened=wallParts(f);expect(opened[0]!.id).toBe(closed[0]!.id);expect(opened[0]!.axis).toBe('y');expect(opened[0]!.open).toBe(true);
  });
  it('centres a living player sharing a cell with a dead creature',()=>{
    const f=frame();f.tiles=[{position:f.observation_center.position,terrain_id:'expedition_floor',passable:true}];
    f.actors=[{actor_id:'p',name:'Player',position:f.observation_center,life_state:'alive'},
      {actor_id:'dead',name:'Dead creature',position:f.observation_center,life_state:'dead'}];
    expect(occupantAnchors(f).get('p')).toEqual(f.observation_center.position);
    expect(observedDungeon(f).actors.map(a=>a.actor_id)).toEqual(['p']);
  });
  it('releases a departing figure skeleton texture once without disposing shared artwork',()=>{
    const root=new Group(),skeleton=new Skeleton([new Bone()]),geometry=new BoxGeometry(),material=new MeshBasicMaterial();
    for(let i=0;i<2;i++){const mesh=new SkinnedMesh(geometry,material);mesh.bind(skeleton);root.add(mesh);}
    skeleton.computeBoneTexture();const dispose=vi.spyOn(skeleton.boneTexture!,'dispose');
    const geometryDispose=vi.spyOn(geometry,'dispose'),materialDispose=vi.spyOn(material,'dispose');
    disposeSkeletons(root);expect(dispose).toHaveBeenCalledTimes(1);expect(skeleton.boneTexture).toBeNull();
    expect(geometryDispose).not.toHaveBeenCalled();expect(materialDispose).not.toHaveBeenCalled();
    geometry.dispose();material.dispose();
  });
  it('does not turn impassable water into masonry',()=>{
    const f=frame();f.tiles=[{position:{x:24,y:9},terrain_id:'expedition_dungeon_water',passable:false}];
    expect(wallParts(f)).toEqual([]);
  });
});
