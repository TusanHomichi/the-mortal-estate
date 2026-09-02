/**
 * Local timing for the browser feel scene's walk experiment.
 *
 * This is deliberately not a gameplay clock. The authoritative pulse is owned
 * at docs/boundary-map.md#21-the-authoritative-pulse-d5; this stand-in exists
 * only so the no-server feel scene can be walked and judged.
 */
export const WALK_STAND_IN_BEAT_SECONDS = 3;

/**
 * This experiment defaults to three authored route squares. The allowance is
 * reopened under owner ruling D2; it is not a settled gameplay value.
 */
export const WALK_MAX_ROUTE_SQUARES = 3;

export class BeatClock {
  readonly startedAt: number;

  constructor(startedAt: number) {
    this.startedAt = startedAt;
  }

  phase(now: number): number {
    const elapsed = Math.max(0, now - this.startedAt);
    return (elapsed / WALK_STAND_IN_BEAT_SECONDS) % 1;
  }

  beatsElapsed(now: number): number {
    return Math.floor(Math.max(0, now - this.startedAt) / WALK_STAND_IN_BEAT_SECONDS);
  }

  nextStrikeAfter(now: number): number {
    const completedBeats = Math.floor(
      Math.max(0, now - this.startedAt) / WALK_STAND_IN_BEAT_SECONDS,
    );
    return this.startedAt + (completedBeats + 1) * WALK_STAND_IN_BEAT_SECONDS;
  }
}
