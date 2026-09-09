import type { Coord, Frame } from "../authoritative/state";

/** Art calibration only. It carries no passability or transition destinations. */
export interface PixelProjection { origin: Coord; step: Coord }
export const TEMPLE_PROJECTION: PixelProjection = { origin: { x: 76, y: 110 }, step: { x: 63, y: 44 } };
export function pixelSceneryRect(point: Coord, foot: Coord, width: number, aspect: number) {
  const height=width/aspect;
  return {x:point.x-foot.x*width,y:point.y-foot.y*height,width,height,depth:point.y};
}
export function projectPixel(cell: Coord, projection: PixelProjection): Coord {
  return { x: projection.origin.x + cell.x * projection.step.x, y: projection.origin.y + cell.y * projection.step.y };
}
export function unprojectPixel(point: Coord, projection: PixelProjection): Coord {
  return { x: Math.round((point.x - projection.origin.x) / projection.step.x),
    y: Math.round((point.y - projection.origin.y) / projection.step.y) };
}
export function pixelCellBounds(cell: Coord, projection: PixelProjection) {
  const center = projectPixel(cell, projection);
  return { left: center.x-projection.step.x/2, top: center.y-projection.step.y/2,
    right: center.x+projection.step.x/2, bottom: center.y+projection.step.y/2 };
}
/** Observed passability supplies the grid. Shared edges are drawn just once. */
export function pixelGridEdges(tiles: Frame["tiles"], projection: PixelProjection): [Coord, Coord][] {
  const edges = new Map<string, [Coord, Coord]>();
  for (const tile of tiles) {
    if (tile.passable !== true) continue;
    const { left, top, right, bottom } = pixelCellBounds(tile.position, projection);
    const tl={x:left,y:top}, tr={x:right,y:top}, bl={x:left,y:bottom}, br={x:right,y:bottom};
    for (const [a,b] of [[tl,tr],[tl,bl],[tr,br],[bl,br]] as [Coord,Coord][]) edges.set(`${a.x}:${a.y}:${b.x}:${b.y}`,[a,b]);
  }
  return [...edges.values()];
}
export function pixelDirection(from: Coord, to: Coord, diagonals = false): string {
  const dx = to.x - from.x, dy = to.y - from.y;
  if (diagonals && Math.abs(dx)>.01 && Math.abs(dy)>.01) return `${dy<0 ? "north" : "south"}-${dx<0 ? "west" : "east"}`;
  return Math.abs(dx) > Math.abs(dy) ? dx > 0 ? "east" : "west" : dy < 0 ? "north" : "south";
}

/** Different source padding and poses cannot move the displayed ground anchor. */
export function pixelSpriteRect(point: Coord, frame: { body_height: number; foot: Coord }, size: number, height: number) {
  const scale=height/frame.body_height;
  return {x:Math.round(point.x-frame.foot.x*scale),y:Math.round(point.y-frame.foot.y*scale),size:Math.round(size*scale),scale};
}
