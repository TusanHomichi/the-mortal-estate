/* The server's answer, written into the selection panel.
 *
 * Everything in this file renders a payload that arrived from the resolver; it
 * computes nothing about identity, coverage, or ambiguity, because the browser
 * is not allowed to have an opinion about any of the three. That is why it is
 * its own module: a rendering file that imports no request function cannot
 * quietly start resolving, and the next person to open it can see that in one
 * screen.
 *
 * Ambiguity is shown loudly and never resolved. When a selection carries more
 * than one candidate identity the panel says so in the owner's face, because
 * the consumer's rule — ask which one you meant, never pick — only works if the
 * person recording the packet knows there was a choice.
 */

import { capturing, state } from "./state.js";

export function renderResolution(answer) {
  const summary = document.getElementById("summary");
  summary.textContent =
    `${answer.gesture} over ${answer.member} · ${answer.cells.length} cell(s)` +
    (capturing() ? ` · over capture ${answer.capture_id}` : "");
  if (answer.ambiguous) {
    const flag = document.createElement("span");
    flag.className = "ambiguous";
    flag.textContent =
      "AMBIGUOUS — this selection carries more than one candidate identity. " +
      "A consumer must ask which one you meant, never pick.";
    summary.append(flag);
  }
  const list = document.getElementById("identities");
  list.innerHTML = "";
  for (const record of answer.semantic) {
    const item = document.createElement("li");
    const head = document.createElement("span");
    head.innerHTML =
      `<span class="kind">${record.kind}</span> ${record.identity}` +
      ` · selection ${record.coverage.selection_coverage.toFixed(2)}` +
      ` · identity ${record.coverage.identity_coverage.toFixed(2)}`;
    item.append(head);
    const detail = Object.entries(record.detail)
      .filter(([, value]) => typeof value !== "object")
      .map(([key, value]) => `${key}=${value}`)
      .join("  ");
    if (detail) {
      const line = document.createElement("span");
      line.className = "detail";
      line.textContent = detail;
      item.append(line);
    }
    list.append(item);
  }
  renderObserved(answer.observed || []);
}

/* What the photographed frame happened to be showing under the gesture. Runtime
 * facts about one instant, listed beside the address and never part of it — so
 * they get their own list, their own heading, and both disappear when there is
 * nothing to say.
 */
function renderObserved(records) {
  const heading = document.getElementById("observed-heading");
  const note = document.getElementById("observed-note");
  const list = document.getElementById("observed");
  list.innerHTML = "";
  const shown = records.filter((record) => record.kind !== "tile");
  heading.hidden = shown.length === 0;
  note.hidden = shown.length === 0;
  for (const record of shown) {
    const item = document.createElement("li");
    item.textContent =
      `${record.kind} ${record.identity} at ${record.coordinate.x},${record.coordinate.y}`;
    list.append(item);
  }
}

/* No selection, and nothing on screen claiming otherwise. Called whenever the
 * ground under the current answer moves — a new member, a new capture, a change
 * of view — because a resolved identity list from a surface you are no longer
 * looking at is a confident wrong answer.
 */
export function clearSelection() {
  state.pending = null;
  state.covered = null;
  state.gesture = null;
  document.getElementById("record").disabled = true;
  renderObserved([]);
}
