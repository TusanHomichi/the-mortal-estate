import {afterEach,describe,expect,it,vi} from 'vitest';
import * as T from 'three';
import {DungeonActors} from '../src/play/dungeon/actors';
import {COMBAT_CLIPS,type DungeonAssets,type FigureAsset} from '../src/play/dungeon/assets';
import type {Snapshot} from '../src/authoritative/state';

function asset(idle='guard'):FigureAsset {
  const scene=new T.Group(),bone=new T.Bone();bone.name='root';scene.add(bone);
  const clips=[...new Set([...COMBAT_CLIPS,idle])].map(name=>new T.AnimationClip(name,1,[
    new T.NumberKeyframeTrack('root.position[x]',[0,1],[0,.1]),
  ]));
  return {scene,clips,idle,stride:1};
}
function assets():DungeonAssets{return {fallback:asset('idle'),male:asset(),female:asset()};}
const place=(x:number)=>({realm:'test',level:'d1_entry',position:{x,y:0}});
function move(x:number){return {kind:'actor_moved',actor_id:'self',from:place(x),to:place(x+1),navigation:'walk'};}
function hit(mode='jumpkick',outcome='hit',source='self',target='target'){
  return {kind:'feedback',cue:{kind:'physical_combat',source:{actor_id:source},target:{actor_id:target},mode,outcome:{kind:outcome}}};
}
function snapshot(sequence:number,x:number,events:unknown[]=[],ready=false,sex='male',kind='state_update',targetX=3):Snapshot {
  return {generation:sequence,raw:'',envelope:{kind,server_sequence:String(sequence),world_revision:String(sequence),static_scene_context:{},events,frame:{
    logical_time:'1000',ready_at:ready?'1000':'4000',can_act:ready,observer_actor_id:'self',observation_center:place(x),
    actors:[{actor_id:'self',name:'Player',position:place(x),life_state:'alive'},
      {actor_id:'target',name:'Target',position:place(targetX),life_state:'alive'}],
    tiles:[0,1,2,3].map(x=>({position:{x,y:0},terrain_id:'floor'})),corpses:[],gold_piles:[],ground_items:[],
    character:{identity:{base_class_id:'martial_artist',sex_or_gender_display:sex}},carried:{items:[]},
  }}} as unknown as Snapshot;
}
function canvas(){vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>({fillText:()=>{}})})});}
function self(actors:DungeonActors){return actors.diagnostics().find(row=>(row as {id:string}).id==='self');}
function present(actors:DungeonActors,s:Snapshot){actors.present(s.envelope.frame,s);}
afterEach(()=>vi.unstubAllGlobals());

describe('closing jumpkick presentation',()=>{
  for(const sex of ['male','female'])for(const distance of [1,2,3])for(const outcome of ['hit','missed','blocked']){
    it(`${sex}: ${distance}-tile ${outcome} keeps the kick through travel and lands in guard`,()=>{
      canvas();let now=0;const actors=new DungeonActors(assets(),()=>now);
      present(actors,snapshot(1,0,[],false,sex,'state_update',distance));
      const events=[...Array.from({length:distance},(_,x)=>move(x)),hit('jumpkick',outcome)];
      const kicked=snapshot(2,distance,events,false,sex,'state_update',distance);present(actors,kicked);
      expect(self(actors)).toMatchObject({body:sex,clip:'flying_kick',moving:true});
      now=2000;actors.update(2);
      expect(self(actors)).toMatchObject({clip:'flying_kick',moving:true});
      expect(actors.anchor('self')!.x).toBeGreaterThan(0);
      expect(actors.anchor('self')!.x).toBeLessThan(distance+.5);
      present(actors,kicked); // Same receipt cannot restart the clip or the route.
      now=3001;actors.update(1.001);
      expect(self(actors)).toMatchObject({clip:'guard',moving:false});
      const landed=actors.anchor('self');present(actors,kicked);
      expect(actors.anchor('self')).toEqual(landed);
      expect(self(actors)).toMatchObject({clip:'guard',moving:false});actors.dispose();
    });
  }
  it('a later cover cue does not overwrite the moving kick',()=>{
    canvas();const actors=new DungeonActors(assets());present(actors,snapshot(1,0));
    present(actors,snapshot(2,3,[move(0),move(1),move(2),hit(),hit('fight','blocked','target','self')]));
    expect(self(actors)).toMatchObject({clip:'flying_kick',moving:true});actors.dispose();
  });
  it('readiness ends a still-playing kick and allows the next punch',()=>{
    canvas();let now=0;const actors=new DungeonActors(assets(),()=>now);present(actors,snapshot(1,0));
    present(actors,snapshot(2,3,[move(0),move(1),move(2),hit()]));now=400;actors.update(.4);
    present(actors,snapshot(3,3,[],true));expect(self(actors)).toMatchObject({clip:'guard',moving:false});
    present(actors,snapshot(4,3,[hit('fight')]));expect(self(actors)).toMatchObject({clip:'jab_left',moving:false});actors.dispose();
  });
  it('a short authoritative remainder bounds both travel and the kick',()=>{
    canvas();let now=0;const actors=new DungeonActors(assets(),()=>now);present(actors,snapshot(1,0));
    const s=snapshot(2,3,[move(0),move(1),move(2),hit()]);s.envelope.frame.ready_at='1100';present(actors,s);
    now=101;actors.update(.101);expect(self(actors)).toMatchObject({clip:'guard',moving:false});actors.dispose();
  });
  it('partial or missing routes never manufacture approach travel',()=>{
    for(const events of [[move(2),hit()],[hit()]]){
      canvas();const actors=new DungeonActors(assets());present(actors,snapshot(1,0));present(actors,snapshot(2,3,events));
      expect(self(actors)).toMatchObject({clip:'flying_kick',moving:false});actors.dispose();
    }
  });
  it('welcome, a first snapshot and ready receipts cannot start travel',()=>{
    for(const kind of ['server_welcome','state_update']){
      canvas();const actors=new DungeonActors(assets());present(actors,snapshot(1,3,[move(0),move(1),move(2),hit()],false,'male',kind));
      expect(self(actors)).toMatchObject({moving:false});actors.dispose();
    }
    const actors=new DungeonActors(assets());present(actors,snapshot(1,0));present(actors,snapshot(2,3,[move(0),move(1),move(2),hit()],true));
    expect(self(actors)).toMatchObject({moving:false});actors.dispose();
  });
});
