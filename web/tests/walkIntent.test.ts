import { describe, expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { BeatClock } from "../src/walk/beat";
import { passabilityFrom } from "../src/walk/layoutPassability";
import {
  advanceWalk,
  cancelWalk,
  createWalkIntent,
  doubleClick,
  presentedCaretakerPosition,
  presentedWalkPosition,
  singleClick,
  walkPace,
  walkIntentKind,
} from "../src/walk/walkIntent";

const layout: FeelSpace = {
  grid_extents: { i: 5, j: 5 },
  cells: Array.from({ length: 25 }, (_, index) => ({
    i: index % 5,
    j: Math.floor(index / 5),
    material: "ground",
  })),
  wall_runs: [],
  roofs: [],
  props: [],
  fixtures: [],
  light_sources: { lantern_glass: null, candles: [] },
  weather: false,
  portals: [],
};
const passability = passabilityFrom(layout);
const clock = new BeatClock(0);

function draftRoute(now = 1) {
  return singleClick(
    createWalkIntent({ i: 0, j: 0 }),
    passability,
    { i: 3, j: 0 },
    clock,
    now,
  );
}

describe("the local walk-intent state machine", () => {
  it("a draft does not move the figure", () => {
    const state = draftRoute();
    expect(walkIntentKind(state)).toBe("draft");
    expect(state.caretakerCell).toEqual({ i: 0, j: 0 });
    expect(presentedCaretakerPosition(state)).toEqual({ i: 0, j: 0 });
  });

  it("a double click commits the drafted route", () => {
    const state = doubleClick(draftRoute(), clock, 1.2);
    expect(walkIntentKind(state)).toBe("committed");
    expect(state.committed?.route.at(-1)).toEqual({ i: 3, j: 0 });
    expect(state.committed?.landsAt).toBe(3);
  });

  it("derives walk, run, and sprint from a draft or committed route", () => {
    const initial = createWalkIntent({ i: 0, j: 0 });
    expect(walkPace(initial)).toBeNull();
    for (const [squares, pace] of [
      [1, "walk"],
      [2, "run"],
      [3, "sprint"],
    ] as const) {
      const draft = singleClick(initial, passability, { i: squares, j: 0 }, clock, 1);
      expect(walkPace(draft)).toBe(pace);
      expect(walkPace(doubleClick(draft, clock, 1.2))).toBe(pace);
    }
  });

  it("a committed route lands whole at the next strike, not before", () => {
    const committed = doubleClick(draftRoute(), clock, 1.2);
    expect(advanceWalk(committed, 2.999).caretakerCell).toEqual({ i: 0, j: 0 });
    const landed = advanceWalk(committed, 3);
    expect(landed.caretakerCell).toEqual({ i: 3, j: 0 });
    expect(landed.committed).toBeNull();
  });

  it("the presented position is never between squares", () => {
    const committed = doubleClick(draftRoute(), clock, 1.2);
    expect(presentedCaretakerPosition(advanceWalk(committed, 2.5))).toEqual({ i: 0, j: 0 });
    expect(presentedCaretakerPosition(advanceWalk(committed, 3))).toEqual({ i: 3, j: 0 });
  });

  it("a second single click on the drafted square commits", () => {
    const draft = draftRoute();
    const committed = singleClick(draft, passability, { i: 3, j: 0 }, clock, 1.5);
    expect(walkIntentKind(committed)).toBe("committed");
    expect(committed.draft).toBeNull();
    expect(committed.committed?.landsAt).toBe(3);
  });

  it("a replacement committed before landing keeps the same strike", () => {
    const first = doubleClick(draftRoute(), clock, 1.2);
    const replacementDraft = singleClick(first, passability, { i: 2, j: 2 }, clock, 2);
    const replacement = doubleClick(replacementDraft, clock, 2.2);
    expect(replacement.committed?.route.at(-1)).toEqual({ i: 2, j: 2 });
    expect(replacement.committed?.landsAt).toBe(first.committed?.landsAt);
  });

  it("a commit after landing lands at the following strike, never the same one", () => {
    const first = doubleClick(draftRoute(), clock, 1.2);
    const landed = advanceWalk(first, 3);
    const secondDraft = singleClick(landed, passability, { i: 4, j: 0 }, clock, 3);
    const second = doubleClick(secondDraft, clock, 3);
    expect(second.committed?.landsAt).toBe(6);
  });

  it("Escape clears an unlanded committed route", () => {
    const committed = doubleClick(draftRoute(), clock, 1.2);
    const cancelled = cancelWalk(committed, 2);
    expect(cancelled.committed).toBeNull();
    expect(cancelled.draft).toBeNull();
    expect(cancelled.caretakerCell).toEqual({ i: 0, j: 0 });
    expect(walkIntentKind(cancelled)).toBe("idle");
  });

  it("an unauthorable click clears the draft without disturbing a pending landing", () => {
    const committed = doubleClick(draftRoute(), clock, 1.2);
    const withDraft = singleClick(committed, passability, { i: 2, j: 2 }, clock, 2);
    const refused = singleClick(withDraft, passability, { i: 4, j: 4 }, clock, 2.1);
    expect(refused.draft).toBeNull();
    expect(refused.committed).toEqual(committed.committed);
  });
});

describe("the walk between pulses (presentation only)", () => {
  const passability = passabilityFrom(layout);
  const clock = new BeatClock(0);
  const committedRun = () => {
    // two squares, committed at t=0.5 to land on the strike at t=3
    const drafted = singleClick(createWalkIntent({ i: 0, j: 0 }), passability, { i: 2, j: 0 }, clock, 0.5);
    return doubleClick(drafted, clock, 0.5);
  };

  it("stands on its square with no commitment", () => {
    const presented = presentedWalkPosition(createWalkIntent({ i: 1, j: 1 }), 7);
    expect(presented).toEqual({ i: 1, j: 1, facing: 0, gait: "idle" });
  });

  it("starts where it stood, is between squares mid-pulse, and is on the target as the strike lands", () => {
    const state = committedRun();
    expect(state.committed?.landsAt).toBe(3);
    expect(presentedWalkPosition(state, 0.5)).toMatchObject({ i: 0, j: 0, gait: "run", facing: 1 });
    const mid = presentedWalkPosition(state, 1.75);
    expect(mid.i).toBeCloseTo(1, 5);
    expect(mid.gait).toBe("run");
    expect(presentedWalkPosition(state, 2.9).i).toBeCloseTo(1.92, 2);
    expect(presentedWalkPosition(state, 3)).toEqual({ i: 2, j: 0, facing: 0, gait: "idle" });
  });

  it("the authoritative square does not move until the strike", () => {
    const state = committedRun();
    expect(presentedCaretakerPosition(state)).toEqual({ i: 0, j: 0 });
    expect(presentedWalkPosition(state, 2.9).i).toBeGreaterThan(1.5);
    expect(advanceWalk(state, 2.9).caretakerCell).toEqual({ i: 0, j: 0 });
    expect(advanceWalk(state, 3).caretakerCell).toEqual({ i: 2, j: 0 });
  });

  it("a replacement continues from where the figure was presented, on the same strike", () => {
    const first = committedRun();
    const redrafted = singleClick(first, passability, { i: 0, j: 1 }, clock, 1.75);
    const replaced = doubleClick(redrafted, clock, 1.75);
    expect(replaced.committed?.landsAt).toBe(3);
    expect(replaced.committed?.from.i).toBeCloseTo(1, 5);
    expect(presentedWalkPosition(replaced, 1.75)).toMatchObject({ i: 1, j: 0 });
    expect(presentedWalkPosition(replaced, 3)).toEqual({ i: 0, j: 1, facing: 0, gait: "idle" });
  });

  it("faces along i by the current segment and 0 along a j-only segment", () => {
    const drafted = singleClick(createWalkIntent({ i: 2, j: 2 }), passability, { i: 2, j: 4 }, clock, 0);
    const state = doubleClick(drafted, clock, 0);
    expect(presentedWalkPosition(state, 1).facing).toBe(0);
    const back = doubleClick(singleClick(createWalkIntent({ i: 2, j: 2 }), passability, { i: 1, j: 2 }, clock, 0), clock, 0);
    expect(presentedWalkPosition(back, 1).facing).toBe(-1);
    expect(presentedWalkPosition(back, 1).gait).toBe("walk");
  });
});
