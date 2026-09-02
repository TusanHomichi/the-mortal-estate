/**
 * Pure intent state for a local, non-authoritative feel experiment.
 *
 * The injected `now` advances only this disposable presentation stand-in. It
 * neither decides real walkability nor ticks gameplay time.
 */
import { BeatClock, WALK_STAND_IN_BEAT_SECONDS } from "./beat";
import { sameCell, type Cell, type LayoutPassability } from "./layoutPassability";
import { findPath } from "./pathfinding";

export interface ActiveStep {
  from: Cell;
  to: Cell;
  startedAt: number;
}

export interface CommittedWalk {
  path: Cell[];
  /** Index of the cell where the active step will land. */
  stepIndex: number;
}

export interface WalkIntentState {
  caretakerCell: Cell;
  activeStep: ActiveStep | null;
  committed: CommittedWalk | null;
  preview: Cell[] | null;
}

export type WalkIntentKind = "idle" | "preview" | "committed";

function copyCell(cell: Cell): Cell {
  return { i: cell.i, j: cell.j };
}

function copyPath(path: readonly Cell[]): Cell[] {
  return path.map(copyCell);
}

export function createWalkIntent(caretakerCell: Cell): WalkIntentState {
  return {
    caretakerCell: copyCell(caretakerCell),
    activeStep: null,
    committed: null,
    preview: null,
  };
}

export function walkIntentKind(state: WalkIntentState): WalkIntentKind {
  if (state.committed !== null) return "committed";
  if (state.preview !== null) return "preview";
  return "idle";
}

export function planningOrigin(state: WalkIntentState): Cell {
  return copyCell(state.activeStep?.to ?? state.caretakerCell);
}

function previewTarget(preview: readonly Cell[] | null): Cell | null {
  return preview === null ? null : copyCell(preview[preview.length - 1]!);
}

function refreshedPreview(
  state: WalkIntentState,
  passability: LayoutPassability,
  target: Cell | null,
): Cell[] | null {
  if (target === null) return null;
  const origin = planningOrigin(state);
  if (sameCell(origin, target)) return null;
  return findPath(passability, origin, target);
}

export function cancelWalk(
  state: WalkIntentState,
  passability: LayoutPassability,
  now: number,
): WalkIntentState {
  const advanced = advanceWalk(state, passability, now);
  return {
    caretakerCell: copyCell(advanced.caretakerCell),
    activeStep: advanced.activeStep,
    committed: null,
    preview: null,
  };
}

export function singleClick(
  state: WalkIntentState,
  passability: LayoutPassability,
  target: Cell,
  now: number,
): WalkIntentState {
  const advanced = advanceWalk(state, passability, now);
  const origin = planningOrigin(advanced);
  if (sameCell(origin, target)) return { ...advanced, preview: null };
  const path = findPath(passability, origin, target);
  if (path === null) return advanced;
  return { ...advanced, preview: path };
}

export function doubleClick(
  state: WalkIntentState,
  passability: LayoutPassability,
  now: number,
): WalkIntentState {
  const advanced = advanceWalk(state, passability, now);
  if (advanced.preview === null || advanced.preview.length < 2) return advanced;

  const path = copyPath(advanced.preview);
  if (advanced.activeStep !== null) {
    return {
      ...advanced,
      committed: { path, stepIndex: 0 },
      preview: null,
    };
  }

  return {
    ...advanced,
    activeStep: {
      from: copyCell(path[0]!),
      to: copyCell(path[1]!),
      startedAt: now,
    },
    committed: { path, stepIndex: 1 },
    preview: null,
  };
}

function stepHasElapsed(step: ActiveStep, now: number): boolean {
  return new BeatClock(step.startedAt).beatsElapsed(now) > 0;
}

export function advanceWalk(
  state: WalkIntentState,
  passability: LayoutPassability,
  now: number,
): WalkIntentState {
  let current = state;
  const target = previewTarget(state.preview);
  while (current.activeStep !== null && stepHasElapsed(current.activeStep, now)) {
    const completed = current.activeStep;
    const landedAt = copyCell(completed.to);
    const nextStartedAt = completed.startedAt + WALK_STAND_IN_BEAT_SECONDS;
    const committed = current.committed;

    if (committed === null || committed.stepIndex >= committed.path.length - 1) {
      current = {
        ...current,
        caretakerCell: landedAt,
        activeStep: null,
        committed: null,
      };
    } else {
      const nextIndex = committed.stepIndex + 1;
      current = {
        ...current,
        caretakerCell: landedAt,
        activeStep: {
          from: copyCell(committed.path[committed.stepIndex]!),
          to: copyCell(committed.path[nextIndex]!),
          startedAt: nextStartedAt,
        },
        committed: { path: committed.path, stepIndex: nextIndex },
      };
    }
    current = { ...current, preview: refreshedPreview(current, passability, target) };
  }
  return current;
}

export function presentedCaretakerPosition(
  state: WalkIntentState,
  now: number,
): { i: number; j: number } {
  if (state.activeStep === null) return copyCell(state.caretakerCell);
  const phase = new BeatClock(state.activeStep.startedAt).phase(now);
  return {
    i: state.activeStep.from.i + (state.activeStep.to.i - state.activeStep.from.i) * phase,
    j: state.activeStep.from.j + (state.activeStep.to.j - state.activeStep.from.j) * phase,
  };
}
