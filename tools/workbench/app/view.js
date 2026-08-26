/* What the owner is looking at: the surface drawn, and the legend under it.
 *
 * Two views share one canvas, so something has to decide which one is on and
 * fit it to the window. That decision is here, above both view modules and
 * below everything that reacts to it, which is what keeps the import graph a
 * line rather than a knot: the views draw, this dispatches, the panels ask.
 *
 * The drag overlay is drawn here too, on purpose. It is the only mark that
 * belongs to neither view — it is the gesture in progress, before the server
 * has been asked anything at all, and it looks the same over a grid as it does
 * over a photograph.
 */

import { diffSummary, drawCandidate } from "./candidate.js";
import { drawCapture } from "./capture.js";
import { colourOf, drawLogical } from "./logical.js";
import {
  capturing,
  currentCapture,
  currentMember,
  previewingCandidate,
  state,
} from "./state.js";
import { canvas, context, surfaceExtent } from "./surface.js";

export function draw() {
  context.clearRect(0, 0, canvas.width, canvas.height);
  if (capturing()) {
    drawCapture();
  } else if (previewingCandidate()) {
    drawCandidate();
  } else {
    drawLogical(currentMember());
  }
  drawDrag(state.scale);
}

function drawDrag(size) {
  if (!state.drag) return;
  context.strokeStyle = "#8c2f1f";
  context.lineWidth = 1.5;
  if (state.drag.kind === "box") {
    const from = state.drag.from;
    const to = state.drag.to;
    context.strokeRect(
      state.origin.x + Math.min(from.x, to.x) * size,
      state.origin.y + Math.min(from.y, to.y) * size,
      Math.abs(to.x - from.x) * size,
      Math.abs(to.y - from.y) * size
    );
  } else if (state.drag.kind === "lasso" && state.drag.points.length > 1) {
    context.beginPath();
    state.drag.points.forEach((point, index) => {
      const x = state.origin.x + point.x * size;
      const y = state.origin.y + point.y * size;
      if (index === 0) context.moveTo(x, y);
      else context.lineTo(x, y);
    });
    context.closePath();
    context.stroke();
  } else if (state.drag.kind === "paint" && capturing()) {
    context.fillStyle = "rgba(140,47,31,0.6)";
    for (const point of state.drag.points) {
      context.fillRect(
        state.origin.x + point.x * size,
        state.origin.y + point.y * size,
        Math.max(2, size),
        Math.max(2, size)
      );
    }
  }
}

/* Put the whole surface on screen. The logical view snaps to whole pixels per
 * cell because a grid drawn at 13.4 pixels a cell shimmers; a photograph does
 * not, so it scales continuously.
 */
export function fit() {
  const extent = surfaceExtent();
  if (!extent) return;
  const scale = capturing()
    ? Math.min(canvas.width / extent.width, canvas.height / extent.height)
    : Math.floor(Math.min(canvas.width / extent.width, canvas.height / extent.height));
  state.scale = capturing() ? Math.max(0.05, scale) : Math.max(6, scale);
  const drawn = capturing()
    ? { width: extent.width, height: extent.height }
    : { width: extent.width - 1, height: extent.height - 1 };
  state.origin = {
    x: (canvas.width - drawn.width * state.scale) / 2,
    y: (canvas.height - drawn.height * state.scale) / 2,
  };
  draw();
}

/* The legend says what the current view is made of, and the two views are made
 * of different things: terrain classes on one side, the provenance of one
 * photograph on the other. One function because it is one strip of the page,
 * and the owner should never have to wonder which strip they are reading.
 */
export function renderLegend() {
  const legend = document.getElementById("legend");
  legend.innerHTML = "";
  if (capturing()) {
    const taken = currentCapture();
    const note = document.createElement("span");
    note.textContent = taken
      ? `${taken.capture_id} · ${taken.member} · frame generation ${taken.frame_generation}` +
        ` · ${taken.viewport.width}×${taken.viewport.height} · ${taken.targets} addressable targets` +
        ` · ${taken.camera.square_pitch_px}px per square · route ${taken.route}`
      : "No capture in this session yet.";
    legend.append(note);
    return;
  }
  const member = currentMember();
  if (!member) return;
  const seen = new Map();
  for (const cell of member.cells) {
    for (const entry of cell.terrain) {
      seen.set(entry.class, entry.layer);
    }
  }
  [...seen.entries()].sort().forEach(([name, layer]) => {
    const item = document.createElement("span");
    const swatch = document.createElement("i");
    swatch.className = "swatch";
    swatch.style.background = colourOf(name, layer);
    item.append(swatch, document.createTextNode(`${name} (${layer})`));
    legend.append(item);
  });
  const marks = document.createElement("span");
  marks.textContent = "A access · D door · L landmark · T transition · ✕ impassable";
  legend.append(marks);
  if (!previewingCandidate()) return;
  /* The count of what moved, under the picture the owner is reading it in. */
  const changed = document.createElement("span");
  changed.className = "changed";
  changed.textContent = diffSummary(member.member);
  legend.append(changed);
}
