/* The logical view: the compiler's own projection, drawn cell by cell.
 *
 * This is not a gameplay preview and it never becomes one. It draws what the
 * authoring compiler knows — terrain layers, passability, the structures,
 * landmarks and transitions — as flat shapes on a grid, and the banner above it
 * says so. Every question about what a drawn cell MEANS still goes to the
 * server; this file only puts pixels where the projection says things are.
 *
 * Gesture handling over the grid lives here too, and stops at the request body.
 * Building the body is view knowledge — a box over cells is a rect of cells, a
 * lasso is a polygon in cell space — while sending it is not, so this file
 * returns the body and never sends it. That is also what keeps the drawing
 * layer below the request layer in the import graph instead of tangled with it.
 */

import { state } from "./state.js";
import { canvas, canvasRectOf, clampCell, context } from "./surface.js";

/* ------------------------------------------------------------------ colour */

/* A stable colour per terrain class, spread evenly over the hue circle across
 * every class the projection carries, in sorted order.
 *
 * Hashing the name was the first attempt and it failed on this land: water and
 * grass hashed two degrees apart and the whole map read as one pink field. The
 * view still carries no opinion about what any class LOOKS like — that belongs
 * to a presentation boundary that does not exist yet — but it does owe the
 * owner classes they can tell apart, because telling them apart is the job.
 */
export function buildPalette(projection) {
  const classes = new Set();
  for (const member of projection.members) {
    for (const cell of member.cells) {
      for (const entry of cell.terrain) classes.add(entry.class);
    }
  }
  const sorted = [...classes].sort();
  const palette = new Map();
  sorted.forEach((name, index) => {
    palette.set(name, Math.round((index * 360) / sorted.length));
  });
  return palette;
}

export function colourOf(name, layer) {
  const hue = state.palette.get(name) ?? 0;
  if (layer === "base_terrain") return `hsl(${hue} 38% 74%)`;
  if (layer === "routes") return `hsl(${hue} 55% 52%)`;
  if (layer === "structure_footprints") return `hsl(${hue} 45% 38%)`;
  return `hsl(${hue} 70% 46%)`;
}

/* ----------------------------------------------------------------- drawing */

export function drawLogical(member) {
  if (!member) return;
  const size = state.scale;

  for (const cell of member.cells) {
    const x = state.origin.x + cell.x * size;
    const y = state.origin.y + cell.y * size;
    if (x + size < 0 || y + size < 0 || x > canvas.width || y > canvas.height) continue;
    context.fillStyle = "#e9e6e0";
    context.fillRect(x, y, size, size);
    for (const entry of cell.terrain) {
      context.fillStyle = colourOf(entry.class, entry.layer);
      if (entry.layer === "base_terrain") {
        context.fillRect(x, y, size, size);
      } else if (entry.layer === "routes") {
        context.fillRect(x + size * 0.15, y + size * 0.15, size * 0.7, size * 0.7);
      } else if (entry.layer === "structure_footprints") {
        context.fillRect(x + 1, y + 1, size - 2, size - 2);
      } else {
        context.beginPath();
        context.arc(x + size / 2, y + size / 2, size * 0.22, 0, Math.PI * 2);
        context.fill();
      }
    }
    if (!cell.passable) {
      context.strokeStyle = "rgba(30,30,30,0.45)";
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(x + 2, y + 2);
      context.lineTo(x + size - 2, y + size - 2);
      context.moveTo(x + size - 2, y + 2);
      context.lineTo(x + 2, y + size - 2);
      context.stroke();
    }
  }

  if (size >= 14) {
    context.strokeStyle = "rgba(0,0,0,0.10)";
    context.lineWidth = 1;
    for (let x = 0; x <= member.width; x += 1) {
      const px = Math.round(state.origin.x + x * size) + 0.5;
      context.beginPath();
      context.moveTo(px, state.origin.y);
      context.lineTo(px, state.origin.y + member.height * size);
      context.stroke();
    }
    for (let y = 0; y <= member.height; y += 1) {
      const py = Math.round(state.origin.y + y * size) + 0.5;
      context.beginPath();
      context.moveTo(state.origin.x, py);
      context.lineTo(state.origin.x + member.width * size, py);
      context.stroke();
    }
  }

  drawFeatures(member, size);
  drawPending(size);
}

function drawFeatures(member, size) {
  context.lineWidth = 2;
  for (const structure of member.structures) {
    context.strokeStyle = "#1f3d5c";
    context.strokeRect(
      state.origin.x + structure.x * size,
      state.origin.y + structure.y * size,
      structure.width * size,
      structure.height * size
    );
    markCell(structure.access, size, "#1f5d8c", "A");
    markCell(structure.facade_door, size, "#8c5a1f", "D");
  }
  for (const landmark of member.landmarks) {
    markCell(landmark.at, size, "#2f7a3d", "L");
  }
  for (const transition of member.transitions) {
    markCell(transition.marker, size, "#7a2f6d", "T");
    markCell(transition.access, size, "#7a2f6d", "a");
  }
}

function markCell(cell, size, colour, glyph) {
  const x = state.origin.x + cell.x * size;
  const y = state.origin.y + cell.y * size;
  context.strokeStyle = colour;
  context.lineWidth = 2;
  context.strokeRect(x + 2, y + 2, size - 4, size - 4);
  if (size >= 18) {
    context.fillStyle = colour;
    context.font = `${Math.round(size * 0.42)}px ui-monospace, monospace`;
    context.fillText(glyph, x + 3, y + size - 4);
  }
}

function drawPending(size) {
  if (!state.pending) return;
  context.fillStyle = "rgba(31,93,140,0.34)";
  context.strokeStyle = "#1f5d8c";
  context.lineWidth = 1.5;
  for (const cell of state.pending) {
    const x = state.origin.x + cell.x * size;
    const y = state.origin.y + cell.y * size;
    context.fillRect(x, y, size, size);
    context.strokeRect(x + 0.5, y + 0.5, size - 1, size - 1);
  }
}

/* ---------------------------------------------------------------- gestures */

/* The request body for a gesture over the grid, or null when the gesture said
 * nothing — a lasso of two points, a drag with no tool behind it. Null means
 * "ask the server nothing", which is why the caller checks rather than sending
 * an empty selection and letting the server refuse it.
 *
 * The canvas rectangle is recorded on the way past: it is the browser's account
 * of where on screen this happened, carried in the packet beside the cells and
 * never used to decide what they are.
 */
export function logicalGesture(member, drag, point) {
  if (state.tool === "click" && !drag) {
    const cell = clampCell(point, member);
    state.canvasRect = canvasRectOf(cell, { x: cell.x + 1, y: cell.y + 1 });
    return { member: member.member, gesture: "click", cell };
  }
  if (drag && drag.kind === "box") {
    const from = clampCell(drag.from, member);
    const to = clampCell(drag.to, member);
    state.canvasRect = canvasRectOf(drag.from, drag.to);
    return {
      member: member.member,
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
    const xs = drag.points.map((p) => p.x);
    const ys = drag.points.map((p) => p.y);
    state.canvasRect = canvasRectOf(
      { x: Math.min(...xs), y: Math.min(...ys) },
      { x: Math.max(...xs), y: Math.max(...ys) }
    );
    return {
      member: member.member,
      gesture: "lasso",
      polygon: drag.points.map((p) => ({ x: p.x, y: p.y })),
    };
  }
  if (drag && drag.kind === "paint") {
    const xs = drag.cells.map((c) => c.x);
    const ys = drag.cells.map((c) => c.y);
    state.canvasRect = canvasRectOf(
      { x: Math.min(...xs), y: Math.min(...ys) },
      { x: Math.max(...xs) + 1, y: Math.max(...ys) + 1 }
    );
    return { member: member.member, gesture: "paint", cells: drag.cells };
  }
  return null;
}
