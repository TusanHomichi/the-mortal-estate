import {describe,it,expect} from 'vitest';
import {bindWorldSpace,geography} from '../src/play/worldSpace';
import promotion from '../../content/lands/first-expedition/promotion.json';
import type {Snapshot} from '../src/authoritative/state';

function snapshot(level:string,min={x:0,y:0}):Snapshot {
  const m=geography.members.find(m=>m.member===level)!;
  const bounds={min,max:{x:m.width-1,y:m.height-1}};
  return {envelope:{frame:{observation_center:{realm:'first_expedition',level,position:min}},
    static_scene_context:{visual_manifest_digest:promotion.master.sha256,site:{realm:'first_expedition',level},bounds,
      walkable_mask:m.cells.filter(c=>c.passable&&c.x>=min.x&&c.y>=min.y).map(c=>({x:c.x,y:c.y}))}}} as Snapshot;
}
describe('one authored world presentation boundary',()=>{
  it('binds every current town, room and dungeon member',()=>{
    for(const m of geography.members)expect(bindWorldSpace(snapshot(m.member))).toBe(m.member);
  });
  it('refuses stale art, a mismatched area, duplicate floors and changed bounds',()=>{
    const mutate=(fn:(context:any)=>void)=>{const s=snapshot('temple');fn(s.envelope.static_scene_context);expect(()=>bindWorldSpace(s)).toThrow();};
    mutate(c=>c.visual_manifest_digest='0'.repeat(64));mutate(c=>c.site.level='arrival');
    mutate(c=>c.walkable_mask[0]=c.walkable_mask[1]);mutate(c=>c.bounds.max.x++);
  });
  it('binds moving outdoor windows and refuses missing or outside floor rows',()=>{
    for(const x of [5,8]){
      const s=snapshot('arrival',{x,y:27});expect(bindWorldSpace(s)).toBe('arrival');
      const context=s.envelope.static_scene_context as {walkable_mask:{x:number;y:number}[]};
      const mask=[...context.walkable_mask];context.walkable_mask=mask.slice(1);expect(()=>bindWorldSpace(s)).toThrow(/floor mask/);
      context.walkable_mask=[...mask.slice(1),{x:100,y:27}];expect(()=>bindWorldSpace(s)).toThrow(/floor mask/);
    }
  });
});
