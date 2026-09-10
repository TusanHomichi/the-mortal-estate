import * as T from 'three';
import {clone} from 'three/addons/utils/SkeletonUtils.js';
import {disposeSkeletons} from './rigResources';
import type {Frame,Snapshot,Coord} from '../../authoritative/state';
import {occupantAnchors,TILE,cellKey} from './view';
import {disposeFigureAssets,type DungeonAssets,type FigureAsset} from './assets';
import {combatCues,movementRoute,movementSeconds} from './motion';
export {loadDungeonBody} from './assets';

type BodyKind='fallback'|'male'|'female';
interface Travel { points:T.Vector3[]; lengths:number[]; distance:number; started:number; seconds:number }
interface Figure {
  root:T.Group;body:T.Object3D;mixer:T.AnimationMixer;label:T.Sprite;asset:FigureAsset;kind:BodyKind;
  action:T.AnimationAction;clip:string;position:Coord;travel:Travel|null;endsAt:number|null;
}

/** Only the controlled character currently exposes class/sex presentation data. */
export function observerBody(snapshot:Snapshot):BodyKind {
  const identity=snapshot.envelope.frame.character.identity;
  if(identity.base_class_id!=='martial_artist')return 'fallback';
  // Unspecified display uses the provisional male body, never a guessed identity.
  return identity.sex_or_gender_display?.trim().toLowerCase()==='female'?'female':'male';
}

export class DungeonActors {
  readonly group=new T.Group();
  private figures=new Map<string,Figure>();
  private sequence:string|null=null;
  private punchVariation=0;
  constructor(private readonly assets:DungeonAssets,private readonly now=()=>performance.now()){}
  present(frame:Frame,snapshot:Snapshot):void {
    const anchors=occupantAnchors(frame),active=new Set<string>(),self=frame.observer_actor_id;
    const fresh=this.sequence!==snapshot.envelope.server_sequence;
    const events=fresh&&snapshot.envelope.kind==='state_update'?snapshot.envelope.events??[]:[];
    const visibleCells=new Set(frame.tiles.map(t=>cellKey(t.position)));
    for(const row of frame.actors){
      if(row.life_state==='dead')continue;
      active.add(row.actor_id);const kind=row.actor_id===self?observerBody(snapshot):'fallback';
      let figure=this.figures.get(row.actor_id);
      if(figure&&figure.kind!==kind){this.remove(figure);this.figures.delete(row.actor_id);figure=undefined;}
      const created=!figure;
      if(!figure){
        const root=new T.Group(),asset=this.assets[kind],body=clone(asset.scene);root.add(body);
        body.traverse(o=>{if(o instanceof T.Mesh){o.castShadow=false;o.receiveShadow=true;}});
        const mixer=new T.AnimationMixer(body),action=mixer.clipAction(asset.clips.find(c=>c.name===asset.idle)!).play();
        const raster=document.createElement('canvas');raster.width=256;raster.height=40;
        const ink=raster.getContext('2d')!;ink.font='22px Georgia';ink.textAlign='center';ink.fillStyle='#ead7b0';ink.shadowColor='#000';ink.shadowBlur=4;ink.fillText(row.name,128,28,250);
        const label=new T.Sprite(new T.SpriteMaterial({map:new T.CanvasTexture(raster),transparent:true,depthTest:false,toneMapped:false}));
        label.position.y=2;label.scale.set(1.65,.26,1);label.renderOrder=25;root.add(label);
        figure={root,body,mixer,label,asset,kind,action,clip:asset.idle,position:row.position.position,travel:null,endsAt:null};
        this.figures.set(row.actor_id,figure);this.group.add(root);root.userData.actorId=row.actor_id;
      }
      const at=anchors.get(row.actor_id)!;const destination=new T.Vector3(at.x*TILE,0,at.y*TILE);
      const moved=cellKey(figure.position)!==cellKey(row.position.position);
      const route=!created&&moved?movementRoute(events,row.actor_id,row.position.level,row.position.realm,visibleCells,row.position.position):null;
      if(route&&cellKey(route[0]!)===cellKey(figure.position)&&figure.kind!=='fallback'&&!frame.can_act){
        const points=route.map(p=>new T.Vector3(p.x*TILE,0,p.y*TILE));points[points.length-1]=destination;
        const lengths=points.slice(1).map((p,i)=>p.distanceTo(points[i]!));
        figure.travel={points,lengths,distance:lengths.reduce((a,b)=>a+b,0),started:this.now(),
          seconds:row.actor_id===self?movementSeconds(frame.logical_time,frame.ready_at):.18};
        figure.root.position.copy(points[0]!);figure.endsAt=null;this.play(figure,'walk');
      }else if(created||moved){figure.travel=null;figure.endsAt=null;figure.root.position.copy(destination);this.play(figure,figure.asset.idle);}
      else if(row.actor_id===self&&frame.can_act&&figure.travel){figure.travel=null;figure.root.position.copy(destination);this.play(figure,figure.asset.idle);}
      else if(!figure.travel)figure.root.position.copy(destination);
      figure.position={...row.position.position};
    }
    for(const [id,figure] of this.figures)if(!active.has(id)){this.remove(figure);this.figures.delete(id);}
    const observer=this.figures.get(self);
    if(observer&&observer.kind!=='fallback'){
      const unarmed=!snapshot.envelope.frame.carried.items.some(item=>item.position==='right_hand');
      const cues=combatCues(events,self,active,this.punchVariation,unarmed);
      this.punchVariation=(this.punchVariation+cues.filter(c=>['jab_left','jab_right','uppercut_right','hook_left'].includes(c.clip)).length)%4;
      for(const cue of cues){const figure=this.figures.get(cue.actorId);if(!figure)continue;
        const target=cue.faceActorId?this.figures.get(cue.faceActorId):null;
        if(target)this.face(figure,target.root.position.clone().sub(figure.root.position));
        this.play(figure,cue.clip,true);
      }
    }
    this.sequence=snapshot.envelope.server_sequence;
  }
  private face(f:Figure,direction:T.Vector3):void {
    if(direction.x*direction.x+direction.z*direction.z>1e-8)f.body.rotation.y=Math.atan2(direction.x,direction.z);
  }
  private play(f:Figure,name:string,once=false):void {
    if(!once&&f.clip===name)return;
    const clip=f.asset.clips.find(c=>c.name===name);if(!clip)throw Error(`Required dungeon clip unavailable: ${name}.`);
    const next=f.mixer.clipAction(clip);const previous=f.action;
    next.reset().setEffectiveWeight(1).setEffectiveTimeScale(1).setLoop(once?T.LoopOnce:T.LoopRepeat,once?1:Infinity);
    next.clampWhenFinished=once;next.play();
    if(previous!==next)next.crossFadeFrom(previous,.12,false);
    if(once){const seconds=Math.min(clip.duration,1.5);next.setDuration(seconds);f.endsAt=this.now()+seconds*1000;}
    else f.endsAt=null;
    f.action=next;f.clip=name;
  }
  update(dt:number):void {
    const now=this.now();
    for(const f of this.figures.values()){
      if(f.endsAt!==null&&now>=f.endsAt)this.play(f,f.travel?'walk':f.asset.idle);
      const travel=f.travel;
      if(travel){
        const progress=Math.min(1,Math.max(0,(now-travel.started)/(travel.seconds*1000)));let remaining=progress*travel.distance;
        for(let i=0;i<travel.lengths.length;i++){
          const length=travel.lengths[i]!;
          if(remaining<=length||i===travel.lengths.length-1){
            f.root.position.lerpVectors(travel.points[i]!,travel.points[i+1]!,length?remaining/length:1);
            this.face(f,travel.points[i+1]!.clone().sub(travel.points[i]!));break;
          }remaining-=length;
        }
        if(f.clip==='walk'){
          // Pose phase follows actual rendered distance, including run/sprint paths.
          f.action.time=(progress*travel.distance/f.asset.stride*f.action.getClip().duration)%f.action.getClip().duration;
          f.action.paused=true;
        }
        if(progress===1){f.root.position.copy(travel.points.at(-1)!);f.travel=null;if(f.endsAt===null)this.play(f,f.asset.idle);}
      }
      f.mixer.update(dt);
    }
  }
  diagnostics():unknown[]{return [...this.figures].map(([id,f])=>({id,body:f.kind,clip:f.clip,moving:f.travel!==null}));}
  anchor(id:string):Coord|null {const f=this.figures.get(id);return f?{x:f.root.position.x/TILE,y:f.root.position.z/TILE}:null;}
  bodies():T.Object3D[]{return [...this.figures.values()].map(f=>f.body);}
  pick(ray:T.Raycaster):string|null {
    const hit=ray.intersectObjects(this.bodies(),true)[0];if(!hit)return null;
    let o:T.Object3D|null=hit.object;while(o&&!o.userData.actorId)o=o.parent;return o?.userData.actorId??null;
  }
  private remove(f:Figure):void {f.mixer.stopAllAction();f.mixer.uncacheRoot(f.body);disposeSkeletons(f.body);f.root.removeFromParent();f.label.material.map?.dispose();f.label.material.dispose();}
  clear():void {for(const f of this.figures.values())this.remove(f);this.figures.clear();this.sequence=null;this.punchVariation=0;}
  dispose():void {this.clear();disposeFigureAssets(Object.values(this.assets));}
}
