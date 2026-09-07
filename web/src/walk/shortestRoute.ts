import type { Cell } from "./layoutPassability";
const key = (cell: Cell): string => `${cell.i},${cell.j}`;
const neighbours = [[-1,-1],[0,-1],[1,-1],[-1,0],[1,0],[-1,1],[0,1],[1,1]] as const;
const distance = (a: Cell, b: Cell): number => (a.i - b.i) ** 2 + (a.j - b.j) ** 2;

/** Geometry-only search. The caller supplies its evidence and step allowance. */
export function shortestRoute(from: Cell, to: Cell, maxSteps: number, canStep: (a: Cell, b: Cell) => boolean): Cell[] | null {
  if (key(from) === key(to) || Math.max(Math.abs(to.i-from.i), Math.abs(to.j-from.j)) > maxSteps) return null;
  const queue: Cell[][] = [[{ ...from }]], seen = new Set([key(from)]);
  for (let head = 0; head < queue.length; head++) {
    const route = queue[head]!;
    if (route.length - 1 >= maxSteps) continue;
    const current = route[route.length - 1]!;
    const ordered = neighbours.map(([i,j]) => ({ i: current.i+i, j: current.j+j }))
      .sort((a,b) => distance(a,to) - distance(b,to));
    for (const next of ordered) {
      if (seen.has(key(next)) || !canStep(current,next)) continue;
      const candidate = [...route,next];
      if (key(next) === key(to)) return candidate;
      seen.add(key(next)); queue.push(candidate);
    }
  }
  return null;
}
