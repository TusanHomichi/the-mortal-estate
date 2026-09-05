/* What this session holds: the packets written, the operations staged, and the
 * captures taken.
 *
 * All three come back from one route — `/api/state` is the session's own account
 * of itself — so one module refreshes all of them. The session directory, the
 * packet list, the staged log and the capture strip are four renderings of a
 * single answer, and splitting them would mean four modules each asking the
 * server the same question and each able to disagree with the others about what
 * came back. The staged log is rendered by the staging panel, which owns how a
 * staged operation looks; what this module owns is that it is re-rendered from
 * the same answer as everything else on the page.
 *
 */

import { readState } from "./api.js";
import { loadCaptureImage } from "./capture.js";
import { clearSelection } from "./identities.js";
import { noteSelections, renderStaged } from "./staging.js";
import { state } from "./state.js";
import { fit, renderLegend } from "./view.js";

export async function refreshSession(highlight) {
  const info = await readState();
  state.captureAvailable = info.capture_available;
  document.getElementById("take-capture").disabled = state.captureInProgress || !state.captureAvailable;
  state.captures = info.captures;
  state.selections = info.selections;
  state.staged = info.staged;
  document.getElementById("session").textContent =
    `${info.session_directory}\n${info.selections.length} packet(s) · ` +
    `${info.operations} log record(s) · ${info.staged.length} staged · ` +
    `${info.applies.length} apply record(s) · ${info.captures.length} capture(s)\n` +
    `revision ${info.repository_revision || "unknown"} (advisory)`;
  const list = document.getElementById("packets");
  list.innerHTML = "";
  for (const id of info.selections) {
    const item = document.createElement("li");
    item.textContent = id === highlight ? `${id}  ← written` : id;
    list.append(item);
  }
  renderCaptureButtons();
  renderStaged(info.staged);
  noteSelections();
}

function renderCaptureButtons() {
  const holder = document.getElementById("captures");
  holder.innerHTML = "";
  for (const taken of state.captures) {
    const button = document.createElement("button");
    button.textContent = taken.capture_id;
    button.classList.toggle("active", taken.capture_id === state.captureId);
    button.addEventListener("click", () => {
      selectCapture(taken.capture_id).catch(() => {});
    });
    holder.append(button);
  }
}

/* The id and the picture it names change together, or neither changes.
 *
 * The picture is loaded FIRST and the id is set only once it is in hand. An id
 * set before the load would name the new capture while `captureImage` still held
 * the previous one — and every gesture in this view is clamped to the picture's
 * bounds and sent with the current id, so a failed load would produce a packet
 * addressed to one capture and measured against another. A confidently wrong
 * packet is the single failure class this tool exists to prevent, and it must not
 * be reachable by an image that simply did not arrive.
 */
export async function selectCapture(identifier) {
  const picture = await loadCaptureImage(identifier);
  state.captureId = identifier;
  state.captureImage = picture;
  clearSelection();
  renderCaptureButtons();
  renderLegend();
  fit();
}
