import { describe, expect, it } from "vitest";
import { Group, Mesh, Texture } from "three";
import { addGround } from "../src/space/ground";
import { createTerrainSurface } from "../src/terrainSurface";
import { paletteFor } from "../src/space/palette";
import { createFeelCamera } from "../src/camera";
import { passabilityFrom, cellKey } from "../src/walk/layoutPassability";
import type { FeelSpace } from "../src/feelTypes";
import type { SpaceSceneOptions } from "../src/space/SpaceScene";
import type { DecodedTexture } from "../src/space/textures";

// A stair opening must remain absent through both ground render paths, while
// its explicit cell continues to refuse routes. No transparent swatch supplies it.
describe("authored floor openings", () => {
  it.each([false, true])("omits the opaque floor with coastal=%s", (coastal) => {
    const space: FeelSpace = {
      grid_extents: { i: 3, j: 1 }, weather: coastal,
      cells: [
        { i: 0, j: 0, material: coastal ? "water" : "stone", walkable: false },
        { i: 1, j: 0, material: "stone", walkable: true },
        { i: 2, j: 0, material: "void", walkable: false },
      ],
      structures: [], wall_runs: [], roofs: [], props: [], fixtures: [], portals: [],
      light_sources: { lantern_glass: null, candles: [] },
    };
    const textures = new Map<string, DecodedTexture>();
    for (const name of ["water", "stone", "grass", "earth"]) {
      textures.set(`terrain/${name}`, { texture: new Texture(), width: 1, height: 1, pixels: null });
    }
    const options = { space, textures, presets: [], anisotropy: 1, camera: createFeelCamera(1280, 800, { i: 1, j: 0 }) } as unknown as SpaceSceneOptions;
    const group = new Group();
    addGround(group, options, paletteFor([], coastal), { value: 0 }, createTerrainSurface(space));
    expect(group.getObjectByName("Ground_void")).toBeUndefined();
    const floor = group.getObjectByName("Ground_stone") as Mesh;
    const positions = floor.geometry.getAttribute("position");
    for (let n = 0; n < positions.count; n++) expect(positions.getX(n)).toBeLessThanOrEqual(1.5);
    group.traverse(object => {
      if (!(object instanceof Mesh) || object.name !== "GroundShadowReceiver") return;
      const shadowPositions = object.geometry.getAttribute("position");
      for (let n = 0; n < shadowPositions.count; n++) {
        expect(shadowPositions.getX(n)).toBeLessThanOrEqual(1.5);
      }
    });
    const occupancy = passabilityFrom(space);
    expect(occupancy.blocked.has(cellKey({ i: 2, j: 0 }))).toBe(true);
  });
});
