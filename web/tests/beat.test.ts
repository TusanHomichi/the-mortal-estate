import { describe, expect, it } from "vitest";
import {
  BeatClock,
  WALK_MAX_ROUTE_SQUARES,
  WALK_STAND_IN_BEAT_SECONDS,
} from "../src/walk/beat";

describe("the local stand-in beat clock", () => {
  it("reports whole elapsed beats only from the injected time", () => {
    const clock = new BeatClock(10);
    expect(clock.beatsElapsed(10 + WALK_STAND_IN_BEAT_SECONDS - 0.001)).toBe(0);
    expect(clock.beatsElapsed(10 + WALK_STAND_IN_BEAT_SECONDS)).toBe(1);
  });

  it("resets its phase on each stand-in beat", () => {
    const clock = new BeatClock(10);
    expect(clock.phase(10 + WALK_STAND_IN_BEAT_SECONDS * 1.5)).toBeCloseTo(0.5);
  });

  it("names the first shared strike strictly after the injected time", () => {
    const clock = new BeatClock(10);
    expect(clock.nextStrikeAfter(10)).toBe(13);
    expect(clock.nextStrikeAfter(12.999)).toBe(13);
    expect(clock.nextStrikeAfter(13)).toBe(16);
  });

  it("carries the experiment's reopened route allowance in one place", () => {
    expect(WALK_MAX_ROUTE_SQUARES).toBe(3);
  });
});
