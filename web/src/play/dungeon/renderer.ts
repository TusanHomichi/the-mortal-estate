import * as T from 'three';
import type {Snapshot,Coord,Frame} from '../../authoritative/state';
import {frameTargets,type Target} from '../../authoritative/targets';
import {PathOverlay} from '../pathOverlay';
import type {WalkPresentation} from '../pathControls';
import {bindPixelSpace} from '../pixelPacket';
import {isDungeon,observedDungeon,TILE,cellKey} from './view';
import {fitDungeonCamera} from './camera';
import {DungeonScenery} from './scenery';
import {DungeonActors,loadDungeonBody} from './actors';
import {DungeonOcclusion} from './occlusion';

/** The same authoritative renderer seam, drawing only current observed cells. */
export class DungeonRenderer {
  readonly width=768;readonly height=768;
  private readonly renderer:T.WebGLRenderer;
  private readonly camera=new T.PerspectiveCamera(20,1,.1,250);
  private readonly scene=new T.Scene();
  private readonly scenery=new DungeonScenery();
  private readonly occlusion=new DungeonOcclusion();
  private readonly overlay=new PathOverlay();
  private readonly loot=new T.Group();
  private readonly ray=new T.Raycaster();
  private frame:Frame|null=null;
  private targets:Target[]=[];
  private animation=0;
  private failed=false;
  private readonly graphicsError=document.createElement('p');
  private last=performance.now();
  private readonly lost=(event:Event)=>{event.preventDefault();this.failed=true;this.clear();this.canvas.dataset.presentationError='graphics-context-lost';this.graphicsError.hidden=false;};
  private readonly resize=()=>{
    const width=Math.max(1,window.innerWidth),height=Math.max(1,window.innerHeight);
    this.renderer.setPixelRatio(Math.min(devicePixelRatio,2));this.renderer.setSize(width,height);
    this.canvas.style.imageRendering='auto';
    if(this.frame){fitDungeonCamera(this.camera,this.frame.observation_center.position,width,height);this.draw();}
  };
  private constructor(readonly canvas:HTMLCanvasElement,private readonly actors:DungeonActors){
    this.renderer=new T.WebGLRenderer({canvas,antialias:true,preserveDrawingBuffer:true});
    this.renderer.shadowMap.enabled=true;this.renderer.shadowMap.type=T.BasicShadowMap;this.renderer.shadowMap.autoUpdate=false;
    this.renderer.toneMapping=T.ACESFilmicToneMapping;this.renderer.toneMappingExposure=1.3;
    this.graphicsError.className='pixel-graphics-error';this.graphicsError.setAttribute('role','alert');this.graphicsError.textContent='Dungeon graphics were lost. Reload to reconnect.';this.graphicsError.hidden=true;canvas.after(this.graphicsError);
    this.scene.background=new T.Color(0);this.scene.add(this.scenery.group,this.actors.group,this.overlay.group,this.loot);
    this.overlay.group.scale.setScalar(TILE);this.canvas.dataset.presentation='dungeon-3d';
    this.canvas.addEventListener('webglcontextlost',this.lost);window.addEventListener('resize',this.resize);this.resize();
    const animate=(now:number)=>{
      const dt=Math.min((now-this.last)/1000,.05);this.last=now;
      if(this.frame&&!document.hidden&&!this.failed){this.actors.update(dt);this.draw();}
      this.animation=requestAnimationFrame(animate);
    };this.animation=requestAnimationFrame(animate);
  }
  static async create(canvas:HTMLCanvasElement):Promise<DungeonRenderer>{return new DungeonRenderer(canvas,new DungeonActors(await loadDungeonBody()));}
  present(snapshot:Snapshot):void {
    if(this.failed)throw Error('Dungeon graphics unavailable. Reload to reconnect.');
    const level=bindPixelSpace(snapshot);if(!isDungeon(level))throw Error('Non-dungeon frame reached dungeon renderer.');
    if(this.frame&&this.frame.observation_center.level!==level)this.clear();
    const frame=observedDungeon(snapshot.envelope.frame);
    // Clear material references before a changed scene is disposed/rebuilt.
    this.occlusion.clear();
    const changed=this.scenery.present(frame);
    this.occlusion.bind(this.scenery.occluders);
    this.frame=frame;this.targets=frameTargets(frame,768,768).targets;
    this.actors.present(frame);this.presentLoot(frame);
    if(changed)this.renderer.shadowMap.needsUpdate=true;
    fitDungeonCamera(this.camera,frame.observation_center.position,this.canvas.clientWidth||innerWidth,this.canvas.clientHeight||innerHeight);
    this.canvas.dataset.studyLevel=level;this.canvas.dataset.studyActorCount=String(frame.actors.length);
    this.canvas.dataset.dungeonView=JSON.stringify({cells:7,center:frame.observation_center.position,level,elevation:55,fieldOfView:20,
      tiles:frame.tiles.map(t=>({position:t.position,terrain:t.terrain_id,passable:t.passable,transition:t.transition})),
      walls:this.scenery.parts,torches:this.scenery.torchCount,playerLight:false});
    this.draw();
  }
  private presentLoot(frame:Frame):void {
    this.clearLoot();
    for(const target of this.targets.filter(t=>['corpse','gold_pile','ground_item'].includes(t.kind))){
      const material=new T.MeshStandardMaterial({color:target.kind==='gold_pile'?0xd5ab42:target.kind==='corpse'?0x775044:0x769080,roughness:.8});
      const mesh=new T.Mesh(new T.BoxGeometry(target.kind==='corpse'?.65:.22,.12,.3),material);
      mesh.position.set(target.coordinate.x*TILE+.36,.08,target.coordinate.y*TILE+.3);this.loot.add(mesh);
    }
    this.canvas.dataset.dungeonContents=String(frame.corpses.length+frame.ground_items.length+frame.gold_piles.length);
  }
  presentWalk(walk:WalkPresentation):void {
    if(!this.frame)return;
    const visible=new Set(this.frame.tiles.map(t=>cellKey(t.position)));
    this.overlay.present({...walk,route:walk.route?.filter(c=>visible.has(`${c.i}:${c.j}`))??null,
      hover:walk.hover&&visible.has(cellKey(walk.hover))?walk.hover:null},()=>0);
    this.draw();
  }
  private draw():void {
    this.scene.updateMatrixWorld(true);this.occlusion.update(this.camera,this.actors.bodies());this.renderer.render(this.scene,this.camera);
    if(!this.frame)return;
    this.canvas.dataset.dungeonOcclusion=String(this.occlusion.count);
    this.canvas.dataset.dungeonDrawCalls=String(this.renderer.info.render.calls);
    // Actual camera projection for proof and diagnosis, not a competing hit grid.
    this.canvas.dataset.dungeonPoints=JSON.stringify(this.frame.tiles.map(t=>{
      const p=new T.Vector3(t.position.x*TILE,.02,t.position.y*TILE).project(this.camera);
      return {...t.position,px:(p.x+1)/2,py:(1-p.y)/2};
    }));
    this.canvas.dataset.dungeonActorPoints=JSON.stringify(this.actors.group.children.map(o=>{const p=o.position.clone();p.y=1;p.project(this.camera);return {id:o.userData.actorId,px:(p.x+1)/2,py:(1-p.y)/2};}));
    this.canvas.dataset.dungeonActorAnchors=JSON.stringify(this.actors.group.children.map(o=>({id:o.userData.actorId,x:o.position.x/TILE,y:o.position.z/TILE})));
  }
  private pointerRay(x:number,y:number):boolean {
    if(!this.frame||this.failed||x<0||y<0||x>=this.width||y>=this.height)return false;
    this.ray.setFromCamera(new T.Vector2(x/this.width*2-1,1-y/this.height*2),this.camera);return true;
  }
  pointer(x:number,y:number):{coordinate:Coord}|null {
    if(!this.pointerRay(x,y))return null;
    const point=this.ray.ray.intersectPlane(new T.Plane(new T.Vector3(0,1,0),0),new T.Vector3());if(!point)return null;
    const coordinate={x:Math.floor(point.x/TILE+.5),y:Math.floor(point.z/TILE+.5)};
    return this.targets.some(t=>t.kind==='tile'&&cellKey(t.coordinate)===cellKey(coordinate))?{coordinate}:null;
  }
  pickActor(x:number,y:number):string|null {return this.pointerRay(x,y)?this.actors.pick(this.ray):null;}
  private clearLoot():void {this.loot.traverse(o=>{if(o instanceof T.Mesh){o.geometry.dispose();(o.material as T.Material).dispose();}});this.loot.clear();}
  clear():void {
    this.frame=null;this.targets=[];this.occlusion.clear();this.scenery.clear();this.actors.clear();this.overlay.clear();this.clearLoot();this.renderer.clear();
    for(const key of ['studyLevel','dungeonView','dungeonPoints','dungeonActorAnchors','dungeonActorPoints','dungeonContents','dungeonOcclusion','dungeonDrawCalls'])delete this.canvas.dataset[key];
    this.canvas.dataset.studyActorCount='0';
  }
  dispose():void {cancelAnimationFrame(this.animation);window.removeEventListener('resize',this.resize);this.canvas.removeEventListener('webglcontextlost',this.lost);this.clear();this.scenery.dispose();this.actors.dispose();this.overlay.dispose();this.renderer.dispose();this.graphicsError.remove();}
}
