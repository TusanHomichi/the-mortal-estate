import type { Coord } from "../authoritative/state";

/** Presentation distance only; positions and permission still come from frames. */
export function pixelRouteLength(route: readonly Coord[]): number {
  return route.slice(1).reduce((sum,to,index)=>sum+Math.hypot(to.x-route[index]!.x,to.y-route[index]!.y),0);
}
export function pixelTravelDuration(route: readonly Coord[], remaining = 2500): number {
  return Math.max(0,Math.min(pixelRouteLength(route)*560,2500,remaining));
}
export function samplePixelTravel(route: readonly Coord[], elapsed: number, duration: number) {
  const total=pixelRouteLength(route), fraction=duration>0 ? Math.min(1,Math.max(0,elapsed/duration)) : 1;
  const distance=total*fraction;
  let remaining=distance;
  for (let i=1;i<route.length;i++) {
    const from=route[i-1]!,to=route[i]!,length=Math.hypot(to.x-from.x,to.y-from.y);
    if (length>0 && (remaining<length || i===route.length-1)) {
      const t=Math.min(1,remaining/length);
      return {at:{x:from.x+(to.x-from.x)*t,y:from.y+(to.y-from.y)*t},from,to,distance,done:fraction===1};
    }
    remaining-=length;
  }
  const at=route.at(-1)!;
  return {at:{...at},from:at,to:at,distance,done:true};
}
/** One gait cycle per room unit: animation cannot run independently of travel. */
export function pixelStrideFrame(distance: number, length: number): number {
  return Math.floor(Math.max(0,distance)*length)%length;
}
