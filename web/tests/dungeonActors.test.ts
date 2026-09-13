import {describe,it,expect,vi,afterEach} from 'vitest';
import * as T from 'three';
import {DungeonActors,observerBody} from '../src/play/dungeon/actors';
import {assertFigureClips,COMBAT_CLIPS,type DungeonAssets,type FigureAsset} from '../src/play/dungeon/assets';
import type {Snapshot} from '../src/authoritative/state';
function asset(idle='guard'):FigureAsset {
 const scene=new T.Group(),bone=new T.Bone();bone.name='root';scene.add(bone);
 const clips=[...new Set([...COMBAT_CLIPS,idle])].map(name=>new T.AnimationClip(name,1,[new T.NumberKeyframeTrack('root.position[x]',[0,1],[0,.1])]));
 return {scene,clips,idle,stride:1,closingKick:{contactPhase:.25,landingPhase:.45}};
}
function assets():DungeonAssets{return {fallback:asset('idle'),male:asset(),female:asset(),tomas:asset('idle'),balm_seller:asset('idle')};}
function snapshot(sequence='1',o:{x?:number;y?:number;sex?:string|null;events?:unknown[];ready?:boolean;kind?:string}={}):Snapshot {
 const position={realm:'test',level:'d1_entry',position:{x:o.x??0,y:o.y??0}};
 return {generation:Number(sequence),raw:'',envelope:{kind:o.kind??'state_update',server_sequence:sequence,world_revision:sequence,static_scene_context:{},events:o.events??[],frame:{
 logical_time:'1000',ready_at:o.ready?'1000':'4000',can_act:o.ready??false,observer_actor_id:'self',observation_center:position,
 actors:[{actor_id:'self',name:'Player',position,life_state:'alive'}],tiles:[{position:{x:0,y:0},terrain_id:'floor'},{position:{x:0,y:1},terrain_id:'floor'},{position:{x:1,y:1},terrain_id:'floor'}],corpses:[],gold_piles:[],ground_items:[],
 character:{identity:{display_class:'Martial Artist',base_class_id:'martial_artist',sex_or_gender_display:o.sex??null}},carried:{items:[]}}}} as unknown as Snapshot;
}
const attack={kind:'feedback',cue:{kind:'physical_combat',source:{actor_id:'self'},target:{actor_id:'hidden'},mode:'fight',outcome:{kind:'hit'}}};
function moved(from:{x:number;y:number},to:{x:number;y:number}){return {kind:'actor_moved',actor_id:'self',from:{realm:'test',level:'d1_entry',position:from},to:{realm:'test',level:'d1_entry',position:to},navigation:'walk'};}
function canvas(){vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>({fillText:()=>{}})})});}
afterEach(()=>vi.unstubAllGlobals());
describe('dungeon shared character playback',()=>{
 it('keeps a ghost stationary and restores its own material without changing shared assets or other figures',()=>{
 canvas();let now=0;const source=assets(),material=new T.MeshStandardMaterial({opacity:.8});
 source.male.scene.add(new T.Mesh(new T.BoxGeometry(),material));
 source.tomas=source.male;
 const a=new DungeonActors(source,()=>now);let s=snapshot();
 s.envelope.frame.actors.push({...s.envelope.frame.actors[0]!,actor_id:'tomas'});
 a.present(s.envelope.frame,s);
 const mesh=(body:T.Object3D)=>{let found:T.Mesh|null=null;body.traverse(o=>{if(o instanceof T.Mesh)found=o;});return found!;};
 const own=mesh(a.bodies()[0]!).material as T.Material,other=mesh(a.bodies()[1]!).material as T.Material;
 expect(own).not.toBe(material);expect(other).not.toBe(own);
 const disposed=vi.spyOn(own,'dispose');
 s=snapshot('2',{x:1,y:1,events:[moved({x:0,y:0},{x:0,y:1}),moved({x:0,y:1},{x:1,y:1}),attack]});
 s.envelope.frame.actors[0]!.life_state='ghost';
 s.envelope.frame.actors.push({...s.envelope.frame.actors[0]!,actor_id:'tomas',life_state:'alive'});
 a.present(s.envelope.frame,s);now=1500;a.update();
 expect(a.diagnostics()).toContainEqual(expect.objectContaining({id:'self',ghost:true,moving:false,clip:'guard'}));
 expect(a.anchor('self')!.x).toBeCloseTo(1.26);expect(a.anchor('self')!.y).toBe(1);expect(own.opacity).toBeCloseTo(.28);expect(own.depthWrite).toBe(false);
 expect(material.opacity).toBe(.8);expect(other.opacity).toBe(.8);
 s=snapshot('3',{ready:true});a.present(s.envelope.frame,s);
 expect(own.opacity).toBe(.8);expect(own.transparent).toBe(false);expect(own.depthWrite).toBe(true);
 a.clear();expect(disposed).toHaveBeenCalledTimes(1);a.dispose();
 });
 it('animates existing non-martial characters and residents through the same accepted route seam',()=>{
 canvas();let now=0;const a=new DungeonActors(assets(),()=>now);let s=snapshot();
 s.envelope.frame.character.identity.base_class_id='fighter';a.present(s.envelope.frame,s);
 s=snapshot('2',{x:1,y:1,events:[moved({x:0,y:0},{x:0,y:1}),moved({x:0,y:1},{x:1,y:1})]});
 s.envelope.frame.character.identity.base_class_id='fighter';a.present(s.envelope.frame,s);now=1500;a.update();
 expect(a.diagnostics()).toMatchObject([{body:'male',clip:'walk',moving:true}]);
 expect(a.anchor('self')!.x).toBeCloseTo(0);expect(a.anchor('self')!.y).toBeCloseTo(1);
 s=snapshot('3',{ready:true});s.envelope.frame.actors.push({...s.envelope.frame.actors[0]!,actor_id:'tomas'});a.present(s.envelope.frame,s);
 expect(a.diagnostics()).toContainEqual(expect.objectContaining({id:'tomas',body:'tomas',clip:'idle'}));a.dispose();
 });
 it('uses selected sex for every player class and an explicit provisional default',()=>{
 expect(observerBody(snapshot())).toBe('male');expect(observerBody(snapshot('1',{sex:'Female'}))).toBe('female');
 const s=snapshot();s.envelope.frame.character.identity.base_class_id='fighter';expect(observerBody(s)).toBe('male');
 s.envelope.frame.character.identity.sex_or_gender_display='female';expect(observerBody(s)).toBe('female');
 });
 it('refuses missing, duplicate and unbound clips',()=>{
 const a=asset();assertFigureClips(a.scene,a.clips,COMBAT_CLIPS);
 expect(()=>assertFigureClips(a.scene,a.clips.filter(c=>c.name!=='walk'),COMBAT_CLIPS)).toThrow('walk');
 expect(()=>assertFigureClips(a.scene,[...a.clips,a.clips[0]!],COMBAT_CLIPS)).toThrow();
 const clip=new T.AnimationClip('walk',1,[new T.NumberKeyframeTrack('absent.position[x]',[0,1],[0,1])]);expect(()=>assertFigureClips(a.scene,[clip],['walk'])).toThrow('bone');
 });
 it('consumes a combat update once and expires a hidden-tab reaction',()=>{
 canvas();let now=0;const a=new DungeonActors(assets(),()=>now),s=snapshot('1',{events:[attack]});a.present(s.envelope.frame,s);
 expect(a.diagnostics()).toMatchObject([{clip:'jab_left'}]);now=500;a.present(s.envelope.frame,s);now=1001;a.update();expect(a.diagnostics()).toMatchObject([{clip:'guard'}]);a.dispose();
 });
 it('never replays welcome cues and clears sequence state on disconnect',()=>{
 canvas();const a=new DungeonActors(assets());let s=snapshot('1',{kind:'server_welcome',events:[attack]});a.present(s.envelope.frame,s);expect(a.diagnostics()).toMatchObject([{clip:'guard'}]);a.clear();expect(a.group.children).toHaveLength(0);
 s=snapshot('1',{events:[attack]});a.present(s.envelope.frame,s);expect(a.diagnostics()).toMatchObject([{clip:'jab_left'}]);a.dispose();
 });
 it('keeps the authoritative corner and ends travel on readiness',()=>{
 canvas();let now=0;const a=new DungeonActors(assets(),()=>now);let s=snapshot();a.present(s.envelope.frame,s);
 s=snapshot('2',{x:1,y:1,events:[moved({x:0,y:0},{x:0,y:1}),moved({x:0,y:1},{x:1,y:1})]});a.present(s.envelope.frame,s);now=1500;a.update();
 const root=a.group.children[0]!;expect(root.position.x).toBeCloseTo(0);expect(root.position.z).toBeCloseTo(1.65);expect(a.diagnostics()).toMatchObject([{clip:'walk',moving:true}]);
 s=snapshot('3',{x:1,y:1,ready:true});a.present(s.envelope.frame,s);expect(root.position.x).toBeCloseTo(1.65);expect(a.diagnostics()).toMatchObject([{clip:'guard',moving:false}]);a.dispose();
 });
 it('refuses a partial route and never animates a ready move past its deadline',()=>{
 canvas();const a=new DungeonActors(assets());let s=snapshot();a.present(s.envelope.frame,s);
 s=snapshot('2',{x:1,y:1,events:[moved({x:0,y:1},{x:1,y:1})]});a.present(s.envelope.frame,s);
 expect(a.diagnostics()).toMatchObject([{clip:'guard',moving:false}]);expect(a.group.children[0]!.position.x).toBeCloseTo(1.65);
 s=snapshot('3',{ready:true,events:[moved({x:1,y:1},{x:0,y:0})]});a.present(s.envelope.frame,s);
 expect(a.diagnostics()).toMatchObject([{clip:'guard',moving:false}]);expect(a.group.children[0]!.position.x).toBe(0);a.dispose();
 });
 it('varies consecutive punches independently of unrelated server sequence increments',()=>{
 canvas();const a=new DungeonActors(assets());
 for(const [sequence,clip] of [['1','jab_left'],['5','jab_right'],['9','uppercut_right'],['13','hook_left']]){
 const s=snapshot(sequence,{events:[attack]});a.present(s.envelope.frame,s);expect(a.diagnostics()).toMatchObject([{clip}]);a.present(s.envelope.frame,s);
 }a.dispose();
 });
 it('snaps missing-event moves without inventing a route',()=>{
 canvas();const a=new DungeonActors(assets());let s=snapshot();a.present(s.envelope.frame,s);s=snapshot('2',{x:1,y:1});a.present(s.envelope.frame,s);expect(a.diagnostics()).toMatchObject([{clip:'guard',moving:false}]);expect(a.group.children[0]!.position.x).toBeCloseTo(1.65);a.dispose();
 });
});
