/* The candidate view: an unattested document, drawn by the accepted land's own
 * renderer.
 *
 * A candidate is what the staged log would produce, replayed against the
 * accepted master by the compiler. It is not a third kind of picture: the server
 * hands back a projection document of exactly the shape `/api/projection`
 * answers with, so this file draws it by calling `drawLogical` — the same
 * function, over the same lattice, with the same palette. A second renderer for
 * the candidate would be a picture that could differ from the accepted land for
 * reasons that had nothing to do with the edit, which is the one thing a preview
 * must never do.
 *
 * **The diff is the exception, and it is presentation.** Outlining the cells
 * that moved is a comparison of two documents the SERVER produced — the accepted
 * projection and the candidate projection — and it decides nothing about what
 * either one means. It resolves no identity, computes no coverage, and is never
 * sent anywhere. It exists because the owner is looking for what changed and
 * counting cells by eye is not a tool.
 *
 * Gestures over this view are built by `logicalGesture`, because the lattice is
 * the same lattice, and answered by `/api/candidate/preview`, because what
 * occupies a cell is not. The binds strip below shows which digests that answer
 * stands on: a candidate's bytes are replaced by the next preview, so the owner
 * is told what this one was computed against rather than left to assume.
 */

import { drawLogical } from "./logical.js";
import { acceptedMember, currentMember, state } from "./state.js";
import { canvas, context } from "./surface.js";

const CHANGED = "#c02f5c";

export function drawCandidate() {
  const member = currentMember();
  if (!member) return;
  drawLogical(member);
  outlineChanged(member, state.scale);
}

function outlineChanged(member, size) {
  const changed = state.candidateDiff ? state.candidateDiff.get(member.member) : null;
  if (!changed || changed.size === 0) return;
  context.strokeStyle = CHANGED;
  context.lineWidth = 2;
  for (const key of changed) {
    const [cellX, cellY] = key.split(",").map(Number);
    const x = state.origin.x + cellX * size;
    const y = state.origin.y + cellY * size;
    if (x + size < 0 || y + size < 0 || x > canvas.width || y > canvas.height) continue;
    context.strokeRect(x + 1, y + 1, size - 2, size - 2);
  }
}

/* ------------------------------------------------------------------ the diff */

/* Every cell whose terrain stack or passability differs, per member.
 *
 * Presentation only — see the module note. The two documents came from the
 * server, this compares them, and the result is drawn and counted and nothing
 * else. A cell one document carries and the other does not counts as changed,
 * because a lattice that changed size is a difference the owner must see rather
 * than a case to skip quietly.
 *
 * The terrain stack is compared in the order each document lists it. A reordered
 * stack with the same entries is a difference in the document and is shown as
 * one; guessing that the order does not matter would be this file having an
 * opinion about what a layer stack means, which is not its opinion to have.
 */
export function buildDiff(candidate) {
  const diff = new Map();
  for (const member of candidate.members) {
    const accepted = acceptedMember(member.member);
    const changed = new Set();
    const before = new Map();
    if (accepted) {
      for (const cell of accepted.cells) before.set(`${cell.x},${cell.y}`, signature(cell));
    }
    for (const cell of member.cells) {
      const key = `${cell.x},${cell.y}`;
      if (before.get(key) !== signature(cell)) changed.add(key);
      before.delete(key);
    }
    for (const key of before.keys()) changed.add(key);
    diff.set(member.member, changed);
  }
  return diff;
}

function signature(cell) {
  const stack = cell.terrain.map((entry) => `${entry.layer}:${entry.class}`).join("|");
  return `${cell.passable ? "passable" : "blocked"}/${stack}`;
}

function changedCount(name) {
  const changed = state.candidateDiff ? state.candidateDiff.get(name) : null;
  return changed ? changed.size : 0;
}

/* The count, in the legend under the picture, where the owner is already
 * looking. It says which member it counted: a diff over the surface says
 * nothing about the interior.
 */
export function diffSummary(name) {
  const total = changedCount(name);
  return (
    `${total} cell(s) of ${name} differ from the accepted projection` +
    (total > 0 ? " — outlined" : "")
  );
}

/* --------------------------------------------------------------- the binds */

/* Which digests the last candidate answer stands on, listed verbatim. */
export function renderBinds(records) {
  const heading = document.getElementById("binds-heading");
  const note = document.getElementById("binds-note");
  const list = document.getElementById("binds");
  list.innerHTML = "";
  const shown = records || [];
  heading.hidden = shown.length === 0;
  note.hidden = shown.length === 0;
  for (const record of shown) {
    const item = document.createElement("li");
    item.textContent = `${record.role}  ${record.sha256.slice(0, 12)}  ${record.path}`;
    list.append(item);
  }
}
