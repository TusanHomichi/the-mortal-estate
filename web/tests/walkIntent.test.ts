import { describe, expect, it } from "vitest";
import type { FeelLayout } from "../src/feelTypes";
import { BeatClock } from "../src/walk/beat";
import { passabilityFrom } from "../src/walk/layoutPassability";
import {
  advanceWalk,
  cancelWalk,
  createWalkIntent,
  doubleClick,
  presentedCaretakerPosition,
  singleClick,
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
