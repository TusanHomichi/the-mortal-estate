/**
 * Pure SVG cursor artwork for the local walk experiment.
 *
 * The pale fill and dark one-pixel outline stay legible over both plaster and
 * grass without borrowing the warm palette reserved for actual light sources.
 */

export const WALK_CURSOR_SIZE = 24;
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
  const hourglass = `<path d="M15 14.5h6v1c0 1.2-.8 1.9-2 2.5 1.2.6 2 1.3 2 2.5v1h-6v-1c0-1.2.8-1.9 2-2.5-1.2-.6-2-1.3-2-2.5z" fill="${PALE}" stroke="${DARK}" stroke-width="1" stroke-linejoin="round"/>`;
  const refusedBar = `<path d="M15 21l6-6" fill="none" stroke="${DARK}" stroke-width="5" stroke-linecap="round"/><path d="M15 21l6-6" fill="none" stroke="${PALE}" stroke-width="3" stroke-linecap="round"/>`;
  return {
    ready: svgDataUri(),
    waiting: svgDataUri(hourglass),
    refused: svgDataUri(refusedBar),
  };
}
