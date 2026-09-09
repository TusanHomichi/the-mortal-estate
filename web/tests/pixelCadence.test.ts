import { describe, expect, it } from 'vitest';
import { nextPixelFrame } from '../src/play/pixelCadence';

describe('pixel render cadence', () => {
  it('retains 30 submissions per second across rounded 60Hz callbacks', () => {
    let deadline = -Infinity;
    const frames: number[] = [];
    for (let i = 0; i < 600; i++) {
      const now = Math.round(i * 1000 / 60 * 100) / 100;
      if (now + .1 >= deadline) { frames.push(now); deadline = nextPixelFrame(deadline, now); }
    }
    expect(frames).toHaveLength(300);
    expect(Math.max(...frames.slice(1).map((time, i) => time - frames[i]!))).toBeLessThan(34);
  });
  it('skips expired deadlines after a stall without catch-up bursts', () => {
    const deadline = nextPixelFrame(100, 501);
    expect(deadline).toBeGreaterThan(501);
    expect(deadline).toBeLessThanOrEqual(501 + 1000 / 30);
    expect(nextPixelFrame(-Infinity, 50)).toBeCloseTo(50 + 1000 / 30);
  });
});
