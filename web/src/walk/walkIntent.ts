/**
 * Pure intent state for a local, non-authoritative feel experiment.
 *
 * The injected `now` advances only this disposable presentation stand-in. It
 * neither decides real walkability nor ticks gameplay time.
 */
import type { BeatClock } from "./beat";
import { sameCell, type Cell, type LayoutPassability } from "./layoutPassability";
import { authorRoute } from "./route";

export interface CommittedRoute {
  route: Cell[];
  landsAt: number;
}

export interface WalkIntentState {
  caretakerCell: Cell;
  draft: Cell[] | null;
  committed: CommittedRoute | null;
}

export type WalkIntentKind = "idle" | "draft" | "committed";
export type WalkPace = "walk" | "run" | "sprint";

function copyCell(cell: Cell): Cell {
  return { i: cell.i, j: cell.j };
}

function copyRoute(route: readonly Cell[]): Cell[] {
  return route.map(copyCell);
}

function routeEnd(route: readonly Cell[]): Cell {
  return route[route.length - 1]!;
}

export function createWalkIntent(caretakerCell: Cell): WalkIntentState {
  return {
    caretakerCell: copyCell(caretakerCell),
    draft: null,
    committed: null,
  };
}

export function walkIntentKind(state: WalkIntentState): WalkIntentKind {
  if (state.committed !== null) return "committed";
  if (state.draft !== null) return "draft";
  return "idle";
}

export function walkPace(state: WalkIntentState): WalkPace | null {
  const route = state.draft ?? state.committed?.route ?? null;
  if (route === null) return null;
  const squares = route.length - 1;
  if (squares === 1) return "walk";
  if (squares === 2) return "run";
  if (squares === 3) return "sprint";
  throw new Error(`a walk-experiment route has an invalid ${squares}-square pace`);
}

export function advanceWalk(state: WalkIntentState, now: number): WalkIntentState {
  if (state.committed === null || now < state.committed.landsAt) return state;
  return {
    caretakerCell: copyCell(routeEnd(state.committed.route)),
    draft: null,
    committed: null,
  };
}

function commitDraft(
  state: WalkIntentState,
  clock: BeatClock,
  now: number,
): WalkIntentState {
  if (state.draft === null) return state;
  return {
    ...state,
    draft: null,
    committed: {
      route: copyRoute(state.draft),
      landsAt: state.committed?.landsAt ?? clock.nextStrikeAfter(now),
    },
  };
}

export function cancelWalk(state: WalkIntentState, now: number): WalkIntentState {
  const advanced = advanceWalk(state, now);
  if (advanced.draft === null && advanced.committed === null) return advanced;
  return { ...advanced, draft: null, committed: null };
}

export function singleClick(
  state: WalkIntentState,
  passability: LayoutPassability,
  target: Cell,
  clock: BeatClock,
  now: number,
): WalkIntentState {
  const advanced = advanceWalk(state, now);
  if (advanced.draft !== null && sameCell(routeEnd(advanced.draft), target)) {
    return commitDraft(advanced, clock, now);
  }

  const draft = authorRoute(passability, advanced.caretakerCell, target);
  return { ...advanced, draft };
}

export function doubleClick(
  state: WalkIntentState,
  clock: BeatClock,
  now: number,
): WalkIntentState {
  return commitDraft(advanceWalk(state, now), clock, now);
}

export function presentedCaretakerPosition(state: WalkIntentState): Cell {
  return copyCell(state.caretakerCell);
}
