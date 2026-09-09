import type { Coord } from "../authoritative/state";
import type { PixelPacket } from "./pixelPacket";
import type { PixelProjection } from "./pixelGeometry";

interface AlphaImage { image: HTMLImageElement; alpha: Uint8ClampedArray; width: number; height: number }
export interface ExteriorLayer {
  id: string; x: number; y: number; width: number; height: number; depth: number; source: AlphaImage;
}
export function exteriorLayerCovers(layer: ExteriorLayer, point: Coord): boolean {
  const u=(point.x-layer.x)/layer.width,v=(point.y-layer.y)/layer.height;
  if(u<0||v<0||u>=1||v>=1)return false;
  return layer.source.alpha[(Math.floor(v*layer.source.height)*layer.source.width+Math.floor(u*layer.source.width))*4+3]!>32;
}

/** Digest-bound scene pixels and silhouettes; observation owns every actor and target. */
export class PixelExteriorRenderer {
  private readonly scenery=new Map<string,AlphaImage>();
  constructor(private readonly packet: PixelPacket) {
    for(const asset of packet.manifest.exterior.foreground) {
      const image=packet.images.get(asset.file)!;
      const canvas=document.createElement("canvas");canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
      const c=canvas.getContext("2d")!;c.drawImage(image,0,0);
      const alpha=c.getImageData(0,0,canvas.width,canvas.height).data;
      let transparent=false,opaque=false;
      for(let i=3;i<alpha.length;i+=4){transparent ||= alpha[i]===0;opaque ||= alpha[i]!>32;}
      if(!transparent||!opaque)throw new Error("Exterior foreground requires real transparent artwork.");
      this.scenery.set(asset.file,{image,alpha,width:canvas.width,height:canvas.height});
    }
  }
  private offset(projection:PixelProjection):Coord {
    const source=this.packet.manifest.exterior.projection;
    return {x:projection.origin.x-source.origin.x,y:projection.origin.y-source.origin.y};
  }
  layers(projection:PixelProjection,width:number,height:number): ExteriorLayer[] {
    const offset=this.offset(projection);
    return this.packet.manifest.exterior.foreground.map(asset=>{
      const source=this.scenery.get(asset.file)!;
      return {id:asset.id,x:asset.x+offset.x,y:asset.y+offset.y,width:source.width,height:source.height,
        depth:asset.depth+offset.y,source};
    }).filter(r=>r.x+r.width>0&&r.y+r.height>0&&r.x<width&&r.y<height)
      .sort((a,b)=>a.depth-b.depth||a.id.localeCompare(b.id));
  }
}
