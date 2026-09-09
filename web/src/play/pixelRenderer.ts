import type { Snapshot, Coord } from "../authoritative/state";
import type { WalkPresentation } from "./pathControls";
import { loadPixelPacket, bindPixelSpace, type PixelPacket, type PixelFigure } from "./pixelPacket";
import { TEMPLE_PROJECTION, projectPixel, unprojectPixel, pixelDirection, pixelSpriteRect } from "./pixelGeometry";
import { pixelCamera, pixelViewport } from "./pixelViewport";
import { PixelExteriorRenderer, exteriorLayerCovers, type ExteriorLayer } from "./pixelExterior";
import { pixelTravelDuration, samplePixelTravel, pixelStrideFrame } from "./pixelMotion";
import { PixelEffects } from "./pixelEffects";
import { PixelOverlays } from "./pixelOverlays";
import { nextPixelFrame } from "./pixelCadence";
import "./pixelStyle.css";

interface Sprite {
  id: string; name: string; at: Coord; target: Coord; direction: string;
  route: Coord[] | null; started: number; duration: number; distance: number; figure: PixelFigure | null;
}
interface Hit { id: string; x: number; y: number; width: number; height: number; depth: number }
const SIZE = 512;
/** Primary pixel presentation. No transports, commands, deadlines or gameplay ledger. */
export class PixelRenderer {
  private readonly floor = document.createElement("canvas");
  private readonly overlays = new PixelOverlays();
  private readonly graphicsError = document.createElement("p");
  private viewport = {width:512,height:512,scale:1};
  private roomOffset = {x:0,y:0};
  private readonly resize = () => {
    this.viewport=pixelViewport(window.innerWidth,window.innerHeight);
    const {width,height,scale}=this.viewport;
    this.canvas.width=width*scale;this.canvas.height=height*scale;
    this.canvas.style.width=`${width*scale}px`;this.canvas.style.height=`${height*scale}px`;
    this.canvas.dataset.pixelViewport=JSON.stringify(this.viewport);
    this.canvas.dataset.pixelNativeSize=`${width}x${height}`;
    this.canvas.dataset.pixelSpriteRaster=`${width*scale}x${height*scale}`;
    this.pointingReady=false;this.hits=[];
    delete this.canvas.dataset.pixelProjection;delete this.canvas.dataset.pixelActorBounds;
    this.dirty=true;
  };
  private readonly exterior: PixelExteriorRenderer;
  private readonly effects: PixelEffects;
  private exteriorLayers: ExteriorLayer[]=[];
  private snapshot: Snapshot | null = null;
  private level: string | null = null;
  private projection = TEMPLE_PROJECTION;
  private walk: WalkPresentation | null = null;
  private readonly sprites = new Map<string, Sprite>();
  private readonly spriteRasters = new Map<string,HTMLCanvasElement>();
  private hits: Hit[] = [];
  private pointingReady = false;
  private animation = 0;
  private dirty = true;
  private nextDraw = -Infinity;
  private lastMetric = 0;
  private readonly costs: number[] = [];

  private constructor(readonly canvas: HTMLCanvasElement, readonly width: number, readonly height: number,
    private readonly packet: PixelPacket) {
    this.effects=new PixelEffects(packet,canvas);
    this.graphicsError.className="pixel-graphics-error";this.graphicsError.setAttribute('role','alert');this.graphicsError.hidden=true;
    canvas.after(this.graphicsError);
    canvas.dataset.pixelEffects="webgl";
    canvas.dataset.pixelAtmosphere=this.effects.profileName;
    this.exterior=new PixelExteriorRenderer(packet);
    this.floor.width = SIZE; this.floor.height = SIZE;
    const floor = this.floor.getContext("2d")!;
    floor.imageSmoothingEnabled = true;
    floor.imageSmoothingQuality = "high";
    floor.drawImage(packet.images.get(packet.manifest.background.file)!, 0, 0, this.floor.width, this.floor.height);
    canvas.dataset.presentation = "pixel-art";
    this.resize();
    window.addEventListener("resize",this.resize);
    document.body.dataset.pixelArt = "true";
    document.title = "The Mortal Estate";
    const animate = (now: number) => {
      if (!document.hidden && now + .1 >= this.nextDraw && (this.dirty || this.moving() || this.level==="arrival" || this.level==="temple")) {
        const start = performance.now();
        try {this.draw(now);} catch(error) {
          this.clear();canvas.dataset.pixelEffects="failed";
          this.graphicsError.textContent=error instanceof Error ? error.message : "Pixel graphics failed. Reload to reconnect.";
          this.graphicsError.hidden=false;
          return;
        }
        this.nextDraw = nextPixelFrame(this.nextDraw,now); this.dirty = false;
        this.costs.push(performance.now() - start); if (this.costs.length > 240) this.costs.shift();
        if (now - this.lastMetric > 1000) {
          const sorted = [...this.costs].sort((a,b) => a-b);
          canvas.dataset.pixelDrawP95Ms = String(sorted[Math.floor((sorted.length - 1) * .95)] ?? 0);
          this.lastMetric = now;
        }
      }
      this.animation = requestAnimationFrame(animate);
    };
    this.animation = requestAnimationFrame(animate);
  }
  static async create(canvas: HTMLCanvasElement, width: number, height: number): Promise<PixelRenderer> {
    return new PixelRenderer(canvas, width, height, await loadPixelPacket());
  }
  private moving(): boolean { return [...this.sprites.values()].some(sprite => sprite.route !== null); }
  private advance(sprite: Sprite, now: number): void {
    const route = sprite.route;
    if (!route) return;
    const sampled=samplePixelTravel(route,now-sprite.started,sprite.duration);
    sprite.direction = pixelDirection(sampled.from,sampled.to,!!sprite.figure?.walk["north-east"]);
    sprite.at = sampled.at; sprite.distance=sampled.distance;
    if (sampled.done) sprite.route=null;
  }
  present(snapshot: Snapshot): void {
    let level: string;
    try { level = bindPixelSpace(snapshot); } catch (error) { this.clear(); throw error; }
    if (level !== this.level) { this.clear(); this.level = level; }
    const frame = snapshot.envelope.frame;
    const visible = new Set<string>();
    const now = performance.now();
    for (const actor of frame.actors) {
      if ((actor as typeof actor & { life_state: string }).life_state === "dead") continue;
      visible.add(actor.actor_id);
      const target = actor.position.position;
      let sprite = this.sprites.get(actor.actor_id);
      if (!sprite) {
        const figure = actor.actor_id === frame.observer_actor_id ? "traveler" : this.packet.manifest.actor_figures[actor.actor_id];
        sprite = { id:actor.actor_id,name:actor.name,at:{ ...target },target:{ ...target },direction:"south",
          route:null,started:now,duration:0,distance:0,figure:figure ? this.packet.manifest.figures[figure]! : null };
        this.sprites.set(actor.actor_id,sprite);
      } else if (target.x !== sprite.target.x || target.y !== sprite.target.y) {
        this.advance(sprite,now);
        const planned = actor.actor_id === frame.observer_actor_id && this.walk?.kind === "committed" ? this.walk.route : null;
        const end = planned?.findIndex(cell => cell.i === target.x && cell.j === target.y) ?? -1;
        const matches = planned?.[0]?.i === sprite.target.x && planned?.[0]?.j === sprite.target.y;
        sprite.route = end > 0 && matches ? [{ ...sprite.at }, ...planned!.slice(1,end+1).map(cell => ({ x:cell.i,y:cell.j }))]
          : [{ ...sprite.at }, { ...target }];
        const remaining=BigInt(frame.ready_at)-BigInt(frame.logical_time);
        sprite.duration=pixelTravelDuration(sprite.route,actor.actor_id===frame.observer_actor_id
          ? Number(remaining>2500n ? 2500n : remaining>0n ? remaining : 0n) : 2500);
        sprite.target = { ...target }; sprite.started = now; sprite.distance=0;
        if (actor.actor_id===frame.observer_actor_id) {
          this.canvas.dataset.pixelMotionRoute=JSON.stringify(sprite.route);
          this.canvas.dataset.pixelMotionDurationMs=String(sprite.duration);
        }
      }
    }
    for (const id of this.sprites.keys()) if (!visible.has(id)) this.sprites.delete(id);
    this.snapshot = snapshot; this.dirty = true;
    this.canvas.dataset.studyLevel = level;
    this.canvas.dataset.studyActorCount = String(this.sprites.size);
  }
  presentWalk(walk: WalkPresentation): void { this.walk = walk; this.dirty = true; }
  private draw(now: number): void {
    if(!this.dirty&&!this.moving()){this.blit(now);return;}
    const c = this.effects, snapshot = this.snapshot;this.hits=[];
    if (!snapshot) {this.effects.begin(null,this.viewport.width,this.viewport.height,{x:0,y:0},this.viewport.scale);this.blit(now);return;}
    const frame = snapshot.envelope.frame;
    const sprites = [...this.sprites.values()]; for (const sprite of sprites) this.advance(sprite,now);
    const focus=this.sprites.get(frame.observer_actor_id)?.at??frame.observation_center.position;
    const viewport={x:this.viewport.width,y:this.viewport.height};
    const base=this.level==="temple" ? TEMPLE_PROJECTION : this.level==="arrival" ? this.packet.manifest.exterior.projection : {origin:{x:0,y:0},step:{x:32,y:25}};
    if(this.level==="temple"||this.level==="arrival") {
      const extent=this.level==="temple" ? {x:512,y:512} : {x:this.packet.manifest.exterior.width,y:this.packet.manifest.exterior.height};
      this.projection=pixelCamera(base,focus,viewport,extent);
    } else this.projection={origin:{x:Math.round(viewport.x/2-focus.x*32),y:Math.round(viewport.y*.6-focus.y*25)},step:{x:32,y:25}};
    this.roomOffset={x:this.projection.origin.x-base.origin.x,y:this.projection.origin.y-base.origin.y};
    this.canvas.dataset.pixelProjection=JSON.stringify(this.projection);
    this.effects.begin(this.level,viewport.x,viewport.y,this.roomOffset,this.viewport.scale);
    this.exteriorLayers=this.level==="arrival" ? this.exterior.layers(this.projection,viewport.x,viewport.y) : [];
    if (this.level === "temple") c.paint(this.floor,this.roomOffset.x,this.roomOffset.y);
    else if(this.level==="arrival")c.paint(this.packet.images.get(this.packet.manifest.exterior.background.file)!,this.roomOffset.x,this.roomOffset.y);
    if(this.overlays.prepare(frame,this.walk,base,this.level!,this.viewport.scale)){
      c.invalidate(this.overlays.ink);c.invalidate(this.overlays.floor);
    }
    const {origin,width:overlayWidth,height:overlayHeight}=this.overlays;
    const overlayX=origin.x+this.roomOffset.x,overlayY=origin.y+this.roomOffset.y;
    if(this.level!=="temple"&&this.level!=="arrival"){
      c.paint(this.overlays.floor,overlayX,overlayY,overlayWidth,overlayHeight);
      const label=this.overlays.label(`${this.level!.replaceAll("_"," ")} · map view`,"12px Georgia","#e4d1a5",this.viewport.scale);
      c.paint(label,(viewport.x-label.width/this.viewport.scale)/2,6,label.width/this.viewport.scale,24);
    }
    c.paint(this.overlays.ink,overlayX,overlayY,overlayWidth,overlayHeight);
    sprites.sort((a,b)=>a.at.y-b.at.y || a.at.x-b.at.x);
    const foreground=this.level==="temple" ? this.packet.manifest.temple_foreground.map(r=>({...r,sourceX:r.x,sourceY:r.y,x:r.x+this.roomOffset.x,y:r.y+this.roomOffset.y,depth:r.depth+this.roomOffset.y})).sort((a,b)=>a.depth-b.depth) : [];
    const outside=[...this.exteriorLayers];
    const revealForeground=(depth:number)=>{
      while (foreground.length && foreground[0]!.depth<=depth) {
        const r=foreground.shift()!;
        c.paint(this.floor,r.x,r.y,r.width,r.height,[r.sourceX,r.sourceY,r.width,r.height]);
        this.effects.foreground(r.x,r.y,r.width,r.height);
      }
      while(outside.length&&outside[0]!.depth<=depth) {
        const layer=outside.shift()!;c.paint(layer.source.image,layer.x,layer.y,layer.width,layer.height);
        this.effects.foreground(layer.x,layer.y,layer.width,layer.height,layer.source.image);
      }
    };
    const contents=[...frame.corpses,...frame.ground_items,...frame.gold_piles];
    const drawables=[...sprites.map(sprite=>({at:sprite.at,sprite})),...contents.map(row=>({at:row.location.position,sprite:null}))]
      .sort((a,b)=>a.at.y-b.at.y||a.at.x-b.at.x);
    for (const {at,sprite} of drawables) {
      const p=projectPixel(at,this.projection);
      revealForeground(p.y);
      if(!sprite){c.fill("#d4b87d",Math.round(p.x)-3,Math.round(p.y)-3,6,4);continue;}
      const sharing = sprites.some(other=>other.id!==sprite.id && other.target.x===sprite.target.x && other.target.y===sprite.target.y);
      if (sharing) p.x += sprite.id===frame.observer_actor_id ? -11 : 11;
      c.paint(this.overlays.contact(sprite.id===frame.observer_actor_id,this.viewport.scale),Math.round(p.x)-20,Math.round(p.y)-12,40,20);
      if (sprite.figure) {
        const f=sprite.figure, frames=f.walk[sprite.direction]!;
        const asset=sprite.route ? frames[pixelStrideFrame(sprite.distance,frames.length)]! : f.rotations[sprite.direction]!;
        const image=this.packet.images.get(asset.file)!;
        const referenceHeight=sprite.id===frame.observer_actor_id ? f.rotations.south!.body_height : asset.body_height;
        const {size,x,y}=pixelSpriteRect(p,{...asset,body_height:referenceHeight},f.size,this.level==="temple" ? 96 : this.level==="arrival" ? 72 : 44);
        const detailSize=size*this.viewport.scale;
        const key=`${asset.file}:${detailSize}`;
        let raster=this.spriteRasters.get(key);
        if(!raster) {
          raster=document.createElement("canvas");raster.width=detailSize;raster.height=detailSize;
          const prepared=raster.getContext("2d")!;
          prepared.imageSmoothingEnabled=true;prepared.imageSmoothingQuality="high";
          prepared.drawImage(image,0,0,detailSize,detailSize);this.spriteRasters.set(key,raster);
        }
        this.effects.actor(raster,x,y,size,p);
        this.hits.push({id:sprite.id,x:x+size*.2,y:y+size*.05,width:size*.6,height:size*.92,depth:p.y});
      } else {
        c.fill("#bd8566",Math.round(p.x)-4,Math.round(p.y)-12,8,12);
        this.hits.push({id:sprite.id,x:p.x-7,y:p.y-18,width:14,height:18,depth:p.y});
      }
    }
    revealForeground(Infinity);
    this.canvas.dataset.pixelActorBounds=JSON.stringify(this.hits);
    if (this.level !== "temple" && this.level!=="arrival") {
      c.fill("#151519db",12,this.viewport.height-35,this.viewport.width-24,24);
      const label=this.overlays.label("AREA ART IN PROGRESS","10px Georgia","#ead5ae",this.viewport.scale);
      c.paint(label,(viewport.x-label.width/this.viewport.scale)/2,viewport.y-38,label.width/this.viewport.scale,24);
    }
    this.blit(now);
    this.pointingReady=true;
  }
  private blit(now=performance.now()):void {
    this.effects.render(now);
  }
  pointer(x: number,y: number): { coordinate: Coord } | null {
    if (!this.snapshot || !this.pointingReady || x<0 || y<0 || x>=this.width || y>=this.height) return null;
    const coordinate=unprojectPixel({x:x*this.viewport.width/this.width,y:y*this.viewport.height/this.height},this.projection);
    return this.snapshot.envelope.frame.tiles.some(tile=>tile.position.x===coordinate.x&&tile.position.y===coordinate.y)
      ? {coordinate} : null;
  }
  pickActor(x: number,y: number): string | null {
    if (!this.snapshot || !this.pointingReady || x<0 || y<0 || x>=this.width || y>=this.height) return null;
    const px=x*this.viewport.width/this.width,py=y*this.viewport.height/this.height;
    const roomX=px-this.roomOffset.x,roomY=py-this.roomOffset.y;
    return [...this.hits].reverse().find(hit=>hit.id!==this.snapshot!.envelope.frame.observer_actor_id &&
      px>=hit.x && px<=hit.x+hit.width && py>=hit.y && py<=hit.y+hit.height &&
      !(this.level==="temple" && this.packet.manifest.temple_foreground.some(r=>r.depth+this.roomOffset.y>hit.depth &&
        roomX>=r.x && roomX<r.x+r.width && roomY>=r.y && roomY<r.y+r.height)) &&
      !this.exteriorLayers.some(layer=>layer.depth>hit.depth&&exteriorLayerCovers(layer,{x:px,y:py})))?.id ?? null;
  }
  clear(): void {
    this.snapshot=null;this.level=null;this.walk=null;this.sprites.clear();this.hits=[];this.pointingReady=false;
    this.exteriorLayers=[];
    this.effects.begin(null,this.viewport.width,this.viewport.height,{x:0,y:0},this.viewport.scale);
    this.effects.clear();this.dirty=true;
    delete this.canvas.dataset.studyLevel; this.canvas.dataset.studyActorCount="0";
    delete this.canvas.dataset.pixelMotionRoute; delete this.canvas.dataset.pixelMotionDurationMs;
    delete this.canvas.dataset.pixelProjection;
    delete this.canvas.dataset.pixelActorBounds;
  }
  dispose(): void { cancelAnimationFrame(this.animation);window.removeEventListener("resize",this.resize);this.clear();this.spriteRasters.clear();this.overlays.dispose();this.effects.dispose();this.graphicsError.remove(); }
}
