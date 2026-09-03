import { describe, expect, it } from "vitest";
import { Euler, Vector3 } from "three";
import { propCardTransform, WALL_CARD_OFFSET } from "../src/space/propCards";
import type { PropPlacement } from "../src/feelTypes";

function placement(mirror: boolean, facing: PropPlacement["facing"] = "view"): PropPlacement {
  return {
    kind: "tree_broad",
    cell_anchor: [0, 0],
    elevation: 0,
    card_height: 2,
    sway: true,
    mirror,
    facing,
  };
}

describe("feel-scene prop placement", () => {
  it("keeps mirror on the card's local horizontal axis", () => {
    expect(propCardTransform(placement(true, "+z")).scaleX).toBe(-1);
  });

  it("places a +z card just in front of its north wall without re-facing it", () => {
    const transform = propCardTransform({
      ...placement(false, "+z"),
      cell_anchor: [4, 1],
    });

    expect(transform.rotationY).toBe(0);
    expect(transform.position.x).toBe(4);
    expect(transform.position.z).toBe(1 - 0.5 + WALL_CARD_OFFSET);
  });

  it("adds the authored elevation to view-facing and wall-mounted card centres", () => {
    for (const facing of ["view", "+z", "+x"] as const) {
      const transform = propCardTransform({
        ...placement(false, facing),
        elevation: 1.25,
      });
      expect(transform.position.y).toBe(2.25);
    }
  });

  it("places a +x card just in front of its west wall with its shadow on the base line", () => {
    const transform = propCardTransform({
      ...placement(false, "+x"),
      cell_anchor: [2, 3],
    });

    expect(transform.rotationY).toBe(Math.PI / 2);
    expect(transform.position.x).toBe(2 - 0.5 + WALL_CARD_OFFSET);
    expect(transform.position.z).toBe(3);
    expect(transform.contactShadowRotation.y).toBe(Math.PI / 2);
    const shadowNormal = new Vector3(0, 0, 1).applyEuler(new Euler(
      transform.contactShadowRotation.x,
      transform.contactShadowRotation.y,
      transform.contactShadowRotation.z,
      transform.contactShadowRotation.order,
    ));
    expect(shadowNormal.x).toBeCloseTo(0);
    expect(shadowNormal.y).toBeCloseTo(1);
    expect(shadowNormal.z).toBeCloseTo(0);
  });
});
