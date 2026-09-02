import type { FeelSpace } from "./feelTypes";

export const GRASS_CLUMP_HEIGHT = 0.32;
export const MAX_GRASS_CLUMPS = 1_800;

export interface GrassClumpPlacement {
  x: number;
  z: number;
  scale: number;
  mirror: boolean;
}

function seededRandom(seed = 0x47524153): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function withinTwoTiles(
  i: number,
  j: number,
  features: readonly { i: number; j: number }[],
): boolean {
  return features.some((feature) =>
    Math.max(Math.abs(feature.i - i), Math.abs(feature.j - j)) <= 2
  );
}

/** Deterministic presentation-only scatter; clumps never enter passability. */
export function scatterGrassClumps(
  space: FeelSpace,
  limit = MAX_GRASS_CLUMPS,
): GrassClumpPlacement[] {
  if (limit <= 0 || !space.weather) return [];
  const features = [
    ...space.cells
      .filter((cell) => cell.material !== "grass")
      .map(({ i, j }) => ({ i, j })),
    ...space.props.map((prop) => ({ i: prop.cell_anchor[0], j: prop.cell_anchor[1] })),
  ];
  const random = seededRandom();
  const clumps: GrassClumpPlacement[] = [];
  for (const cell of space.cells) {
    if (cell.material !== "grass") continue;
    const nearby = withinTwoTiles(cell.i, cell.j, features);
    const count = nearby
      ? 3 + Math.floor(random() * 3)
      : 1 + Math.floor(random() * 2);
    for (let index = 0; index < count && clumps.length < limit; index += 1) {
      clumps.push({
        x: cell.i + (random() * 2 - 1) * 0.38,
        z: cell.j + (random() * 2 - 1) * 0.38,
        scale: 0.8 + random() * 0.4,
        mirror: random() < 0.5,
      });
    }
    if (clumps.length === limit) break;
  }
  return clumps;
}
