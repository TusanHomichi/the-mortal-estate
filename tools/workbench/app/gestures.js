/* A gesture, from the mouse button to the written packet.
 *
 * Collecting a drag and asking what it means are the same story told in two
 * halves, so they share a file: the handlers below accumulate a drag in surface
 * coordinates, hand it to the current view for a request body, and send it. The
 * view modules decide what the shape IS — cells or pixels — and this module
 * decides nothing at all beyond which of them to ask.
 *
 * Preview and record are the only two things a gesture can become, and both go
 * to the server. There is no local resolution step between them and no cached
 * answer: the preview the owner is looking at and the packet that gets written
 * came from the same resolver an agent reads later, which is the only reason
 * the picture on screen is evidence of anything.
 *
 * A refused request leaves the last good state on screen. The mouse handler
 * swallows the throw for exactly that reason — `refuse()` has already shown the
 * server's own words, and half-applying a failed selection would be worse than
 * showing nothing new.
 */

import {
  previewCandidateGesture,
  previewCaptureGesture,
  previewLogicalGesture,
  writeCaptureSelection,
  writeLogicalSelection,
} from "./api.js";
import { renderBinds } from "./candidate.js";
import { captureGesture } from "./capture.js";
import { renderResolution } from "./identities.js";
import { logicalGesture } from "./logical.js";
import { capturing, currentMember, previewingCandidate, state } from "./state.js";
import { refreshSession } from "./session.js";
import {
  canvas,
  clampCell,
  clampPixel,
  surfacePoint,
  zoomCeiling,
  zoomFloor,
} from "./surface.js";
import { draw } from "./view.js";

/* One gesture, three routes, one answer shape. A gesture over the candidate is
 * resolved in the candidate's own frame — same lattice, different occupants —
 * and comes back in the shape the identity panel already renders, so the panel
 * is not told which surface produced it. What the candidate answer carries
 * extra is the digest list it stands on, which is shown beside it.
 */
export async function preview(body) {
  state.gesture = body;
  let answer;
  if (capturing()) answer = await previewCaptureGesture(body);
  else if (previewingCandidate()) answer = await previewCandidateGesture(body);
  else answer = await previewLogicalGesture(body);
  state.pending = capturing() ? null : answer.cells;
  state.covered = capturing() ? answer.observed : null;
  renderResolution(answer);
  renderBinds(answer.binds || []);
  /* No packet is written from the candidate view. A packet binds the bytes it
   * was taken against, and the next preview replaces a candidate's bytes. */
  document.getElementById("record").disabled = previewingCandidate();
  draw();
}

export async function record() {
  if (!state.gesture || previewingCandidate()) return;
  const body = Object.assign({}, state.gesture, {
    comment: document.getElementById("comment").value,
  });
  if (!capturing()) body.canvas_rect = state.canvasRect;
  const answer = capturing()
    ? await writeCaptureSelection(body)
    : await writeLogicalSelection(body);
  document.getElementById("comment").value = "";
  await refreshSession(answer.packet.selection_id);
}

/* Registered once, from the entry point, so that the order the page starts in
 * is written down in one place instead of falling out of import order.
 */
export function installGestures() {
  let gestureStarted = false;
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());

  canvas.addEventListener("mousedown", (event) => {
    if (capturing() ? !state.captureImage : !currentMember()) return;
    if (event.button === 2 || state.spaceHeld) {
      state.panning = { x: event.clientX, y: event.clientY, origin: { ...state.origin } };
      return;
    }
    if (event.button !== 0) return;
    gestureStarted = true;
    const point = surfacePoint(event);
    if (state.tool === "box") {
      state.drag = { kind: "box", from: point, to: point };
    } else if (state.tool === "lasso") {
      state.drag = { kind: "lasso", points: [point] };
    } else if (state.tool === "paint") {
      if (capturing()) {
        state.drag = { kind: "paint", points: [clampPixel(point)] };
      } else {
        state.drag = { kind: "paint", cells: [clampCell(point, currentMember())] };
        state.pending = state.drag.cells.slice();
      }
    }
    draw();
  });

  window.addEventListener("mousemove", (event) => {
    if (state.panning) {
      const box = canvas.getBoundingClientRect();
      const ratio = canvas.width / box.width;
      state.origin = {
        x: state.panning.origin.x + (event.clientX - state.panning.x) * ratio,
        y: state.panning.origin.y + (event.clientY - state.panning.y) * ratio,
      };
      draw();
      return;
    }
    if (!state.drag) return;
    const point = surfacePoint(event);
    if (state.drag.kind === "box") {
      state.drag.to = point;
    } else if (state.drag.kind === "lasso") {
      state.drag.points.push(point);
    } else if (state.drag.kind === "paint" && capturing()) {
      const pixel = clampPixel(point);
      if (!state.drag.points.some((row) => row.x === pixel.x && row.y === pixel.y)) {
        state.drag.points.push(pixel);
      }
    } else if (state.drag.kind === "paint") {
      const cell = clampCell(point, currentMember());
      if (!state.drag.cells.some((row) => row.x === cell.x && row.y === cell.y)) {
        state.drag.cells.push(cell);
        state.pending = state.drag.cells.slice();
      }
    }
    draw();
  });

  window.addEventListener("mouseup", async (event) => {
    if (state.panning) {
      state.panning = null;
      gestureStarted = false;
      return;
    }
    if (!gestureStarted) return;
    gestureStarted = false;
    if (capturing() ? !state.captureImage : !currentMember()) return;
    const drag = state.drag;
    state.drag = null;
    const point = surfacePoint(event);

    try {
      const body = capturing()
        ? captureGesture(drag, point)
        : logicalGesture(currentMember(), drag, point);
      if (body) await preview(body);
    } catch (error) {
      /* refuse() already showed the reason; the view stays on the last good state. */
    }
    draw();
  });

  canvas.addEventListener("wheel", (event) => {
    event.preventDefault();
    const before = surfacePoint(event);
    const limit = zoomCeiling();
    const floor = zoomFloor();
    state.scale = Math.max(floor, Math.min(limit, state.scale * (event.deltaY < 0 ? 1.12 : 0.89)));
    const after = surfacePoint(event);
    state.origin.x += (after.x - before.x) * state.scale;
    state.origin.y += (after.y - before.y) * state.scale;
    draw();
  }, { passive: false });

  window.addEventListener("keydown", (event) => {
    if (event.code === "Space") state.spaceHeld = true;
  });
  window.addEventListener("keyup", (event) => {
    if (event.code === "Space") state.spaceHeld = false;
  });
}
