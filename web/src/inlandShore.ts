import type { FeelSpace } from "./feelTypes";

type Point = [number, number];
const key = ([x, z]: Point) => `${x},${z}`;
const neighbours = ([x, z]: Point): Point[] => [[x + 1, z], [x, z + 1], [x - 1, z], [x, z - 1]];

/** Rounded scenic contours for enclosed water, derived from the existing cells.
 * Open coastline and every gameplay verdict retain their existing owners.
 */
export function createInlandShore(space: FeelSpace): (x: number, z: number) => number | null {
  const cells = new Map(space.cells.map(c => [`${c.i},${c.j}`, c]));
  const wet = new Set(space.cells.filter(c => ["water", "floor_planks"].includes(c.material)).map(c => `${c.i},${c.j}`));
  const ponds: { loops: Point[][]; bounds: number[] }[] = [];
  const openWater = new Set<string>();
  while (wet.size) {
    const start = wet.values().next().value!;
    wet.delete(start);
    const component: Point[] = [start.split(",").map(Number) as Point];
    let open = false;
    for (const cell of component) for (const next of neighbours(cell)) {
      if (!cells.has(key(next))) open = true;
      if (wet.delete(key(next))) component.push(next);
    }
    if (open) { component.forEach(p => openWater.add(key(p))); continue; }
    const members = new Set(component.map(key));
    const edges = new Map<string, Point[]>();
    for (const [x, z] of component) {
      const corners: Point[] = [[x - .5, z - .5], [x + .5, z - .5], [x + .5, z + .5], [x - .5, z + .5]];
      const adjacent: Point[] = [[x, z - 1], [x + 1, z], [x, z + 1], [x - 1, z]];
      for (let i = 0; i < 4; i++) if (!members.has(key(adjacent[i]!))) {
        const a = corners[i]!, b = corners[(i + 1) % 4]!;
        const outgoing = edges.get(key(a)) ?? []; outgoing.push(b); edges.set(key(a), outgoing);
      }
    }
    const loops: Point[][] = [];
    while (edges.size) {
      const first = edges.keys().next().value!, loop: Point[] = [];
      let at = first;
      do {
        const a = at.split(",").map(Number) as Point;
        loop.push(a);
        const choices = edges.get(at)!;
        // At a diagonal touch, follow the right-hand boundary of this cell.
        const previous = loop.at(-2);
        if (previous && choices.length > 1) choices.sort((b, c) => {
          const turn = (p: Point) => Math.atan2((a[0] - previous[0]) * (p[1] - a[1]) - (a[1] - previous[1]) * (p[0] - a[0]),
            (a[0] - previous[0]) * (p[0] - a[0]) + (a[1] - previous[1]) * (p[1] - a[1]));
          return turn(b) - turn(c);
        });
        at = key(choices.pop()!);
        if (!choices.length) edges.delete(key(a));
      } while (at !== first);
      let contour = loop.filter((p, i) => {
        const a = loop[(i + loop.length - 1) % loop.length]!, b = loop[(i + 1) % loop.length]!;
        return (p[0] - a[0]) * (b[1] - p[1]) !== (p[1] - a[1]) * (b[0] - p[0]);
      });
      for (let pass = 0; pass < 2; pass++) contour = contour.flatMap((a, i): Point[] => {
        const b = contour[(i + 1) % contour.length]!;
        return [[a[0] * .75 + b[0] * .25, a[1] * .75 + b[1] * .25],
          [a[0] * .25 + b[0] * .75, a[1] * .25 + b[1] * .75]];
      });
      loops.push(contour);
    }
    ponds.push({ loops, bounds: [Math.min(...component.map(p => p[0])) - 1,
      Math.max(...component.map(p => p[0])) + 1, Math.min(...component.map(p => p[1])) - 1, Math.max(...component.map(p => p[1])) + 1] });
  }
  return (x, z) => {
    if (openWater.has(key([Math.round(x), Math.round(z)]))) return null;
    let cover: number | null = null;
    for (const { loops, bounds: [left, right, top, bottom] } of ponds) {
      if (x < left! || x > right! || z < top! || z > bottom!) continue;
      const px = x + .10 * Math.sin(x * .8 + z * 1.1) + .035 * Math.sin(x * 7.1 + z * 4.8);
      const pz = z + .08 * Math.sin(x * 1.3 - z * .7) + .027 * Math.sin(z * 5.3 - x * 2.9);
      let inside = false, distance = Infinity;
      for (const loop of loops) for (let i = 0; i < loop.length; i++) {
        const a = loop[i]!, b = loop[(i + 1) % loop.length]!;
        if ((a[1] > pz) !== (b[1] > pz) && px < (b[0] - a[0]) * (pz - a[1]) / (b[1] - a[1]) + a[0]) inside = !inside;
        const dx = b[0] - a[0], dz = b[1] - a[1];
        const t = Math.max(0, Math.min(1, ((px - a[0]) * dx + (pz - a[1]) * dz) / (dx * dx + dz * dz)));
        distance = Math.min(distance, Math.hypot(px - a[0] - t * dx, pz - a[1] - t * dz));
      }
      const t = Math.max(0, Math.min(1, .5 + (inside ? -distance : distance) / .5));
      const land = t * t * (3 - 2 * t);
      cover = cover === null ? land : Math.min(cover, land);
    }
    return cover;
  };
}
