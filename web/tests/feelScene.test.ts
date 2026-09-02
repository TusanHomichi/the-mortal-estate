import { describe, expect, it } from "vitest";
import { Mesh } from "three";
import { applyPropPlacementMirror } from "../src/space/SpaceScene";
import type { PropPlacement } from "../src/feelTypes";

function placement(mirror: boolean): PropPlacement {
  return {
    kind: "tree_broad",
    cell_anchor: [0, 0],
    nominal_height: 2,
    sway: true,
    mirror,
  };
}

describe("feel-scene prop placement", () => {
  it("gives a mirrored placement negative x scale", () => {
    const mesh = new Mesh();

    applyPropPlacementMirror(mesh, placement(true));

    expect(mesh.scale.x).toBe(-1);
  });
});
