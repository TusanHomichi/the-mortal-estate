/* The canvas, and the arithmetic between a pointer and the thing under it.
 *
 * Both views draw into one canvas and share one camera — an origin in canvas
 * pixels and a scale — so converting a mouse position into a surface coordinate
 * belongs to neither view. It is here, once, and both call it. A second copy of
 * this arithmetic is how the two views start disagreeing about what the owner
 * pointed at, which is the one disagreement this tool cannot afford.
 *
 * What a surface coordinate MEANS is still not decided here. Over the logical
 * view it is a cell of the compiler's grid; over a capture it is a pixel of the
 * photograph. That is why the two clamps are separate functions with separate
 * bounds rather than one clamp with a flag: the bound comes from a different
 * authority in each case — the member's extent, or the picture's own size — and
 * blurring them is exactly how a pixel starts being treated as a cell.
 *
 * This module imports the state and nothing else, so the view modules can draw
 * through it without the drawing dispatch having to reach back down.
 */

import { capturing, currentMember, state } from "./state.js";

export const canvas = document.getElementById("canvas");
export const context = canvas.getContext("2d");

export function surfacePoint(event) {
  const box = canvas.getBoundingClientRect();
  const x = (event.clientX - box.left) * (canvas.width / box.width);
  const y = (event.clientY - box.top) * (canvas.height / box.height);
  return {
    x: (x - state.origin.x) / state.scale,
    y: (y - state.origin.y) / state.scale,
  };
}

export function surfaceExtent() {
  if (capturing()) {
    const picture = state.captureImage;
    return picture ? { width: picture.width, height: picture.height } : null;
  }
  const member = currentMember();
  return member ? { width: member.width + 1, height: member.height + 1 } : null;
}

export function clampCell(cell, member) {
  return {
    x: Math.max(0, Math.min(member.width - 1, Math.floor(cell.x))),
    y: Math.max(0, Math.min(member.height - 1, Math.floor(cell.y))),
  };
}

export function clampPixel(point) {
  const picture = state.captureImage;
  return {
    x: Math.max(0, Math.min(picture.width - 1, Math.floor(point.x))),
    y: Math.max(0, Math.min(picture.height - 1, Math.floor(point.y))),
  };
}

/* The rectangle a gesture covered in canvas pixels, recorded in the packet as
 * the client's own account of where on screen the owner was pointing.
 */
export function canvasRectOf(from, to) {
  return {
    x: Math.round(Math.min(from.x, to.x) * state.scale + state.origin.x),
    y: Math.round(Math.min(from.y, to.y) * state.scale + state.origin.y),
    width: Math.round(Math.abs(to.x - from.x) * state.scale),
    height: Math.round(Math.abs(to.y - from.y) * state.scale),
  };
}

/* The zoom stops the wheel and the two zoom buttons both obey. They are one
 * fact in one place because the wheel and the buttons doing different things at
 * the same zoom level is a bug nobody would think to look for. `fit()` keeps
 * its own logical floor, deliberately: fitting a whole member on screen is a
 * different question from how far a person may zoom out by hand.
 */
export function zoomCeiling() {
  return capturing() ? 12 : 72;
}

export function zoomFloor() {
  return capturing() ? 0.05 : 5;
}
