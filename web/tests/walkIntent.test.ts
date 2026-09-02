import { describe, expect, it } from "vitest";
import type { FeelLayout } from "../src/feelTypes";
import { WALK_STAND_IN_BEAT_SECONDS } from "../src/walk/beat";
import { passabilityFrom } from "../src/walk/layoutPassability";
import {
  advanceWalk,
  doubleClick,
  presentedCaretakerPosition,
  singleClick,
  createWalkIntent,
  walkIntentKind,
} from "../src/walk/walkIntent";

const layout: FeelLayout = {
  grid_extents: { i: 5, j: 5 },
  cells: Array.from({ length: 25 }, (_, index) => ({
    i: index % 5,
    j: Math.floor(index / 5),
    material: "ground",
  })),
  wall_runs: [],
  props: [],
  light_sources: { lantern_glass: [0, 0, 0], candles: [] },
};
const passability = passabilityFrom(layout);

describe("the local walk-intent state machine", () => {
  it("a single click previews and does not move", () => {
    const state = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 3, j: 0 }, 10);
    expect(walkIntentKind(state)).toBe("preview");
    expect(state.caretakerCell).toEqual({ i: 0, j: 0 });
    expect(state.activeStep).toBeNull();
  });

  it("a double click commits the previewed path", () => {
    const preview = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 2, j: 0 }, 10);
    const state = doubleClick(preview, passability, 11);
    expect(walkIntentKind(state)).toBe("committed");
    expect(state.activeStep).toMatchObject({ from: { i: 0, j: 0 }, to: { i: 1, j: 0 } });
  });

  it("advances exactly one cell per injected beat and never from outside time", () => {
    const preview = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 3, j: 0 }, 10);
    const committed = doubleClick(preview, passability, 10);
    expect(
      advanceWalk(
        committed,
        passability,
        10 + WALK_STAND_IN_BEAT_SECONDS - 0.001,
      ).caretakerCell,
    ).toEqual({ i: 0, j: 0 });
    expect(
      advanceWalk(committed, passability, 10 + WALK_STAND_IN_BEAT_SECONDS).caretakerCell,
    ).toEqual({ i: 1, j: 0 });
    expect(
      advanceWalk(committed, passability, 10 + WALK_STAND_IN_BEAT_SECONDS * 2)
        .caretakerCell,
    ).toEqual({ i: 2, j: 0 });
    expect(committed.caretakerCell).toEqual({ i: 0, j: 0 });
  });

  it("a re-preview during a walk starts from the next cell", () => {
    const preview = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 3, j: 0 }, 10);
    const committed = doubleClick(preview, passability, 10);
    const replacement = singleClick(committed, passability, { i: 2, j: 2 }, 11);
    expect(walkIntentKind(replacement)).toBe("committed");
    expect(replacement.committed?.path).toEqual(committed.committed?.path);
    expect(replacement.preview?.[0]).toEqual({ i: 1, j: 0 });
  });

  it("a re-preview during a walk keeps the walk going", () => {
    const preview = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 3, j: 0 }, 10);
    const committed = doubleClick(preview, passability, 10);
    const replacement = singleClick(committed, passability, { i: 2, j: 2 }, 11);
    const advanced = advanceWalk(replacement, passability, 10 + WALK_STAND_IN_BEAT_SECONDS);
    expect(advanced.caretakerCell).toEqual({ i: 1, j: 0 });
    expect(advanced.activeStep).toMatchObject({ from: { i: 1, j: 0 }, to: { i: 2, j: 0 } });
    expect(advanced.committed).not.toBeNull();
    expect(advanced.preview).not.toBeNull();
  });

  it("the in-progress step completes while a replacement path is previewed and recommitted", () => {
    const preview = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 3, j: 0 }, 10);
    const committed = doubleClick(preview, passability, 10);
    const replacement = singleClick(committed, passability, { i: 2, j: 2 }, 11);
    const beforeLanding = presentedCaretakerPosition(replacement, 12);
    const recommitted = doubleClick(replacement, passability, 12);
    const landed = advanceWalk(recommitted, passability, 10 + WALK_STAND_IN_BEAT_SECONDS);
    expect(beforeLanding.i).toBeGreaterThan(0);
    expect(beforeLanding.i).toBeLessThan(1);
    expect(landed.caretakerCell).toEqual({ i: 1, j: 0 });
    expect(landed.activeStep?.from).toEqual({ i: 1, j: 0 });
  });
});
