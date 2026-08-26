/* The three views, wired together and started.
 *
 * This page draws and collects gestures. It resolves nothing: every question
 * about what a selection MEANS goes to the server, which answers with the one
 * resolver an agent also reaches. A second resolver in the browser would be a
 * second answer, and agent parity is a law here.
 *
 * The logical view draws the compiler's projection cell by cell. The capture
 * view draws a picture the client took and nothing else — no overlay of the
 * compiler's opinion on top of a photograph, because the whole value of a
 * capture is that it shows what the client drew. Which pixel is which identity
 * is a question for the server, which reads the identity sidecar written with
 * the picture.
 *
 * The third view is the candidate: what the staged operations would produce,
 * drawn by the logical view's own renderer because the server hands it back in
 * the logical view's own shape. It is an unattested document that exists only in
 * this session, and its banner says so.
 *
 * This file is the entry point and holds only what belongs to no single panel:
 * the toolbar wiring and the boot sequence. Everything else is a module, and
 * this is the only file the page loads by name — `index.html` names `app.js`,
 * and every other module arrives through an import from here.
 *
 *   state.js       the one state object, and the questions asked of it
 *   api.js         every request, and the one place a refusal is shown
 *   surface.js     the canvas, and pointer-to-surface arithmetic
 *   logical.js     the compiler's projection: drawing it, gesturing over it
 *   capture.js     the photograph: drawing it, gesturing over its pixels
 *   candidate.js   the candidate: the same renderer, plus what changed
 *   view.js        drawing whichever view is on, fitting it, and its legend
 *   views.js       which view is on, and what that makes the page about
 *   identities.js  the server's answer, written into the selection panel
 *   session.js     what this session holds: packets, and captures
 *   staging.js     the operation vocabulary, and the staged log
 *   parameters.js  the vocabulary's parameters, as typed inputs and back
 *   apply.js       the candidate preview, Apply, and the owner's acceptance
 *   gestures.js    a gesture, from the mouse button to the written packet
 */

import { readProjection, readState, refuse } from "./api.js";
import { installApply } from "./apply.js";
import { installGestures, record } from "./gestures.js";
import { clearSelection } from "./identities.js";
import { buildPalette } from "./logical.js";
import { refreshSession, takeCapture } from "./session.js";
import { installStaging } from "./staging.js";
import { state } from "./state.js";
import { zoomCeiling, zoomFloor } from "./surface.js";
import { draw, fit, renderLegend } from "./view.js";
import { setView } from "./views.js";

/* ------------------------------------------------------------------ wiring */

installGestures();
installStaging();
installApply();

document.getElementById("tools").addEventListener("click", (event) => {
  const button = event.target.closest("button[data-tool]");
  if (!button) return;
  state.tool = button.dataset.tool;
  for (const other of document.querySelectorAll("#tools button")) {
    other.classList.toggle("active", other === button);
  }
});

document.getElementById("views").addEventListener("click", (event) => {
  const button = event.target.closest("button[data-view]");
  if (!button) return;
  setView(button.dataset.view).catch((error) => refuse({ error: "view", detail: String(error) }));
});

document.getElementById("zoom-in").addEventListener("click", () => {
  state.scale = Math.min(zoomCeiling(), state.scale * 1.2);
  draw();
});
document.getElementById("zoom-out").addEventListener("click", () => {
  state.scale = Math.max(zoomFloor(), state.scale / 1.2);
  draw();
});
document.getElementById("reset").addEventListener("click", fit);
document.getElementById("record").addEventListener("click", () => {
  record().catch(() => {});
});
document.getElementById("take-capture").addEventListener("click", () => {
  takeCapture().catch(() => {});
});

/* ------------------------------------------------------------------- boot */

async function boot() {
  const info = await readState();
  document.getElementById("version").textContent =
    `${info.workbench_version} · ${info.session.land_id}`;
  state.projection = await readProjection();
  state.palette = buildPalette(state.projection);
  state.member = state.projection.members[0].member;

  const members = document.getElementById("members");
  for (const member of state.projection.members) {
    const button = document.createElement("button");
    button.textContent = member.member;
    button.dataset.member = member.member;
    button.classList.toggle("active", member.member === state.member);
    button.addEventListener("click", () => {
      state.member = member.member;
      clearSelection();
      for (const other of members.querySelectorAll("button")) {
        other.classList.toggle("active", other === button);
      }
      renderLegend();
      fit();
    });
    members.append(button);
  }

  renderLegend();
  fit();
  await refreshSession(null);
}

boot().catch((error) => refuse({ error: "boot", detail: String(error) }));
