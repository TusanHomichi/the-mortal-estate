import type { Frame, Coord } from "../authoritative/state";
import { shortestRoute } from "../walk/shortestRoute";
import type { Cell } from "../walk/layoutPassability";
const cellKey = (cell: Cell): string => `${cell.i},${cell.j}`;

const headings: Record<string, string> = { "0,-1": "north", "1,-1": "northeast", "1,0": "east",
  "1,1": "southeast", "0,1": "south", "-1,1": "southwest", "-1,0": "west", "-1,-1": "northwest" };
export const asCell = (p: Coord): Cell => ({ i: p.x, j: p.y });
export function routeDirections(route: readonly Cell[]): string[] {
  return route.slice(1).map((cell, index) => headings[`${cell.i - route[index]!.i},${cell.j - route[index]!.j}`]!);
}
/** Proposal from observed terrain only. Rust preview owns costs and outcomes. */
export function proposePath(frame: Frame, target: Coord): Cell[] | null {
  const tiles = new Map(frame.tiles.map(tile => [cellKey(asCell(tile.position)), tile]));
  const open = (p: Cell) => tiles.get(cellKey(p))?.passable === true;
  const from = asCell(frame.observation_center.position);
  const continuesHere = (cell: Cell): boolean => {
    const transition = tiles.get(cellKey(cell))?.transition as {
      navigation?: string; door_open?: boolean; target?: {realm:string;level:string;position:Coord}
    } | undefined;
    if (transition == null) return true;
    const target = transition.target, site = frame.observation_center;
    return transition.navigation === "door" && transition.door_open === true &&
      target?.realm === site.realm && target.level === site.level &&
      target.position.x === cell.i && target.position.y === cell.j;
  };
  // Three is the wire request's bound, not a client movement budget. The Rust
  // codec validates every request and the server assesses the proposed route.
  return shortestRoute(from, asCell(target), 3, (a, b) => {
    if (!open(b) || (cellKey(a) !== cellKey(from) && !continuesHere(a))) return false;
    return a.i === b.i || a.j === b.j || (open({ i:a.i,j:b.j }) && open({ i:b.i,j:a.j }));
  });
}
