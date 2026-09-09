import type { Coord } from "../authoritative/state";
import type { PixelProjection } from "./pixelGeometry";

/** Integer CSS-pixel enlargement; spare fractional pixels become outer margins. */
export function pixelViewport(width: number, height: number) {
  const availableWidth=Math.max(1,Math.floor(width)),availableHeight=Math.max(1,Math.floor(height));
  const scale=Math.max(1,Math.min(Math.floor(availableWidth/640),Math.floor(availableHeight/360)));
  return {width:Math.floor(availableWidth/scale),height:Math.floor(availableHeight/scale),scale};
}

/** Camera translation only: the scene keeps its authored, uniform cell projection. */
export function pixelCamera(projection: PixelProjection, focus: Coord, viewport: Coord, scene: Coord): PixelProjection {
  const target={x:projection.origin.x+focus.x*projection.step.x,y:projection.origin.y+focus.y*projection.step.y};
  const offset=(position:number,extent:number,view:number)=>extent<=view ? Math.round((view-extent)/2)
    : -Math.max(0,Math.min(extent-view,Math.round(position-view*.6)));
  return {origin:{x:projection.origin.x+offset(target.x,scene.x,viewport.x),
    y:projection.origin.y+offset(target.y,scene.y,viewport.y)},step:{...projection.step}};
}
