import { describe, expect, it } from "vitest";
import { BeatClock, WALK_STAND_IN_BEAT_SECONDS } from "../src/walk/beat";

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
});
