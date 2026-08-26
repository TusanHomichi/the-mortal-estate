/* Preview the candidate, apply the staged set, and record what the owner meant.
 *
 * Three buttons, three routes, and one rule they all obey: what the server
 * answered is shown as it was written. A receipt is printed whole, a rejection
 * is printed whole, and the compiler's refusal appears in the compiler's own
 * words. A summary of a rejection is a paraphrase of the only sentence that
 * matters, and an owner who has to trust a paraphrase is back to trusting the
 * tool instead of the evidence.
 *
 * All three routes run the authoring compiler and cost a moment rather than
 * milliseconds. Each button says so before it is pressed, and says what it
 * actually cost afterwards.
 *
 * **Apply is not promotion, and this panel never lets that blur.** Everything
 * Apply writes lands inside the disposable session directory; accepting a
 * candidate records the owner's intent in the session log and grants nothing.
 * The sentence beside the accept button is not decoration — it is the boundary
 * between this tool and the owner ceremony that does the promoting, and a tool
 * that let an owner think it had promoted something would be the most expensive
 * lie in the repository.
 */

import { acceptCandidate, applyStaged, previewCandidate } from "./api.js";
import { buildDiff } from "./candidate.js";
import { refreshSession } from "./session.js";
import { CANDIDATE, previewingCandidate, state } from "./state.js";
import { fit } from "./view.js";
import { offerCandidate, setView } from "./views.js";

const CANDIDATE_MASTER = "candidate_master";

export function installApply() {
  document.getElementById("preview-candidate").addEventListener("click", () => {
    preview().catch(() => {});
  });
  document.getElementById("apply").addEventListener("click", () => {
    run().catch(() => {});
  });
  document.getElementById("accept").addEventListener("click", () => {
    accept().catch(() => {});
  });
}

/* ---------------------------------------------------------- the candidate */

async function preview() {
  const button = document.getElementById("preview-candidate");
  const cost = document.getElementById("candidate-cost");
  button.disabled = true;
  cost.textContent = "replaying the staged set through the compiler…";
  const started = performance.now();
  let answer;
  try {
    answer = await previewCandidate();
  } catch (error) {
    /* `refuse()` showed the server's own words. The candidate on screen, if
     * there was one, is gone on the server too — so it goes from here as well. */
    cost.textContent = "the compiler did not answer — see the refusal above";
    await discard();
    button.disabled = false;
    return;
  }
  const elapsed = ((performance.now() - started) / 1000).toFixed(1);
  cost.textContent = `${elapsed}s`;
  button.disabled = false;

  const outcome = answer.outcome;
  if (!outcome.accepted || !answer.projection) {
    document.getElementById("candidate-outcome").textContent =
      `REFUSED at stage ${outcome.stage} — ${outcome.operations.length} operation(s) replayed`;
    show("candidate-detail", outcome.detail);
    await discard();
    return;
  }

  state.candidate = answer.projection;
  state.candidateDiff = buildDiff(answer.projection);
  document.getElementById("candidate-outcome").textContent =
    `ACCEPTED — candidate ${outcome.candidate.sha256.slice(0, 12)} at ${outcome.candidate.path}\n` +
    `${outcome.operations.length} operation(s) replayed against base ` +
    `${outcome.base.sha256.slice(0, 12)}`;
  show("candidate-detail", outcome.detail);
  await offerCandidate(true);
  /* A preview the owner asked for is a picture they asked to see, so the view
   * follows. Standing in it already, the picture is refitted instead — the
   * candidate underneath it has been replaced. */
  if (previewingCandidate()) fit();
  else await setView(CANDIDATE);
}

async function discard() {
  state.candidate = null;
  state.candidateDiff = null;
  await offerCandidate(false);
}

/* ---------------------------------------------------------------- Apply */

async function run() {
  const button = document.getElementById("apply");
  const cost = document.getElementById("apply-cost");
  button.disabled = true;
  cost.textContent = "replaying and judging the staged set…";
  const started = performance.now();
  let answer;
  try {
    answer = await applyStaged({ author: "owner" });
  } catch (error) {
    cost.textContent = "the compiler did not answer — see the refusal above";
    button.disabled = false;
    return;
  }
  cost.textContent = `${((performance.now() - started) / 1000).toFixed(1)}s`;
  button.disabled = false;

  const record = answer.record;
  const outcome = document.getElementById("apply-outcome");
  const box = document.getElementById("accept-box");
  if (answer.accepted) {
    outcome.textContent = `APPLIED ${answer.apply_id} — receipt ${answer.path}`;
    box.hidden = false;
    document.getElementById("accept").disabled = false;
    document.getElementById("accept-record").textContent = "";
  } else {
    outcome.textContent =
      `REJECTED ${answer.apply_id} at stage ${record.stage} — ${record.assertion}`;
    box.hidden = true;
  }
  /* The whole record, as the server wrote it. */
  show("apply-record", record);
  state.applied = answer.accepted ? answer : null;
  await refreshSession(null);
}

/* --------------------------------------------------------- the acceptance */

async function accept() {
  const applied = state.applied;
  if (!applied) return;
  const output = applied.record.outputs.find((entry) => entry.role === CANDIDATE_MASTER);
  const answer = await acceptCandidate({
    candidate_sha256: output.sha256,
    apply_id: applied.apply_id,
    note: document.getElementById("accept-note").value,
  });
  document.getElementById("accept").disabled = true;
  document.getElementById("accept-note").value = "";
  show("accept-record", answer.record);
  await refreshSession(null);
}

/* Pretty-printed into a <pre>, never summarised: a receipt and a rejection are
 * both read whole or not read at all.
 */
function show(identifier, record) {
  document.getElementById(identifier).textContent = JSON.stringify(record, null, 2);
}
