import { describe, expect, it } from "vitest";
import type { PropPlacement } from "../src/feelTypes";
import { FLOOR_CARD_LIFT, propCardTransform } from "../src/space/propCards";

const placement = (facing: PropPlacement["facing"]): PropPlacement => ({
  kind: "rug",
  cell_anchor: [4, 7],
  elevation: 0,
  card_height: 1.2,
  sway: false,
  mirror: false,
  facing,
});

describe("card transforms", () => {
  it("centres an upright card half its card height above its elevation", () => {
    const transform = propCardTransform(placement("view"));
    expect(transform.position).toEqual({ x: 4, y: 0.6, z: 7 });
    expect(transform.rotationX).toBe(0);
  });

  it("lays a floor card flat on its cell, just above the ground, up toward north", () => {
    const transform = propCardTransform(placement("floor"));
    expect(transform.position).toEqual({ x: 4, y: FLOOR_CARD_LIFT, z: 7 });
    expect(transform.rotationX).toBeCloseTo(-Math.PI / 2);
    expect(transform.rotationY).toBe(0);
    expect(transform.scaleX).toBe(1);
  });

  it("keeps a wall card in its wall plane", () => {
    const transform = propCardTransform(placement("+z"));
    expect(transform.rotationX).toBe(0);
    expect(transform.position.z).toBeCloseTo(6.51);
  });
});
