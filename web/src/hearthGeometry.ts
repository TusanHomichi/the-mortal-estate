import type { FixturePlacement } from "./feelTypes";
import { WALL_PROFILE } from "./wallGeometry";
import type { GeometryData } from "./wallGeometry";

export const HEARTH_PROFILE = Object.freeze({
  breastWidth: 1,
  breastDepth: 0.42,
  fireboxWidth: 0.56,
  fireboxHeight: 0.62,
  fireboxSill: 0.08,
  fireboxRecess: 0.3,
  mantelWidth: 1.16,
  mantelHeight: 0.1,
  mantelDepth: 0.16,
  mantelUnderside: 1.02,
  mantelOverhang: 0.06,
  hearthstoneWidth: 1.16,
  hearthstoneHeight: 0.05,
  hearthstoneDepth: 0.32,
  fireHeight: 0.5,
  fireFrontInset: 0.1,
  lightHeight: 0.55,
});

export type HearthMaterial = "fieldstone" | "fieldstone_dark" | "post";

export interface HearthGeometryPart {
  label: "breast" | "firebox" | "mantel" | "hearthstone";
  fixtureIndex: number;
  material: HearthMaterial;
  geometry: GeometryData;
}

export interface HearthAnchor {
  position: [number, number, number];
  lateral: [number, number, number];
}

type Vertex = [number, number, number];

function emptyGeometry(): GeometryData {
  return { positions: [], uvs: [], indices: [] };
}

function pushQuad(
  geometry: GeometryData,
  vertices: readonly [Vertex, Vertex, Vertex, Vertex],
  uLength: number,
  vLength: number,
): void {
  const offset = geometry.positions.length / 3;
  for (const vertex of vertices) geometry.positions.push(...vertex);
  geometry.uvs.push(0, 0, uLength, 0, uLength, vLength, 0, vLength);
  geometry.indices.push(offset, offset + 1, offset + 2, offset, offset + 2, offset + 3);
}

function localToWorld(
  fixture: FixturePlacement,
  lateral: number,
  y: number,
  depth: number,
): Vertex {
  const [i, j] = fixture.cell;
  return fixture.against === "north"
    ? [i + lateral, y, j - 0.5 + depth]
    : [i - 0.5 + depth, y, j - lateral];
}

function cuboid(
  fixture: FixturePlacement,
  lateral0: number,
  lateral1: number,
  y0: number,
  y1: number,
  depth0: number,
  depth1: number,
): GeometryData {
  const geometry = emptyGeometry();
  const toWorld = (lateral: number, y: number, depth: number): Vertex =>
    localToWorld(fixture, lateral, y, depth);
  const width = lateral1 - lateral0;
  const height = y1 - y0;
  const depth = depth1 - depth0;
  pushQuad(geometry, [toWorld(lateral0, y0, depth1), toWorld(lateral1, y0, depth1), toWorld(lateral1, y1, depth1), toWorld(lateral0, y1, depth1)], width, height);
  pushQuad(geometry, [toWorld(lateral1, y0, depth0), toWorld(lateral0, y0, depth0), toWorld(lateral0, y1, depth0), toWorld(lateral1, y1, depth0)], width, height);
  pushQuad(geometry, [toWorld(lateral1, y0, depth1), toWorld(lateral1, y0, depth0), toWorld(lateral1, y1, depth0), toWorld(lateral1, y1, depth1)], depth, height);
  pushQuad(geometry, [toWorld(lateral0, y0, depth0), toWorld(lateral0, y0, depth1), toWorld(lateral0, y1, depth1), toWorld(lateral0, y1, depth0)], depth, height);
  pushQuad(geometry, [toWorld(lateral0, y1, depth1), toWorld(lateral1, y1, depth1), toWorld(lateral1, y1, depth0), toWorld(lateral0, y1, depth0)], width, depth);
  pushQuad(geometry, [toWorld(lateral0, y0, depth0), toWorld(lateral1, y0, depth0), toWorld(lateral1, y0, depth1), toWorld(lateral0, y0, depth1)], width, depth);
  return geometry;
}

function breastGeometry(fixture: FixturePlacement): GeometryData {
  const geometry = emptyGeometry();
  const halfWidth = HEARTH_PROFILE.breastWidth / 2;
  const halfOpening = HEARTH_PROFILE.fireboxWidth / 2;
  const openingTop = HEARTH_PROFILE.fireboxSill + HEARTH_PROFILE.fireboxHeight;
  const front = HEARTH_PROFILE.breastDepth;
  const top = WALL_PROFILE.capTop;
  const toWorld = (lateral: number, y: number, depth: number): Vertex =>
    localToWorld(fixture, lateral, y, depth);

  // The four front pieces leave a literal hole. A dark card painted over a
  // cuboid would look flat again, which is the entire sin this fixture fixes.
  pushQuad(geometry, [toWorld(-halfWidth, 0, front), toWorld(-halfOpening, 0, front), toWorld(-halfOpening, top, front), toWorld(-halfWidth, top, front)], halfWidth - halfOpening, top);
  pushQuad(geometry, [toWorld(halfOpening, 0, front), toWorld(halfWidth, 0, front), toWorld(halfWidth, top, front), toWorld(halfOpening, top, front)], halfWidth - halfOpening, top);
  pushQuad(geometry, [toWorld(-halfOpening, openingTop, front), toWorld(halfOpening, openingTop, front), toWorld(halfOpening, top, front), toWorld(-halfOpening, top, front)], HEARTH_PROFILE.fireboxWidth, top - openingTop);
  pushQuad(geometry, [toWorld(-halfOpening, 0, front), toWorld(halfOpening, 0, front), toWorld(halfOpening, HEARTH_PROFILE.fireboxSill, front), toWorld(-halfOpening, HEARTH_PROFILE.fireboxSill, front)], HEARTH_PROFILE.fireboxWidth, HEARTH_PROFILE.fireboxSill);

  // Outer back, sides, cap, and floor retain the breast's cuboid extent. UVs
  // are world-unit lengths so the square fieldstone swatch tiles naturally.
  pushQuad(geometry, [toWorld(halfWidth, 0, 0), toWorld(-halfWidth, 0, 0), toWorld(-halfWidth, top, 0), toWorld(halfWidth, top, 0)], HEARTH_PROFILE.breastWidth, top);
  pushQuad(geometry, [toWorld(halfWidth, 0, front), toWorld(halfWidth, 0, 0), toWorld(halfWidth, top, 0), toWorld(halfWidth, top, front)], front, top);
  pushQuad(geometry, [toWorld(-halfWidth, 0, 0), toWorld(-halfWidth, 0, front), toWorld(-halfWidth, top, front), toWorld(-halfWidth, top, 0)], front, top);
  pushQuad(geometry, [toWorld(-halfWidth, top, front), toWorld(halfWidth, top, front), toWorld(halfWidth, top, 0), toWorld(-halfWidth, top, 0)], HEARTH_PROFILE.breastWidth, front);
  pushQuad(geometry, [toWorld(-halfWidth, 0, 0), toWorld(halfWidth, 0, 0), toWorld(halfWidth, 0, front), toWorld(-halfWidth, 0, front)], HEARTH_PROFILE.breastWidth, front);
  return geometry;
}

function fireboxGeometry(fixture: FixturePlacement): GeometryData {
  const geometry = emptyGeometry();
  const halfOpening = HEARTH_PROFILE.fireboxWidth / 2;
  const y0 = HEARTH_PROFILE.fireboxSill;
  const y1 = y0 + HEARTH_PROFILE.fireboxHeight;
  const front = HEARTH_PROFILE.breastDepth;
  const back = front - HEARTH_PROFILE.fireboxRecess;
  const toWorld = (lateral: number, y: number, depth: number): Vertex =>
    localToWorld(fixture, lateral, y, depth);

  pushQuad(geometry, [toWorld(-halfOpening, y0, back), toWorld(halfOpening, y0, back), toWorld(halfOpening, y1, back), toWorld(-halfOpening, y1, back)], HEARTH_PROFILE.fireboxWidth, HEARTH_PROFILE.fireboxHeight);
  pushQuad(geometry, [toWorld(-halfOpening, y0, back), toWorld(-halfOpening, y0, front), toWorld(-halfOpening, y1, front), toWorld(-halfOpening, y1, back)], HEARTH_PROFILE.fireboxRecess, HEARTH_PROFILE.fireboxHeight);
  pushQuad(geometry, [toWorld(halfOpening, y0, front), toWorld(halfOpening, y0, back), toWorld(halfOpening, y1, back), toWorld(halfOpening, y1, front)], HEARTH_PROFILE.fireboxRecess, HEARTH_PROFILE.fireboxHeight);
  pushQuad(geometry, [toWorld(-halfOpening, y1, back), toWorld(halfOpening, y1, back), toWorld(halfOpening, y1, front), toWorld(-halfOpening, y1, front)], HEARTH_PROFILE.fireboxWidth, HEARTH_PROFILE.fireboxRecess);
  return geometry;
}

export function buildHearthGeometry(
  fixtures: readonly FixturePlacement[],
): HearthGeometryPart[] {
  return fixtures.flatMap((fixture, fixtureIndex): HearthGeometryPart[] => {
    const front = HEARTH_PROFILE.breastDepth;
    const mantelDepth0 = front + HEARTH_PROFILE.mantelOverhang - HEARTH_PROFILE.mantelDepth;
    return [
      { label: "breast", fixtureIndex, material: "fieldstone", geometry: breastGeometry(fixture) },
      { label: "firebox", fixtureIndex, material: "fieldstone_dark", geometry: fireboxGeometry(fixture) },
      {
        label: "mantel",
        fixtureIndex,
        material: "post",
        geometry: cuboid(
          fixture,
          -HEARTH_PROFILE.mantelWidth / 2,
          HEARTH_PROFILE.mantelWidth / 2,
          HEARTH_PROFILE.mantelUnderside,
          HEARTH_PROFILE.mantelUnderside + HEARTH_PROFILE.mantelHeight,
          mantelDepth0,
          front + HEARTH_PROFILE.mantelOverhang,
        ),
      },
      {
        label: "hearthstone",
        fixtureIndex,
        material: "fieldstone",
        geometry: cuboid(
          fixture,
          -HEARTH_PROFILE.hearthstoneWidth / 2,
          HEARTH_PROFILE.hearthstoneWidth / 2,
          0,
          HEARTH_PROFILE.hearthstoneHeight,
          front,
          front + HEARTH_PROFILE.hearthstoneDepth,
        ),
      },
    ];
  });
}

export function hearthFireAnchor(fixture: FixturePlacement): HearthAnchor {
  return {
    position: localToWorld(
      fixture,
      0,
      HEARTH_PROFILE.fireboxSill + HEARTH_PROFILE.fireHeight / 2,
      HEARTH_PROFILE.breastDepth - HEARTH_PROFILE.fireFrontInset,
    ),
    lateral: fixture.against === "north" ? [1, 0, 0] : [0, 0, -1],
  };
}

export function hearthLightPosition(fixture: FixturePlacement): [number, number, number] {
  return localToWorld(
    fixture,
    0,
    HEARTH_PROFILE.lightHeight,
    HEARTH_PROFILE.breastDepth - HEARTH_PROFILE.fireFrontInset,
  );
}
