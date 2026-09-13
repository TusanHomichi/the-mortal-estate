import {describe,it,expect,vi} from 'vitest';
import * as T from 'three';
import {SettlementScenery} from '../src/play/settlementScenery';
import {DungeonOcclusion} from '../src/play/dungeon/occlusion';
import {gridEdges} from '../src/play/worldGrid';
import {geography} from '../src/play/worldSpace';
import receipt from '../src/play/settlementReceipt.json';
import type {Frame} from '../src/authoritative/state';

describe('full 3D settlement geometry',()=>{
  it('places every building from its current authored footprint and entrance',()=>{
    const assets=new Map(Object.keys(receipt.models).map(name=>[name,new T.Group()]));
    const scenery=new SettlementScenery(assets),town=geography.members.find(m=>m.member==='arrival')!;
    scenery.presentStatic('arrival');expect(scenery.structures).toHaveLength(town.structures.length);
    for(const structure of town.structures){
      const transition=town.transitions.find(t=>t.access.x===structure.access.x&&t.access.y===structure.access.y)!;
      const model=scenery.group.getObjectByName(`town_${transition.target_member}`)!;
      expect(model.position.x/1.65).toBeCloseTo(structure.x+(structure.width-1)/2);
      expect(model.position.z/1.65).toBeCloseTo(structure.y+(structure.height-1)/2);
    }
    scenery.dispose();expect(scenery.group.children).toHaveLength(0);
  });
  it('reuses verified room geometry without disposing source assets on area changes',()=>{
    const model=new T.Group(),mesh=new T.Mesh(new T.BoxGeometry(),new T.MeshStandardMaterial());model.add(mesh);
    const dispose=vi.spyOn(mesh.geometry,'dispose'),scenery=new SettlementScenery(new Map([['room_temple',model]]));
    expect(scenery.presentStatic('temple')).toBe(true);expect(scenery.presentStatic('temple')).toBe(false);
    scenery.clear();expect(dispose).not.toHaveBeenCalled();scenery.dispose();mesh.geometry.dispose();mesh.material.dispose();
  });
  it('grounds every authored interior cell while leaving void open and floors out of occlusion',()=>{
    const assets=new Map(Object.keys(receipt.models).map(name=>[name,new T.Group()]));
    const scenery=new SettlementScenery(assets),matrix=new T.Matrix4(),point=new T.Vector3();
    for(const member of geography.members.filter(m=>assets.has(`room_${m.member}`))){
      scenery.presentStatic(member.member);
      const floor=scenery.group.getObjectByName('settlement-floor') as T.InstancedMesh;
      expect(floor).toBeInstanceOf(T.InstancedMesh);
      const covered=new Set<string>();
      for(let i=0;i<floor.count;i++){
        floor.getMatrixAt(i,matrix);point.setFromMatrixPosition(matrix);
        covered.add(`${Math.round(point.x/1.65)}:${Math.round(point.z/1.65)}`);
        expect(point.y).toBeLessThan(0);
      }
      const expected=member.cells.filter(c=>c.terrain.some(t=>t.layer==='base_terrain'&&t.class!=='expedition_void'));
      expect([...covered].sort()).toEqual(expected.map(c=>`${c.x}:${c.y}`).sort());
      expect(scenery.occluders.some(o=>o.getObjectById(floor.id))).toBe(false);
      const disposed=vi.spyOn(floor.geometry,'dispose');scenery.clear();expect(disposed).toHaveBeenCalledOnce();
    }
    scenery.dispose();
  });
  it('shares observed grid edges and excludes unknown and blocked squares',()=>{
    const tiles=[{position:{x:0,y:0},terrain_id:'floor',passable:true},{position:{x:1,y:0},terrain_id:'floor',passable:true},
      {position:{x:2,y:0},passable:true},{position:{x:3,y:0},terrain_id:'wall',passable:false}];
    const frame={tiles} as Frame;expect(gridEdges(frame)).toHaveLength(7);
    expect(gridEdges({tiles:[...tiles].reverse()} as Frame).map(e=>e.join(':')).sort()).toEqual(gridEdges(frame).map(e=>e.join(':')).sort());
  });
  it('fades a multi-material building for an extended limb and restores all source materials',()=>{
    const building=new T.Group(),original=[new T.MeshBasicMaterial(),new T.MeshBasicMaterial()];
    const mesh=new T.Mesh(new T.BoxGeometry(.5,2,.2),original);mesh.position.set(1,1,1);building.add(mesh);
    mesh.geometry.clearGroups();mesh.geometry.addGroup(0,18,0);mesh.geometry.addGroup(18,18,1);
    const body=new T.Group(),hand=new T.Bone();hand.name='RightHand';hand.position.set(1,1,0);body.add(hand);
    const camera=new T.PerspectiveCamera(40,1,.1,100);camera.position.set(1,1,5);camera.lookAt(1,1,0);camera.updateMatrixWorld(true);
    building.updateMatrixWorld(true);body.updateMatrixWorld(true);
    const occlusion=new DungeonOcclusion();occlusion.bind([building]);occlusion.update(camera,[body]);
    expect(occlusion.count).toBe(1);expect((mesh.material as T.Material[]).map(m=>m.opacity)).toEqual([.4,.4]);
    expect((mesh.material as T.Material[]).every(m=>m.depthFunc===T.EqualDepth)).toBe(true);
    const depth=mesh.getObjectByName('occlusion-depth') as T.Mesh;
    expect(depth.geometry).toBe(mesh.geometry);expect(depth.visible).toBe(true);expect(depth.renderOrder).toBeLessThan(mesh.renderOrder);
    occlusion.update(camera,[]);expect(mesh.material).toBe(original);occlusion.clear();
    expect(depth.visible).toBe(false);expect(depth.parent).toBeNull();expect(mesh.material).toBe(original);mesh.geometry.dispose();original.forEach(m=>m.dispose());
  });
});
