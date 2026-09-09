import type {Frame,Coord} from '../authoritative/state';
import type {WalkPresentation} from './pathControls';
import {pixelCellBounds,pixelGridEdges,projectPixel,type PixelProjection} from './pixelGeometry';

/** Small, discardable Canvas preparations; camera travel never rerasterizes them. */
export class PixelOverlays {
 readonly ink=document.createElement('canvas');
 readonly floor=document.createElement('canvas');
 origin:Coord={x:0,y:0};width=1;height=1;
 private signature='';
 private readonly contacts=new Map<string,HTMLCanvasElement>();
 private readonly labels=new Map<string,HTMLCanvasElement>();
 prepare(frame:Frame,walk:WalkPresentation|null,projection:PixelProjection,level:string,scale:number):boolean {
  const tiles=frame.tiles;
  const signature=JSON.stringify([level,scale,projection,tiles.map(t=>[t.position,t.passable,!!t.transition]),walk?.hover,walk?.cursor,walk?.kind,walk?.route]);
  if(signature===this.signature)return false;this.signature=signature;
  const bounds=tiles.map(t=>pixelCellBounds(t.position,projection));
  const left=Math.floor(bounds.length?Math.min(...bounds.map(b=>b.left)):0)-1,top=Math.floor(bounds.length?Math.min(...bounds.map(b=>b.top)):0)-1;
  const right=Math.ceil(Math.max(1,...bounds.map(b=>b.right)))+1,bottom=Math.ceil(Math.max(1,...bounds.map(b=>b.bottom)))+1;
  this.origin={x:left,y:top};this.width=right-left;this.height=bottom-top;
  this.ink.width=this.width;this.ink.height=this.height;
  const ink=this.ink.getContext('2d')!;ink.translate(-left,-top);
  const square=(c:CanvasRenderingContext2D,cell:Coord,colour:string,fill=false)=>{
   const b=pixelCellBounds(cell,projection),x=Math.floor(b.left)+.5,y=Math.floor(b.top)+.5;
   c.strokeStyle=colour;c.fillStyle=colour;c.lineWidth=1;
   if(fill)c.fillRect(x,y,projection.step.x,projection.step.y);else c.strokeRect(x,y,projection.step.x,projection.step.y);
  };
  ink.strokeStyle=level==='temple'?'#352b255c':'#bcaa8355';ink.lineWidth=1;ink.beginPath();
  for(const [a,b] of pixelGridEdges(tiles,projection)){ink.moveTo(Math.floor(a.x)+.5,Math.floor(a.y)+.5);ink.lineTo(Math.floor(b.x)+.5,Math.floor(b.y)+.5);}ink.stroke();
  const hover=walk?.hover;
  if(hover&&tiles.some(t=>t.passable===true&&t.position.x===hover.x&&t.position.y===hover.y))square(ink,hover,walk?.cursor==='refused'?'#cb866dcc':'#f0d09bc0');
  for(const tile of tiles)if(tile.transition)square(ink,tile.position,'#cba970a0');
  for(const [index,cell] of (walk?.route??[]).entries())if(index){
   const at={x:cell.i,y:cell.j};square(ink,at,walk?.kind==='committed'?'#dfae6888':'#f7dc9fc0');
   const p=projectPixel(at,projection);ink.fillStyle='#ffe1a5';ink.fillRect(Math.round(p.x-5),Math.round(p.y-2),3,5);ink.fillRect(Math.round(p.x+2),Math.round(p.y+1),3,5);
  }
  if(level!=='temple'&&level!=='arrival'){
   this.floor.width=this.width*scale;this.floor.height=this.height*scale;
   const c=this.floor.getContext('2d')!;c.setTransform(scale,0,0,scale,-left*scale,-top*scale);
   for(const tile of tiles)square(c,tile.position,tile.passable?'#514d43':'#242d31',true);
  }
  return true;
 }
 contact(observer:boolean,scale:number):HTMLCanvasElement {
  const key=`${observer}:${scale}`;let image=this.contacts.get(key);if(image)return image;
  image=document.createElement('canvas');image.width=40*scale;image.height=20*scale;
  const c=image.getContext('2d')!;c.scale(scale,scale);c.fillStyle='#12121866';c.beginPath();c.ellipse(20,10,12,4,0,0,Math.PI*2);c.fill();
  if(observer){c.strokeStyle='#e4c085';c.beginPath();c.ellipse(20,10,14,5,0,0,Math.PI*2);c.stroke();}
  this.contacts.set(key,image);return image;
 }
 label(text:string,font:string,colour:string,scale:number):HTMLCanvasElement {
  const key=JSON.stringify([text,font,colour,scale]);let image=this.labels.get(key);if(image)return image;
  image=document.createElement('canvas');const c=image.getContext('2d')!;c.font=font;
  const width=Math.ceil(c.measureText(text).width/2)*2+4;image.width=width*scale;image.height=24*scale;
  c.scale(scale,scale);c.font=font;c.fillStyle=colour;c.textAlign='center';c.fillText(text,width/2,18);
  this.labels.set(key,image);return image;
 }
 dispose():void {this.contacts.clear();this.labels.clear();}
}
