/** Render deadlines retain their phase when a display callback arrives late. */
export const PIXEL_FRAME_INTERVAL = 1000 / 30;
export function nextPixelFrame(deadline: number, now: number): number {
  if (!Number.isFinite(deadline)) return now + PIXEL_FRAME_INTERVAL;
  return deadline + (Math.floor(Math.max(0, now - deadline) / PIXEL_FRAME_INTERVAL) + 1) * PIXEL_FRAME_INTERVAL;
}
