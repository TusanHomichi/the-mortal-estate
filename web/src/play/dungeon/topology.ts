import geography from '../../../../content/lands/first-expedition/generated/workbench_projection.json';
import type {Coord, Frame} from '../../authoritative/state';
import {cellKey, isWall, doorState, NORTH_HEIGHT, WALL_HEIGHT} from './view';
export interface WallPart { id:string; tile:Coord; axis:'x'|'y'; plane:number; start:number; end:number; height:number; door:boolean; open:boolean }
/** Authored terrain supplies orientation only. It never grants visibility or passability. */
export function wallParts(frame:Frame):WallPart[] {
  const member=geography.members.find(m=>m.member===frame.observation_center.level);
  if(!member)throw Error('Unknown dungeon member.');
  const architecture=new Set(member.cells.filter(c=>c.terrain.some(t=>isWall(t.class)||t.class.includes('doorway'))).map(cellKey));
  const solid=(x:number,y:number)=>architecture.has(`${x}:${y}`);
  const center=frame.observation_center.position, parts:WallPart[]=[];
  for(const tile of frame.tiles){
    const p=tile.position,door=doorState(tile)?.navigation==='door';
    if(!isWall(tile.terrain_id)&&!door)continue;
    const horizontal=solid(p.x-1,p.y)||solid(p.x+1,p.y);
    const vertical=solid(p.x,p.y-1)||solid(p.x,p.y+1);
    // A door has one fixed hinge axis, even at the edge of an observation window.
    const axes:('x'|'y')[]=door?[horizontal?'x':'y']:horizontal&&vertical?['x','y']:horizontal?['x']:vertical?['y']:['x'];
    for(const axis of axes){
      const plane=axis==='x'?p.y+.5:p.x+(p.x<center.x?.5:-.5);
      let start=p[axis]-.5,end=p[axis]+.5;
      if(axis==='x'){
        // Meet the adjoining receding edge at corners, without crossing its cell.
        if(!solid(p.x-1,p.y)&&(solid(p.x,p.y-1)||solid(p.x,p.y+1)))start=p.x+(p.x<center.x?.5:-.5);
        if(!solid(p.x+1,p.y)&&(solid(p.x,p.y-1)||solid(p.x,p.y+1)))end=p.x+(p.x<center.x?.5:-.5);
      }
      if(end-start<.01)continue;
      parts.push({id:`${frame.observation_center.level}:${cellKey(p)}:${axis}`,tile:p,axis,plane,start,end,
        height:axis==='x'&&p.y<center.y?NORTH_HEIGHT:WALL_HEIGHT,door,open:doorState(tile)?.door_open===true});
    }
  }
  return parts;
}
