/* The capture view: a photograph the client took, and nothing drawn over it.
 *
 * The whole value of a capture is that it shows what the client actually drew,
 * so this file draws the picture and stops. There is no overlay of the
 * compiler's opinion on top of a photograph — that would be a second renderer
 * wearing the first one's evidence — and the only marks added are the
 * rectangles the SERVER said a gesture covered, drawn back exactly where the
 * server put them.
 *
 * Gestures here are expressed in the picture's own pixels, which is the space
 * the identity raster indexes. Nothing is converted to cells in this file:
 * which square a pixel belongs to is the sidecar's answer, not the browser's
 * guess, and the difference between those two sentences is the reason the
 * capture view is trustworthy at all.
 */

import { captureImagePath } from "./api.js";
import { state } from "./state.js";
import { clampPixel, context } from "./surface.js";

export function drawCapture() {
  const picture = state.captureImage;
  if (!picture) return;
  context.imageSmoothingEnabled = false;
  context.drawImage(
    picture,
    state.origin.x,
    state.origin.y,
    picture.width * state.scale,
    picture.height * state.scale
  );
  if (!state.covered) return;
  context.fillStyle = "rgba(31,93,140,0.34)";
  context.strokeStyle = "#1f5d8c";
  context.lineWidth = 1.5;
  for (const record of state.covered) {
    const shape = record.hit_shape;
    const x = state.origin.x + shape.x * state.scale;
    const y = state.origin.y + shape.y * state.scale;
    context.fillRect(x, y, shape.width * state.scale, shape.height * state.scale);
    context.strokeRect(x, y, shape.width * state.scale, shape.height * state.scale);
  }
}

/* The picture is loaded rather than fetched, and it either arrives or the
 * promise rejects. A capture whose image did not load is not drawn dimly or
 * approximated from the sidecar: the caller hears about it.
 */
export function loadCaptureImage(identifier) {
  return new Promise((resolve, reject) => {
    const picture = new Image();
    picture.addEventListener("load", () => resolve(picture));
    picture.addEventListener("error", () => reject(new Error("the capture image did not load")));
    picture.src = captureImagePath(identifier);
  });
}

/* The request body for a gesture over the picture, or null when the gesture
 * said nothing. Every coordinate in it is a pixel of this capture.
 */
export function captureGesture(drag, point) {
  const capture_id = state.captureId;
  if (state.tool === "click" && !drag) {
    return { capture_id, gesture: "click", point: clampPixel(point) };
  }
  if (drag && drag.kind === "box") {
    const from = clampPixel(drag.from);
    const to = clampPixel(drag.to);
    return {
      capture_id,
      gesture: "box",
      rect: {
        x: Math.min(from.x, to.x),
        y: Math.min(from.y, to.y),
        width: Math.abs(to.x - from.x) + 1,
        height: Math.abs(to.y - from.y) + 1,
      },
    };
  }
  if (drag && drag.kind === "lasso" && drag.points.length >= 3) {
    return {
      capture_id,
      gesture: "lasso",
      polygon: drag.points.map((p) => ({ x: p.x, y: p.y })),
    };
  }
  if (drag && drag.kind === "paint" && drag.points.length > 0) {
    return { capture_id, gesture: "paint", points: drag.points };
  }
  return null;
}
