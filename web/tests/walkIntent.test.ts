import { describe, expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { passabilityFrom } from "../src/walk/layoutPassability";
import {
  advanceWalk, cancelWalk, createWalkIntent, doubleClick, presentedCaretakerPosition,
  presentedWalkPosition, singleClick, walkPace, walkIntentKind,
} from "../src/walk/walkIntent";

const layout: FeelSpace = {
  grid_extents: { i: 5, j: 5 },
  cells: Array.from({ length: 25 }, (_, index) => ({ i: index % 5, j: Math.floor(index / 5), material: "ground" })),
  wall_runs: [], roofs: [], props: [], fixtures: [], structures: [],
  light_sources: { lantern_glass: null, candles: [] }, weather: false, portals: [],
};
const passability = passabilityFrom(layout);
const initial = () => createWalkIntent({ i: 0, j: 0 });
const draft = (now: number, i = 3, j = 0) => singleClick(initial(), passability, { i, j }, now);
const commit = (now: number, i = 3, j = 0) => doubleClick(draft(now, i, j), now);

describe("local movement lock", () => {
  it("drafts without moving and commits on a second click or double-click", () => {
    const planned = draft(1);
    expect(walkIntentKind(planned)).toBe("draft");
    expect(presentedCaretakerPosition(planned)).toEqual({ i: 0, j: 0 });
    for (const state of [doubleClick(planned, 1.2), singleClick(planned, passability, { i: 3, j: 0 }, 1.2)]) {
      expect(walkIntentKind(state)).toBe("committed");
      expect(state.draft).toBeNull();
      expect(state.committed).toEqual({ route: planned.draft, committedAt: 1.2, landsAt: 4.2 });
    }
  });

  it.each([0, 0.001, 1.5, 2.999999, 3, 5.999999, 101.998])(
    "gives a commitment at %s the full three seconds, independent of former beat phase", (now) => {
      for (const [i, j] of [[1, 0], [2, 0], [3, 0], [3, 3]] as const) {
        const state = commit(now, i, j);
        expect(state.committed!.landsAt - state.committed!.committedAt).toBeCloseTo(3, 10);
        expect(presentedWalkPosition(state, now + 0.03)).toMatchObject({ gait: i === 1 ? "walk" : i === 2 ? "run" : "sprint" });
        expect(presentedWalkPosition(state, now + 0.03).i).toBeCloseTo(i * 0.01, 8);
        expect(presentedWalkPosition(state, now + 1.5).i).toBeCloseTo(i / 2, 8);
        expect(advanceWalk(state, now + 2.999).caretakerCell).toEqual({ i: 0, j: 0 });
        const landed = advanceWalk(state, now + 3);
        expect(landed.caretakerCell).toEqual({ i, j });
        expect(landed.committed).toBeNull();
      }
    },
  );

  it("locks out redirection, repeated commits, and cancellation until completion", () => {
    const state = commit(2.999);
    for (const now of [3, 3.5, 5.998]) {
      expect(singleClick(state, passability, { i: 0, j: 2 }, now)).toBe(state);
      expect(doubleClick(state, now)).toBe(state);
      expect(cancelWalk(state, now)).toBe(state);
    }
    expect(state.committed!.landsAt).toBeCloseTo(5.999, 10);
  });

  it("starts a fresh full interval after the previous lock expires", () => {
    const first = commit(2.999);
    const next = singleClick(first, passability, { i: 3, j: 2 }, first.committed!.landsAt);
    expect(next.caretakerCell).toEqual({ i: 3, j: 0 });
    expect(walkIntentKind(next)).toBe("draft");
    const second = doubleClick(next, first.committed!.landsAt);
    expect(second.committed!.landsAt).toBeCloseTo(8.999, 10);
    expect(presentedWalkPosition(second, second.committed!.committedAt + 1.5)).toMatchObject({ i: 3, j: 1 });
  });

  it("allows clearing drafts and refuses unauthorable targets", () => {
    expect(walkIntentKind(cancelWalk(draft(1), 1.2))).toBe("idle");
    expect(singleClick(draft(1), passability, { i: 4, j: 4 }, 1.2).draft).toBeNull();
    expect(doubleClick(initial(), 1)).toEqual(initial());
  });

  it("keeps movement categories while the packet chooses their animation clips", () => {
    expect(walkPace(initial())).toBeNull();
    for (const [squares, pace] of [[1, "walk"], [2, "run"], [3, "sprint"]] as const) {
      expect(walkPace(draft(1, squares))).toBe(pace);
      expect(walkPace(commit(1, squares))).toBe(pace);
    }
  });

  it("keeps logical position fixed until completion while presenting the authored route", () => {
    const state = commit(0.5, 2);
    expect(presentedWalkPosition(state, 0.5)).toMatchObject({ i: 0, j: 0, gait: "run" });
    expect(presentedWalkPosition(state, 2)).toMatchObject({ i: 1, j: 0, gait: "run" });
    expect(presentedCaretakerPosition(state)).toEqual({ i: 0, j: 0 });
    expect(presentedWalkPosition(state, 3.5)).toEqual({ i: 2, j: 0, facing: { i: 1, j: 0 }, gait: "idle" });
    expect(presentedWalkPosition(initial(), 9)).toEqual({ i: 0, j: 0, facing: null, gait: "idle" });
  });
});
