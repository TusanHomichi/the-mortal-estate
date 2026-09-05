/* Which view is on, and what every panel on the page is therefore about.
 *
 * Switching view switches the subject of the whole page — the banner, the
 * toolbar groups, the legend, what a gesture means and which route answers it —
 * so it is one function rather than a flag each panel reads for itself. It lives
 * in its own module because two other modules switch views: the entry point,
 * when the owner presses a button, and the candidate preview, which must leave
 * the candidate view the moment there stops being a candidate to look at.
 *
 * The one rule the switch enforces beyond visibility: a view whose subject does
 * not exist is not entered. There is no candidate until a preview has produced
 * one, and a candidate view drawn from nothing would be a blank grid the owner
 * could mistake for an empty land.
 */

import { renderBinds } from "./candidate.js";
import { clearSelection } from "./identities.js";
import { selectCapture } from "./session.js";
import { CANDIDATE, CAPTURE, LOGICAL, state } from "./state.js";
import { fit, renderLegend } from "./view.js";

export async function setView(requested) {
  /* The rule above, enforced here rather than left to the disabled button: the
   * candidate view is only the candidate view while there is a candidate. */
  const view = requested === CANDIDATE && !state.candidate ? LOGICAL : requested;
  state.view = view;
  /* A candidate carries the members the compiler projected for it. If the one
   * the owner was looking at is not among them, the view moves to one that is
   * rather than drawing an empty grid where a member used to be. */
  if (view === CANDIDATE) {
    const carried = state.candidate.members.some((member) => member.member === state.member);
    if (!carried) state.member = state.candidate.members[0].member;
  }
  /* A member the candidate does not carry cannot be pointed at in the candidate
   * view: there is no candidate document for it, and a button that led to an
   * empty grid would read as a member that had been emptied. */
  for (const button of document.querySelectorAll("#members button")) {
    button.classList.toggle("active", button.dataset.member === state.member);
    button.disabled =
      view === CANDIDATE &&
      !state.candidate.members.some((member) => member.member === button.dataset.member);
  }
  document.getElementById("banner-logical").hidden = view !== LOGICAL;
  document.getElementById("banner-capture").hidden = view !== CAPTURE;
  document.getElementById("banner-candidate").hidden = view !== CANDIDATE;
  document.getElementById("members").hidden = view === CAPTURE;
  document.getElementById("captures").hidden = view !== CAPTURE;
  /* A packet binds the exact bytes it was taken against. A candidate's bytes are
   * replaced by the next preview, so a packet bound to them would be stale by
   * design — worse than no packet. The reason is on screen beside the disabled
   * button, not left for the owner to discover by pressing it. */
  document.getElementById("record-refusal").hidden = view !== CANDIDATE;
  for (const button of document.querySelectorAll("#views button")) {
    button.classList.toggle("active", button.dataset.view === view);
  }
  clearSelection();
  renderBinds([]);
  if (view === CAPTURE && state.captures.length > 0 && !state.captureId) {
    await selectCapture(state.captures[state.captures.length - 1].capture_id);
    return;
  }
  renderLegend();
  fit();
}

/* The candidate view is enabled by a preview that produced a candidate, and
 * disabled by one that did not. When it is disabled while the owner is standing
 * in it, they are moved back to the accepted land rather than left looking at a
 * document the server has already discarded.
 */
export async function offerCandidate(available) {
  document.getElementById("view-candidate").disabled = !available;
  if (!available && state.view === CANDIDATE) await setView(LOGICAL);
}
