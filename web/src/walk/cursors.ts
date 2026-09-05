/**
 * Pure SVG cursor artwork for the local walk experiment.
 *
 * Large outlined badges distinguish a refused target from an active cooldown.
 */

export const WALK_CURSOR_SIZE = 40;
export const WALK_CURSOR_HOTSPOT = { x: 2, y: 2 } as const;

export type WalkCursorKind = "ready" | "waiting" | "refused";

export interface WalkCursorDataUris {
  ready: string;
  waiting: string;
  refused: string;
}

const PALE = "#eef2f3";
const DARK = "#17232b";
const ARROW = `<path d="M2.5 2.5v15.25l4.25-4.25 4.1 8 3.15-1.6-4.05-7.9H16.5z" fill="${PALE}" stroke="${DARK}" stroke-width="1" stroke-linejoin="round"/>`;

function svgDataUri(detail = ""): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${WALK_CURSOR_SIZE}" height="${WALK_CURSOR_SIZE}" viewBox="0 0 ${WALK_CURSOR_SIZE} ${WALK_CURSOR_SIZE}">${ARROW}${detail}</svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

export function walkCursorDataUris(): WalkCursorDataUris {
  const badge = (colour: string) => `<circle cx="27" cy="27" r="11" fill="${colour}" stroke="${DARK}" stroke-width="3"/><circle cx="27" cy="27" r="11.5" fill="none" stroke="${PALE}" stroke-width="1"/>`;
  const hourglass = `${badge("#f2c66d")}<path d="M22 20h10v2c0 2-2 3-4 5 2 2 4 3 4 5v2H22v-2c0-2 2-3 4-5-2-2-4-3-4-5z" fill="${DARK}"/>`;
  const refusedCross = `${badge("#f18d85")}<path d="M23 23l8 8M31 23l-8 8" fill="none" stroke="${DARK}" stroke-width="3.5" stroke-linecap="round"/>`;
  return {
    ready: svgDataUri(),
    waiting: svgDataUri(hourglass),
    refused: svgDataUri(refusedCross),
  };
}
