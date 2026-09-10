import type {Coord, Frame} from '../../authoritative/state';

export const DUNGEON_LEVELS = ['d1_entry', 'd2', 'd3', 'd4'] as const;
export const TILE = 1.65;
export const WALL_HEIGHT = TILE * Math.tan(Math.PI / 3);
export const NORTH_HEIGHT = 2.285;
export const isDungeon = (level: string): boolean => (DUNGEON_LEVELS as readonly string[]).includes(level);
export const cellKey = (p: Coord): string => `${p.x}:${p.y}`;
export const withinView = (center: Coord, p: Coord): boolean => Math.abs(center.x-p.x)<=3 && Math.abs(center.y-p.y)<=3;
export const isWall = (id: string | undefined): boolean => id === 'expedition_wall' || id === 'expedition_masonry';
export interface DoorState { navigation?: string; door_open?: boolean }
export const doorState = (tile: Frame['tiles'][number]): DoorState | undefined => tile.transition as DoorState | undefined;
/** A draw selection of actual observer rows, including no unexplored placeholders. */
export function observedDungeon(frame: Frame): Frame {
  const inside=(p:Coord)=>withinView(frame.observation_center.position,p);
  const tiles=frame.tiles.filter(t=>!!t.terrain_id && inside(t.position));
  const seen=new Set(tiles.map(t=>cellKey(t.position)));
  const visible=(p:Coord)=>seen.has(cellKey(p));
  return {...frame,tiles,actors:frame.actors.filter(a=>a.life_state!=='dead'&&visible(a.position.position)),
    corpses:frame.corpses.filter(a=>visible(a.location.position)),ground_items:frame.ground_items.filter(a=>visible(a.location.position)),
    gold_piles:frame.gold_piles.filter(a=>visible(a.location.position))};
}
/** Stable sorting; a lone body is exactly at the cell centre. */
export function occupantAnchors(frame: Frame): Map<string,Coord> {
  const groups=new Map<string,typeof frame.actors>();
  for(const actor of frame.actors){if(actor.life_state==='dead')continue;const key=cellKey(actor.position.position);const group=groups.get(key)??[];group.push(actor);groups.set(key,group);}
  const result=new Map<string,Coord>();
  for(const actors of groups.values()) {
    actors.sort((a,b)=>a.actor_id.localeCompare(b.actor_id));
    actors.forEach((actor,i)=>{const p=actor.position.position,n=actors.length;
      const angle=i/n*Math.PI*2;
      result.set(actor.actor_id,{x:p.x+(n===1?0:Math.cos(angle)*.26),y:p.y+(n===1?0:Math.sin(angle)*.26)});
    });
  }
  return result;
}
