import { describe, expect, it } from "vitest";
import { Vector3 } from "three";
import { facingBetween, facingYaw } from "../src/walk/facing";
import { presentedWalkPosition, type WalkIntentState } from "../src/walk/walkIntent";

describe("ground-plane figure facing", () => {
  it("turns authored +z forward along all eight routes, including north and south", () => {
    for (let i = -1; i <= 1; i += 1) for (let j = -1; j <= 1; j += 1) {
      if (!i && !j) continue;
      const heading = facingBetween({ i: 4, j: 3 }, { i: 4 + i, j: 3 + j })!;
      expect(heading).toEqual({ i, j });
      const forward = new Vector3(0, 0, 1).applyAxisAngle(new Vector3(0, 1, 0), facingYaw(heading));
      const expected = new Vector3(i, 0, j).normalize();
      expect(forward.distanceTo(expected)).toBeLessThan(1e-9);
    }
    expect(facingBetween({ i: 2, j: 2 }, { i: 2, j: 2 })).toBeNull();
    expect(() => facingYaw({ i: 0, j: 0 })).toThrow(/eight/);
  });

  it("turns at a bent route's corner and retains the final segment on a skipped landing frame", () => {
    const state: WalkIntentState = { caretakerCell: { i: 0, j: 0 }, draft: null,
      committed: { route: [{ i: 0, j: 0 }, { i: 1, j: 0 }, { i: 1, j: -1 }],
        committedAt: 0, landsAt: 3 } };
    expect(presentedWalkPosition(state, 1).facing).toEqual({ i: 1, j: 0 });
    expect(presentedWalkPosition(state, 2).facing).toEqual({ i: 0, j: -1 });
    expect(presentedWalkPosition(state, 4)).toMatchObject({ i: 1, j: -1, facing: { i: 0, j: -1 }, gait: "idle" });
  });
});
