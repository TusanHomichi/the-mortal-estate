/* The staging panel: a typed operation, derived from a selection the owner made.
 *
 * Every verb, every parameter name, every closed set of classes and every
 * rejection line in this panel came from `/api/operations`, which is the
 * compiler's own vocabulary document. Nothing here carries a verb table. A copy
 * of one in the browser would be a second statement of what an operation is, and
 * it would be wrong the first time a verb was added and nobody remembered this
 * file — the failure this whole tool is built to refuse.
 *
 * **The vocabulary is fetched once, when it is first wanted, and the cost is on
 * screen before it is paid.** That route runs the authoring compiler. Fetching
 * it at boot would put a program start on the path of a tool whose selection
 * loop is guaranteed to start nothing at all, so the panel opens closed and says
 * what opening it costs. If the compiler cannot answer, the refusal stands and
 * no picker appears: a picker built from nothing would be a list of verbs this
 * tool cannot promise exist.
 *
 * **The dressing class is shown as the bound limit it is.** It ships zero verbs,
 * and the ruling saying why is on screen. An empty group in the picker would
 * read as a list that failed to load rather than a limit somebody decided.
 *
 * **Retracting appends.** The log is the honest record of a working session, so
 * a retraction is a record naming the one it retracts, and the staged list is
 * re-rendered from what the server answers with — never edited in place here.
 */

import { readPacket, readVocabulary, retractOperation, stageOperation } from "./api.js";
import { option, readParameters, renderParameters } from "./parameters.js";
import { ASSET, state, TRUTH } from "./state.js";

/* The cells of the packet the staged operation will derive from, as the server
 * listed them. Replaced whole whenever the chosen selection changes.
 */
let selectedCells = [];

export function installStaging() {
  document.getElementById("vocabulary-load").addEventListener("click", () => {
    loadVocabulary().catch(() => {});
  });
  document.getElementById("verb").addEventListener("change", showVerb);
  document.getElementById("stage-selection").addEventListener("change", () => {
    loadSelectionCells().catch(() => {});
  });
  document.getElementById("stage").addEventListener("click", () => {
    stage().catch(() => {});
  });
}

/* ----------------------------------------------------------- the vocabulary */

async function loadVocabulary() {
  const button = document.getElementById("vocabulary-load");
  const cost = document.getElementById("vocabulary-cost");
  button.disabled = true;
  cost.textContent = "asking the compiler…";
  const started = performance.now();
  try {
    state.vocabulary = await readVocabulary();
  } catch (error) {
    /* `refuse()` has already shown the server's own words. The panel stays
     * closed: there is no vocabulary, so there is nothing honest to pick from. */
    cost.textContent = "the compiler did not answer — see the refusal above";
    button.disabled = false;
    return;
  }
  const elapsed = ((performance.now() - started) / 1000).toFixed(1);
  cost.textContent = `the compiler answered in ${elapsed}s · cached for this page`;
  button.hidden = true;
  document.getElementById("staging-body").hidden = false;
  renderVocabulary();
}

function renderVocabulary() {
  const vocabulary = state.vocabulary;
  document.getElementById("dressing-ruling").textContent =
    `DRESSING — ${vocabulary.dressing.ruling}`;

  const picker = document.getElementById("verb");
  picker.innerHTML = "";
  const truth = document.createElement("optgroup");
  truth.label = "truth — the authored map";
  for (const spec of vocabulary.truth.verbs) {
    truth.append(option(`${TRUTH}:${spec.verb}`, spec.verb));
  }
  picker.append(truth);

  const served = vocabulary.asset.verbs.filter((spec) => spec.served);
  if (served.length > 0) {
    const asset = document.createElement("optgroup");
    asset.label = "asset — image operations with an adapter";
    for (const spec of served) asset.append(option(`${ASSET}:${spec.verb}`, spec.verb));
    picker.append(asset);
  }

  /* The declared asset verbs nothing serves are named as a limit rather than
   * offered as buttons that refuse. */
  const declared = vocabulary.asset.verbs.filter((spec) => !spec.served);
  document.getElementById("asset-limit").textContent = declared.length
    ? `ASSET — ${declared.map((spec) => spec.verb).join(", ")} ` +
      "are declared contracts with no adapter registered in this tree, and are " +
      `not offered here. The registered adapters are: ${vocabulary.asset.adapters.join(", ")}.`
    : "";
  showVerb();
}

function chosen() {
  const [operationClass, verb] = document.getElementById("verb").value.split(":");
  const specs = state.vocabulary[operationClass].verbs;
  return { operationClass, spec: specs.find((row) => row.verb === verb) };
}

/* The verb's own sentence, the assertion it can trip, and its fields. The
 * rejection line is shown beside the picker on purpose: an owner staging an edit
 * should be able to read what the compiler will refuse it for before they stage
 * it, not after.
 */
function showVerb() {
  if (!state.vocabulary) return;
  const { operationClass, spec } = chosen();
  document.getElementById("verb-summary").textContent = spec.summary;
  document.getElementById("verb-rejects").textContent = spec.rejects
    ? `REJECTS — ${spec.rejects}`
    : "REJECTS — this verb declares no single assertion; the compiler judges the " +
      "whole candidate";
  renderParameters(
    document.getElementById("parameters"),
    operationClass,
    spec,
    selectedCells
  );
}

/* ------------------------------------------------------- the selection used */

/* The packets this session holds, and which one the next staged operation
 * derives from. Called whenever the session is re-read, and it lands on the
 * newest packet every time: the session is re-read because a packet was just
 * written, and the packet just written is the one the owner is pointing with.
 * Any other packet in the session is one choice away in the same picker.
 */
export function noteSelections() {
  const picker = document.getElementById("stage-selection");
  const previous = picker.value;
  picker.innerHTML = "";
  for (const identifier of state.selections) picker.append(option(identifier, identifier));
  picker.value = state.selections[state.selections.length - 1] || "";
  document.getElementById("stage").disabled = state.selections.length === 0;
  document.getElementById("stage-reason").hidden = state.selections.length > 0;
  if (picker.value && picker.value !== previous) loadSelectionCells().catch(() => {});
}

async function loadSelectionCells() {
  const identifier = document.getElementById("stage-selection").value;
  const note = document.getElementById("stage-selection-note");
  if (!identifier) {
    selectedCells = [];
    note.textContent = "";
    return;
  }
  const answer = await readPacket(identifier);
  selectedCells = answer.packet.cells;
  note.textContent =
    `${selectedCells.length} cell(s) over ${answer.packet.scene.member}, ` +
    `pointed by ${answer.packet.author} — prefilled below`;
  showVerb();
}

/* ------------------------------------------------------------- the staging */

async function stage() {
  const { operationClass, spec } = chosen();
  const { values, adapter } = readParameters(document.getElementById("parameters"));
  const answer = await stageOperation({
    selection_id: document.getElementById("stage-selection").value,
    class: operationClass,
    member: state.vocabulary.truth.member,
    verb: spec.verb,
    parameters: values,
    adapter,
    comment: document.getElementById("stage-comment").value,
    author: "owner",
  });
  document.getElementById("stage-comment").value = "";
  state.staged = answer.staged;
  renderStaged(answer.staged);
}

export function renderStaged(records) {
  const list = document.getElementById("staged");
  list.innerHTML = "";
  document.getElementById("staged-count").textContent =
    `${records.length} operation(s) standing, in log order`;
  for (const record of records) {
    const item = document.createElement("li");
    const head = document.createElement("span");
    head.className = "kind";
    head.textContent = `${record.record_id}  ${record.class} · ${record.verb}`;
    const detail = document.createElement("span");
    detail.className = "detail";
    detail.textContent =
      JSON.stringify(record.parameters) +
      (record.adapter ? ` · adapter ${JSON.stringify(record.adapter)}` : "") +
      `\nfrom ${record.selection_id} · staged by ${record.author}` +
      (record.comment ? `\n${record.comment}` : "");
    const reason = document.createElement("input");
    reason.type = "text";
    reason.className = "retract-reason";
    reason.placeholder = "why — recorded on the retraction";
    const button = document.createElement("button");
    button.textContent = "retract";
    button.addEventListener("click", () => {
      retract(record.record_id, reason.value).catch(() => {});
    });
    item.append(head, detail, reason, button);
    list.append(item);
  }
}

async function retract(identifier, reason) {
  const answer = await retractOperation({
    record_id: identifier,
    reason,
    author: "owner",
  });
  state.staged = answer.staged;
  renderStaged(answer.staged);
}
