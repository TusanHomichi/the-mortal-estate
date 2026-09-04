/**
 * Pure intent state for a local, non-authoritative feel experiment.
 *
 * The injected `now` advances only this disposable presentation stand-in. It
 * neither decides real walkability nor ticks gameplay time.
 */
import { facingBetween, type FigureFacing } from "./facing";
import { WALK_MOVE_SECONDS } from "./beat";
import { sameCell, type Cell, type LayoutPassability } from "./layoutPassability";
import { authorRoute } from "./route";

export interface CommittedRoute {
  route: Cell[];
  landsAt: number;
  /** When the commitment was made. */
  committedAt: number;

}

/** A presented (not authoritative) position: fractional between squares while walking. */
export interface PresentedPoint {
  i: number;
  j: number;
}

export type Gait = "idle" | WalkPace;

export interface PresentedWalk extends PresentedPoint {
  /** Heading of the current presented segment; null only while standing still. */
  facing: FigureFacing | null;
  gait: Gait;
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
  now: number,
): WalkIntentState {
  if (state.committed !== null || state.draft === null) return state;
  return {
    ...state,
    draft: null,
    committed: {
      route: copyRoute(state.draft),
      landsAt: now + WALK_MOVE_SECONDS,
      committedAt: now,
    },
  };
}

function segmentLengths(path: readonly PresentedPoint[]): number[] {
  return path.slice(1).map((point, index) => Math.hypot(point.i - path[index]!.i, point.j - path[index]!.j));
}

function presentedFraction(committed: CommittedRoute, now: number): number {
  const span = committed.landsAt - committed.committedAt;
  return span <= 0 ? 1 : Math.min(1, Math.max(0, (now - committed.committedAt) / span));
}

export function cancelWalk(state: WalkIntentState, now: number): WalkIntentState {
  const advanced = advanceWalk(state, now);
  if (advanced.committed !== null || advanced.draft === null) return advanced;
  return { ...advanced, draft: null, committed: null };
}

export function singleClick(
  state: WalkIntentState,
  passability: LayoutPassability,
  target: Cell,
  now: number,
): WalkIntentState {
  const advanced = advanceWalk(state, now);
  if (advanced.committed !== null) return advanced;
  if (advanced.draft !== null && sameCell(routeEnd(advanced.draft), target)) {
    return commitDraft(advanced, now);
  }

  const draft = authorRoute(passability, advanced.caretakerCell, target);
  return { ...advanced, draft };
}

export function doubleClick(
  state: WalkIntentState,
  now: number,
): WalkIntentState {
  return commitDraft(advanceWalk(state, now), now);
}

/** The authoritative square: what the game believes, never between squares. */
export function presentedCaretakerPosition(state: WalkIntentState): Cell {
  return copyCell(state.caretakerCell);
}

function paceOf(route: readonly Cell[]): WalkPace {
  const squares = route.length - 1;
  if (squares <= 1) return "walk";
  if (squares === 2) return "run";
  return "sprint";
}

/**
 * Each local commitment receives a full movement interval. Inputs cannot
 * replace it or shorten its lock. Logical position lands when that interval
 * ends; this preview is independent of authoritative server timing.
 */
export function presentedWalkPosition(state: WalkIntentState, now: number): PresentedWalk {
  const committed = state.committed;
  if (committed === null) return { ...copyCell(state.caretakerCell), facing: null, gait: "idle" };
  // At or past the strike the figure stands on the target even before the
  // state has been advanced to land it; the two agree the moment it is.
  if (now >= committed.landsAt) return { ...copyCell(routeEnd(committed.route)),
    facing: facingBetween(committed.route[committed.route.length - 2]!, routeEnd(committed.route)), gait: "idle" };
  const fraction = presentedFraction(committed, now);
  const path = committed.route;
  const lengths = segmentLengths(path);
  const total = lengths.reduce((sum, length) => sum + length, 0);
  let remaining = fraction * total;
  for (let index = 0; index < lengths.length; index += 1) {
    const length = lengths[index]!;
    const start = path[index]!;
    const end = path[index + 1]!;
    if (remaining <= length || index === lengths.length - 1) {
      const t = length === 0 ? 1 : Math.min(1, remaining / length);
      return {
        i: start.i + (end.i - start.i) * t,
        j: start.j + (end.j - start.j) * t,
        facing: facingBetween(start, end),
        gait: paceOf(committed.route),
      };
    }
    remaining -= length;
  }
  const end = path[path.length - 1]!;
  return { i: end.i, j: end.j, facing: null, gait: paceOf(committed.route) };
}
