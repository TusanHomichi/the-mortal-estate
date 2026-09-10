import type {Snapshot,Coord} from '../authoritative/state';
import type {WalkPresentation} from './pathControls';
import {PixelRenderer} from './pixelRenderer';
import {DungeonRenderer} from './dungeon/renderer';
import {isDungeon} from './dungeon/view';

/** Authored area selects its presentation. Neither backend is an error fallback. */
export class WorldRenderer {
  private active:PixelRenderer|DungeonRenderer;
  private constructor(private readonly pixel:PixelRenderer,private readonly dungeon:DungeonRenderer){this.active=pixel;dungeon.canvas.hidden=true;}
  static async create(canvas:HTMLCanvasElement):Promise<WorldRenderer>{
    const pixel=await PixelRenderer.create(canvas,768,768),other=document.createElement('canvas');
    other.hidden=true;other.tabIndex=0;other.setAttribute('aria-label',canvas.getAttribute('aria-label')!);canvas.after(other);
    try{return new WorldRenderer(pixel,await DungeonRenderer.create(other));}catch(error){pixel.dispose();other.remove();throw error;}
  }
  get canvas():HTMLCanvasElement{return this.active.canvas;}
  get width():number{return this.active.width;}
  get height():number{return this.active.height;}
  present(snapshot:Snapshot):void {
    const next=isDungeon(snapshot.envelope.frame.observation_center.level)?this.dungeon:this.pixel;
    if(next!==this.active){
      const focused=document.activeElement===this.active.canvas;
      this.active.clear();this.active.canvas.hidden=true;this.active.canvas.removeAttribute('id');this.active=next;
      this.active.canvas.id='world-canvas';this.active.canvas.hidden=false;if(focused)this.active.canvas.focus();
    }
    this.active.present(snapshot);
  }
  presentWalk(walk:WalkPresentation):void{this.active.presentWalk(walk);}
  pointer(x:number,y:number):{coordinate:Coord}|null{return this.active.pointer(x,y);}
  pickActor(x:number,y:number):string|null{return this.active.pickActor(x,y);}
  clear():void{this.pixel.clear();this.dungeon.clear();}
  dispose():void{this.pixel.dispose();this.dungeon.dispose();}
}
