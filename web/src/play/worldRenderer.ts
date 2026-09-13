import * as T from 'three';
import type {Snapshot,Coord,Frame} from '../authoritative/state';
import {frameTargets,type Target} from '../authoritative/targets';
import {PathOverlay} from './pathOverlay';
import type {WalkPresentation} from './pathControls';
import {bindWorldSpace} from './worldSpace';
import {SettlementScenery,SETTLEMENT_SHADOW_MAP} from './settlementScenery';
import {loadSettlementAssets,disposeSettlementAssets,type SettlementAssets} from './settlementAssets';
import {fitSettlementCamera} from './worldCamera';
import {prepareEntryBackdrop} from './entryBackdrop';
import {WorldGrid} from './worldGrid';
import './worldStyle.css';
import {isDungeon,observedDungeon,TILE,cellKey} from './dungeon/view';
import {fitDungeonCamera} from './dungeon/camera';
import {DungeonScenery} from './dungeon/scenery';
import {DungeonActors,loadDungeonBody} from './dungeon/actors';
import {DungeonOcclusion} from './dungeon/occlusion';

/** One 3D world surface; authoritative area selects scenery, never a renderer. */
export class WorldRenderer {
  readonly width=768;readonly height=768;
  private readonly renderer:T.WebGLRenderer;
  private readonly camera=new T.PerspectiveCamera(20,1,.1,250);
  private readonly scene=new T.Scene();
  private readonly dungeon=new DungeonScenery();
  private readonly settlement:SettlementScenery;
  private scenery: DungeonScenery|SettlementScenery;
  private disposeBackdrop:()=>void=()=>{};
  private readonly occlusion=new DungeonOcclusion();
  private readonly overlay=new PathOverlay();
  private readonly loot=new T.Group();
  private readonly grid=new WorldGrid();
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
    if(this.frame)this.draw();
  };
  private constructor(readonly canvas:HTMLCanvasElement,private readonly actors:DungeonActors,private readonly assets:SettlementAssets){
    this.settlement=new SettlementScenery(assets);this.scenery=this.settlement;
    this.renderer=new T.WebGLRenderer({canvas,antialias:true,preserveDrawingBuffer:true});
    this.renderer.shadowMap.enabled=true;this.renderer.shadowMap.type=SETTLEMENT_SHADOW_MAP;this.renderer.shadowMap.autoUpdate=false;
    this.renderer.toneMapping=T.ACESFilmicToneMapping;this.renderer.toneMappingExposure=1.3;
    this.graphicsError.className='world-graphics-error';this.graphicsError.setAttribute('role','alert');this.graphicsError.textContent='World graphics were lost. Reload to reconnect.';this.graphicsError.hidden=true;canvas.after(this.graphicsError);
    this.scene.background=new T.Color(0);this.scene.add(this.settlement.group,this.actors.group,this.overlay.group,this.loot,this.grid.mesh);
    this.overlay.group.scale.setScalar(TILE);this.canvas.dataset.presentation='world-3d';
    document.body.dataset.world3d='true';
  }
  private initialize():void {
    // A static menu view is drawn from the same verified 3D scene, without a
    // synthetic player or server frame. Its raster owns no live world state.
    this.settlement.presentStatic('arrival');
    this.renderer.shadowMap.needsUpdate=true;
    this.renderer.setSize(innerWidth,innerHeight);
    fitSettlementCamera(this.camera,{x:13,y:12},innerWidth,innerHeight);
    this.renderer.render(this.scene,this.camera);
    this.disposeBackdrop=prepareEntryBackdrop(this.canvas);this.settlement.clear();
    this.canvas.addEventListener('webglcontextlost',this.lost);window.addEventListener('resize',this.resize);this.resize();
    const animate=(now:number)=>{
      const dt=Math.min((now-this.last)/1000,.05);this.last=now;
      if(this.frame&&!document.hidden&&!this.failed){this.actors.update(dt);this.draw();}
      this.animation=requestAnimationFrame(animate);
    };this.animation=requestAnimationFrame(animate);
  }
  static async create(canvas:HTMLCanvasElement):Promise<WorldRenderer>{
    const assets=await loadSettlementAssets();let actors:DungeonActors|undefined,view:WorldRenderer|undefined;
    try {actors=new DungeonActors(await loadDungeonBody());view=new WorldRenderer(canvas,actors,assets);view.initialize();return view;}
    catch(error){if(view)view.dispose();else {actors?.dispose();disposeSettlementAssets(assets);}throw error;}
  }
  present(snapshot:Snapshot):void {
    if(this.failed)throw Error('World graphics unavailable. Reload to reconnect.');
    const level=bindWorldSpace(snapshot);
    if(this.frame&&this.frame.observation_center.level!==level)this.clear();
    const source=snapshot.envelope.frame;
    const frame=isDungeon(level)?observedDungeon(source):{...source,actors:source.actors.filter(a=>a.life_state!=='dead')};
    const next=isDungeon(level)?this.dungeon:this.settlement;
    if(this.scenery!==next){this.scenery.group.removeFromParent();this.scenery=next;this.scene.add(next.group);}
    this.renderer.shadowMap.type=isDungeon(level)?T.BasicShadowMap:SETTLEMENT_SHADOW_MAP;
    // Clear material references before a changed scene is disposed/rebuilt.
    this.occlusion.clear();
    const changed=this.scenery instanceof DungeonScenery?this.scenery.present(frame):this.scenery.present(snapshot);
    this.occlusion.bind(this.scenery.occluders);
    this.frame=frame;this.targets=frameTargets(frame,768,768).targets;
    this.actors.present(frame,snapshot);this.presentLoot(frame);this.grid.present(frame);
    if(changed)this.renderer.shadowMap.needsUpdate=true;
    this.canvas.dataset.studyLevel=level;this.canvas.dataset.studyActorCount=String(frame.actors.length);
    this.canvas.dataset.worldView=JSON.stringify({cells:isDungeon(level)?7:9,center:frame.observation_center.position,level,elevation:55,fieldOfView:20,
      tiles:frame.tiles.map(t=>({position:t.position,terrain:t.terrain_id,passable:t.passable,transition:t.transition})),
      walls:this.scenery instanceof DungeonScenery ? this.scenery.parts : [],torches:this.scenery instanceof DungeonScenery ? this.scenery.torchCount : 0,
      structures:this.scenery instanceof SettlementScenery?this.scenery.structures:[],playerLight:false});
    this.draw();
  }
  private presentLoot(frame:Frame):void {
    this.clearLoot();
    for(const target of this.targets.filter(t=>['corpse','gold_pile','ground_item'].includes(t.kind))){
      const material=new T.MeshStandardMaterial({color:target.kind==='gold_pile'?0xd5ab42:target.kind==='corpse'?0x775044:0x769080,roughness:.8});
      const mesh=new T.Mesh(new T.BoxGeometry(target.kind==='corpse'?.65:.22,.12,.3),material);
      mesh.position.set(target.coordinate.x*TILE+.36,.08,target.coordinate.y*TILE+.3);this.loot.add(mesh);
    }
    this.canvas.dataset.worldContents=String(frame.corpses.length+frame.ground_items.length+frame.gold_piles.length);
  }
  presentWalk(walk:WalkPresentation):void {
    if(!this.frame)return;
    const visible=new Set(this.frame.tiles.map(t=>cellKey(t.position)));
    this.overlay.present({...walk,route:walk.route?.filter(c=>visible.has(`${c.i}:${c.j}`))??null,
      hover:walk.hover&&visible.has(cellKey(walk.hover))?walk.hover:null},()=>0);
    this.draw();
  }
  private draw():void {
    if(this.frame)(isDungeon(this.frame.observation_center.level)?fitDungeonCamera:fitSettlementCamera)(this.camera,this.actors.anchor(this.frame.observer_actor_id)??this.frame.observation_center.position,
      this.canvas.clientWidth||innerWidth,this.canvas.clientHeight||innerHeight);
    this.scene.updateMatrixWorld(true);this.occlusion.update(this.camera,this.actors.bodies());this.renderer.render(this.scene,this.camera);
    if(!this.frame)return;
    this.canvas.dataset.worldOcclusion=String(this.occlusion.count);
    this.canvas.dataset.worldMotions=JSON.stringify(this.actors.diagnostics());
    this.canvas.dataset.worldDrawCalls=String(this.renderer.info.render.calls);
    this.canvas.dataset.worldGridEdges=String(this.grid.mesh.geometry.getAttribute('position')?.count/2||0);
    // Actual camera projection for proof and diagnosis, not a competing hit grid.
    this.canvas.dataset.worldPoints=JSON.stringify(this.frame.tiles.map(t=>{
      const p=new T.Vector3(t.position.x*TILE,.02,t.position.y*TILE).project(this.camera);
      return {...t.position,px:(p.x+1)/2,py:(1-p.y)/2};
    }));
    this.canvas.dataset.worldActorPoints=JSON.stringify(this.actors.group.children.map(o=>{const p=o.position.clone();p.y=1;p.project(this.camera);return {id:o.userData.actorId,px:(p.x+1)/2,py:(1-p.y)/2};}));
    this.canvas.dataset.worldActorAnchors=JSON.stringify(this.actors.group.children.map(o=>({id:o.userData.actorId,x:o.position.x/TILE,y:o.position.z/TILE})));
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
    this.frame=null;this.targets=[];this.occlusion.clear();this.dungeon.clear();this.settlement.clear();this.actors.clear();this.overlay.clear();this.grid.clear();this.clearLoot();this.renderer.clear();
    for(const key of ['studyLevel','worldView','worldPoints','worldActorAnchors','worldActorPoints','worldContents','worldOcclusion','worldDrawCalls','worldMotions','worldGridEdges'])delete this.canvas.dataset[key];
    this.canvas.dataset.studyActorCount='0';
  }
  dispose():void {cancelAnimationFrame(this.animation);window.removeEventListener('resize',this.resize);this.canvas.removeEventListener('webglcontextlost',this.lost);this.clear();this.dungeon.dispose();this.settlement.dispose();this.actors.dispose();disposeSettlementAssets(this.assets);this.disposeBackdrop();this.overlay.dispose();this.grid.dispose();this.renderer.dispose();this.graphicsError.remove();}
}
