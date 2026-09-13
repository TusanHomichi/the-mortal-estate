import * as T from 'three';
import type {FigureAsset} from './assets';

/** Pose time and transitions use the same elapsed clock as accepted travel. */
export class FigurePlayback {
  private readonly mixer:T.AnimationMixer;
  private action:T.AnimationAction;
  private last:number;
  private started:number;
  private endsAt:number|null=null;
  constructor(private readonly body:T.Object3D,private readonly asset:FigureAsset,now:number){
    this.mixer=new T.AnimationMixer(body);this.last=now;this.started=now;
    this.action=this.mixer.clipAction(asset.clips.find(c=>c.name===asset.idle)!).play();
    this.mixer.update(0);
  }
  get clip():string {return this.action.getClip().name;}
  play(name:string,now:number,seconds?:number):void {
    this.update(now);
    if(seconds===undefined&&this.clip===name)return;
    this.select(name,now,seconds);
    this.mixer.update(0);
  }
  private select(name:string,now:number,seconds?:number):void {
    const clip=this.asset.clips.find(c=>c.name===name);
    if(!clip)throw Error(`Required dungeon clip unavailable: ${name}.`);
    const previous=this.action,next=this.mixer.clipAction(clip),once=seconds!==undefined;
    next.reset().setEffectiveWeight(1).setEffectiveTimeScale(1).setLoop(once?T.LoopOnce:T.LoopRepeat,once?1:Infinity);
    next.clampWhenFinished=once;next.play();
    if(previous!==next)next.crossFadeFrom(previous,.12,false);
    if(once)next.setDuration(seconds);
    this.action=next;this.started=now;this.endsAt=once?now+seconds*1000:null;
  }
  update(now:number,walkDistance?:number):void {
    // Finish at the actual deadline before advancing the recovery. A delayed or
    // hidden-tab frame cannot begin an expired reaction's fade on its return.
    if(this.endsAt!==null&&now>=this.endsAt){
      const end=this.endsAt;this.mixer.update(Math.max(0,end-this.last)/1000);this.last=end;
      this.select(this.asset.idle,end);
    }
    if(this.clip==='walk'&&walkDistance!==undefined){
      this.action.time=(walkDistance/this.asset.stride*this.action.getClip().duration)%this.action.getClip().duration;
      this.action.paused=true;
    }
    this.mixer.update(Math.max(0,now-this.last)/1000);this.last=now;
  }
  diagnostics():{clip:string;phase:number;elapsedMs:number;weights:{clip:string;weight:number}[]} {
    return {clip:this.clip,phase:this.action.time/this.action.getClip().duration,elapsedMs:this.last-this.started,
      weights:this.asset.clips.flatMap(c=>{
        const action=this.mixer.existingAction(c),weight=action?.getEffectiveWeight()??0;
        return action?.isScheduled()&&weight>1e-6?[{clip:c.name,weight}]:[];
      })};
  }
  dispose():void {this.mixer.stopAllAction();this.mixer.uncacheRoot(this.body);}
}
