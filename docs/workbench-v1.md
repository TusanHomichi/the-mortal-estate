---
last_updated: 2026-09-05
revision: 4
status: Implemented under G9 mutation authorization; compiler bridge is the only process-spawning module. Owner acceptance remains at the recorded stop point.
public_safe: true
summary: Workbench staged operations, candidate preview, atomic Apply, image operations, agent CLI, and promotion limits.
routes:
  - tools/workbench/operations.py
  - tools/workbench/replay.py
  - tools/workbench/apply.py
  - tools/workbench/stage.py
  - tools/workbench/bridge.py
  - tools/workbench_integrity.py
  - tools/workbench/imageops/**
  - crates/tme-authoring/src/operations.rs
  - crates/tme-authoring/src/replay.rs
  - crates/tme-authoring/src/cli.rs
  - tests/test_workbench_apply.py
  - tests/test_workbench_operations.py
  - tests/test_workbench_imageops.py
  - content/authoring-fixture/asset-provenance.json
---

# Workbench V1 — staged operations and Apply

V0 proved the address: the owner points, and an agent receives an exact, stable,
machine-resolvable place. V1 earns the edit. The owner points, stages a typed
operation against what they pointed at, sees the candidate the whole staged log
produces, and applies it — atomically, or not at all.

Its governing authority is the successor design spec *The Mortal Estate —
Workbench Selection Bridge*, sections 6 through 10, and the G9 gate that
authorized mutation through this surface. **Section 8.1 binds without exception:
the existing promotion gate is not weakened by anything here.**

## What this document owns, and what V0 keeps

One fact, one owner. The split is by subject, not by chronology:

| [Workbench V0](workbench-v0.md) owns | This document owns |
| --- | --- |
| the selection packet and its fields | the session directory, whole |
| semantic identities, ranking, ambiguity | the staged-operation log and every record kind in it |
| the capture path, its two routes, the identity raster | the operation classes and the verb vocabulary |
| fail-closed staleness per bound digest | Apply, its receipts, and its rejections |
| the agent read path (`resolve.py`) | the agent write path (`stage.py`, `apply.py`) |
| what V0 does not do | the candidate lifecycle and image operations |

Where V0 described the session directory as a place packets land, this document
describes it whole, and V0 points here.

## What V1 does not do

Each is a bound limit with a reason, not an apology.

- **It does not promote.** Nothing Apply writes leaves the disposable session.
  Turning an accepted candidate into an accepted master is an owner ceremony —
  measured below, and deliberately unchanged.
- **It creates and destroys nothing.** The accepted contract pins the structure,
  landmark, and transition programs exactly. V1 edits the features the fixture
  already declares; growing the programs is a source change and a
  re-attestation.
- **It edits an authored land, and creates no authority over one** (ruling D2).
  No canon, no names, and no geography authority is created by anything staged or
  applied. V1 shipped against the synthetic fixture; since slice S1 the compiler
  carries a table of lands and every entry point names the land it addresses, so
  the loop runs unchanged over the identity proof's land — which is the land the
  runtime serves, and therefore the land where an applied edit reaches play.
- **It ships zero dressing verbs.** See ruling 2.
- **It has no asset view.** V1 ships the asset operation, the adapter contract,
  and the preservation rule; a surface to point at pixels in is the slice after
  this one. A commit mask is stated (`stage.py mask`), not drawn.
- **It calls no hosted model and starts no network connection.** The one adapter
  registered is local, deterministic, and standard-library.
- **It edits one member.** A land declares which of its members has the
  candidate entry point, and a truth operation against any other is refused by
  name rather than by a confusing later diagnostic. The compiler's projection
  carries that member, so nothing in the tool holds a second opinion about which
  member is editable.

## The five rulings

The genesis plan gave this slice five plan-level decisions to rule on with
evidence. Each is stated with the evidence that decided it.

### 1. The operation verb set (spec §13.1)

**Ruled: six verbs, derived from the master's object model, each with an
acceptance case and a rejection case in the compiler's own corpus.**

The spec binds the verb set with two constraints: every verb must be expressible
against the current master without inventing a parallel format, and every verb
must have a validator failure it provably triggers. The evidence that shaped the
set is `crates/tme-authoring/src/contract/`, which declares each member's
programs **exactly**:

- the structure, landmark and transition programs pin which
  features exist, their scopes, their roles and their pairings, and
  `compile_member` asserts set equality against each. **So no verb creates or
  destroys a feature** — such a verb could only ever produce a document the
  compiler rejects by construction. `place_landmark` and `remove_landmark`,
  which the brief expected, do not exist for that reason.
- `layer_of` maps every tile class to exactly one authored layer, so a verb
  writes into the layer its class belongs to or is refused naming the layer it
  belongs to instead.
- The façade door is **derived** (`compile::facade_door`), not authored. So
  there is no `set_door`; there is `set_structure_access`, and the door follows.

That leaves six verbs, and three layers that no verb touches at all because they
are derived: `structure_footprints` (the union of the structure objects'
rectangles), `landmark_marks` (the marker each program pins), and `passability`
(`compile::cell_is_passable` over the three layers that decide it). The replay
refreshes them and the compiler independently asserts them — so a wrong refresh
is a rejected candidate, never an accepted lie. Two mutants hold that:
`replay::tests::a_terrain_edit_without_the_passability_refresh_is_rejected` and
`::a_structure_move_without_the_footprint_refresh_is_rejected`.

### 2. Dressing edits (spec §13.5)

**Ruled: the `dressing` class exists in the vocabulary, its downward-honest rule
is specified, and it ships zero verbs.**

The authored presentation layer is not bound to Workbench edits. The browser
loads external candidate packets; those packets are not accepted editable
masters in this contract. There is no authored dressing fact for these verbs to
target ([presentation direction](presentation-direction.md#candidate-assets)).

The rule stands, written down, for the slice that binds it: *a dressing edit may
not imply a gameplay fact that is not true* — it may not paint a walkable-looking
surface over a blocked cell, a door where no transition exists, or cover that
does not occlude. That is a blocking check on dressing operations when they
exist, not a style note.

Staging a dressing operation today is **refused with the reason**
(`tests/test_workbench_operations.py::test_a_dressing_operation_is_refused_with_the_ruling`).
An empty picker would have been the dishonest version of the same fact.

**No honest dressing target was found.** The one candidate — the grid view's
colours — is derived, so authoring it would create a presentation fact in a tool
that is forbidden to be a second renderer.

### 3. Asset edits, image operations, and the state-3 exemplar (spec §9)

**Ruled: the five operation records ship as typed contracts; one reference
adapter is implemented, local and deterministic; the preservation rule is
enforced project-side; and a tiny tracked synthetic editable master is added so
the rule has something real to protect.**

No hosted adapter and no generative model — out of scope for V1 and an owner
decision besides. `edit_region` has one adapter, `palette_fill`. The other four
verbs are declared contracts with nothing registered to serve them, and
executing one says exactly that rather than returning an empty result.

The load-bearing half is the preservation rule, and its ruling changed during
implementation on evidence:

> **Restoration is the rule; the count is provenance, not a verdict.**

The first form of this ruling refused any operation whose adapter wrote outside
the commit mask. That was wrong, and the reference adapter proved it in the
first end-to-end run: `palette_fill` fills the whole context image it is handed —
deliberately, because an adapter that respected the mask itself would prove
nothing — so every edit was refused. And a blending model is *supposed* to return
a context image whose every pixel differs a little; there is no honest threshold
between "blended" and "scribbled". A rule that refused both would refuse every
generative adapter the contract exists to accommodate.

What protects accepted work is that none of those pixels can reach the result.
`preserve.composite` starts from a copy of the source and copies in **only** the
mask's pixels; it never diffs the adapter's output and repairs what it finds. The
count of what the adapter touched outside the boundary is recorded on the
receipt, because what an adapter did is provenance.

**The state-3 exemplar.** There were no tracked assets at all, so V1 adds one:
`content/authoring-fixture/fixture-swatch.png`, 32×24, four neutral values,
depicting nothing, written once by this project's own standard-library PNG
encoder, with `content/authoring-fixture/asset-provenance.json` carrying its
digest, its palette, its synthesis in words, and a bounded authority block. It is
a **lane-authored synthetic editable master**, and its provenance record says so:
`accepted_visual_master: false`. Calling it owner-accepted would have fabricated
exactly the approval the promotion gate exists to require — the same reason the
fixture's own receipt carried a lane attestation until the owner accepted it at
G4X.

It has **one** anchor, not two, and that is proportionality (P10) rather than a
weakened rule: no runtime loads it, no projection derives from it, and nothing
grants authority on its basis. The master `.tmj` is double-anchored because a
compiled land reaches the runtime. A swatch reaches nothing.

### 4. Candidate lifecycle (spec §10)

**Ruled: states 1–3 in V1; states 4 and 5 out of scope.**

- **State 1, experiment** — a staged operation and a preview. Ignored working
  state, no provenance requirement, deleted freely. Costs one command.
- **State 2, review candidate** — an applied candidate: the candidate master,
  its projection, any edited assets, and a receipt naming every base digest, the
  operation set, and every output's digest. Still ignored, still not tracked.
- **State 3, owner-accepted editable master** — recorded as a **typed intent**: a
  `candidate_accepted` record naming the candidate's digest and the Apply it came
  from, carrying a `grants` block in which every field is `false`. The tracked
  write, the re-signed receipt and the changed digest constant are the owner
  ceremony, outside this tool, and the record is what the owner carries into it.
- **States 4 and 5** — deterministic export and promoted, verified asset. Both
  exist today for the master (`cargo run -p tme-authoring`, and the promotion
  gate) and V1 touches neither.

Certification, provenance sealing and full verification attach at states 3
through 5 and **must not** attach at 1 and 2. Using the Workbench requires no
verification run at all; a preview costs 0.4 s and an Apply 0.3 s (measured
below).

### 5. The second promotion anchor (spec §13.4)

**Ruled: do not change it. Record the friction honestly, measured once.**

The gate is double-anchored: a signed receipt on disk, and that land's
`master_digest` in reviewed Rust source. Every accepted truth edit therefore costs a source change
and a rebuild. Measured, by performing the whole ceremony by hand on this machine
for one accepted candidate and reverting it:

| Step | Cost |
| --- | --- |
| 1. write the candidate over the master | 0.01 s |
| 2. re-sign `promotion.json` | 0.03 s |
| 3. change the land's `master_digest` in Rust source | 0.01 s |
| 4. rebuild and regenerate the three projections | 1.6 s |
| 5. prove it (`--check` plus the crate's own tests) | 3.6 s |
| **total, mechanically** | **5.2 s** |

**The stopwatch is not the friction.** Five seconds of machine time is nothing.
What the measurement actually found is the part nobody writes down:

1. **It is a six-file commit across two boundaries** — the master, three
   generated projections, the receipt, and reviewed Rust source — and every one
   of them must move together or the gate fails closed. That is the arrangement
   working, and it is also what makes an accepted edit a deliberate act rather
   than a save.
2. **It sweeps the mutant corpus.** The first attempt failed two tests, both
   because a check pinned a coordinate the edit moved:
   `an_unpainted_landmark_marker_is_rejected` cleared a marker tile at the ruin's
   old cell — which after the move was already empty, so the mutant was
   **accepted** and a check that had been binding quietly stopped binding
   anything; and `terrain_carries_the_authored_layer_each_class_belongs_to`
   asserted the terrain stack of that same cell. Both are now derived from the
   landmark they are about rather than written down, so the corpus survives a
   move. **That is the finding worth keeping**: an accepted geography edit can
   silently disarm a coordinate-pinned mutant, and the failure mode is a green
   suite proving less than it did yesterday.

The recommendation stands: revisit the SHAPE of the second anchor when there is
real usage evidence, and never its existence. There is no evidence here that it
is too expensive — there is evidence that the expensive part is the corpus, and
that the answer to that is to write mutants about what they are about.

## The three operation classes

| Class | Target | Owner of its vocabulary | Verbs in V1 |
| --- | --- | --- | --- |
| `truth` | the authored master document | the authoring compiler | 6 |
| `dressing` | the presentation layer | — | 0, by ruling 2 |
| `asset` | an editable master asset | the project's image operations | 5 declared, 1 served |

An operation declares its class. A single owner gesture may stage operations in
more than one class; **Apply keeps their routing separate and their acceptance
joint** — the map edit and the picture edit in one Apply either both land or
neither does.

### Why the truth vocabulary lives in the compiler

A verb is a statement about the authored document's object model. That model is
declared once (`contract.rs`) and asserted once (`compile.rs`). A verb set
defined in the tool would be a second opinion about what an authored member is
made of, and a Python re-implementation of the derived layers would be a second
implementation of the compiler's semantics — the false-green failure the
authoring standard exists to kill.

So the Workbench owns the session, the log, the ordering and the digests, and the
compiler owns the document. The Workbench **never inspects a truth operation's
parameters**; it passes them through opaquely and the compiler parses, applies
and judges them.

## The verb table

Read from the compiler itself with `python3 tools/workbench/stage.py verbs`, or
`cargo run -p tme-authoring -- describe-operations --land <id>`. The verbs are
the same for every land; the class sets a parameter draws from are the addressed
member's own. Every row's rejection is a
test in `crates/tme-authoring/tests/operation_replay.rs`, and a test in that file
asserts every published verb has both an acceptance and a rejection there.

| Verb | Parameters | Rejection it provably trips |
| --- | --- | --- |
| `set_terrain` | `cells`, `class` (from the base-terrain set) | watering over a structure's access cell — *"structure fixture_structure_north access cell is blocked"*; and sealing a walkable pocket — *"the surface has N walkable cells no one can reach"* |
| `set_route` | `cells`, `class` (a route class, or null to clear) | clearing the route under the arrival — *"the arrival landmark must stand on an authored route cell"* |
| `move_structure` | `structure_id`, `to` | moving an isolated building onto town ground — *"structure … is scoped \"isolated\" but its footprint at 8,9 disagrees"* |
| `set_structure_access` | `structure_id`, `cell` | an access cell inside the footprint — *"access cell must touch exactly one footprint cell, not 2"*; and one nowhere near it — *"… not 0"* |
| `move_landmark` | `landmark_id`, `to` | onto open water — *"landmark fixture_ruin_marker stands on a blocked cell"* |
| `set_transition_endpoint` | `transition_id`, `marker`, `access` | separating the access cell from its marker — *"access cell must be cardinally adjacent to its marker"* |

The vocabulary also refuses, before any candidate exists: an unknown verb (naming
the whole vocabulary), an unknown parameter, a cell outside the envelope, a
feature the member does not carry, a class that belongs to another layer, a
non-truth class, and a member with no candidate entry point.

A parameter that draws from a closed set carries **its own choices** in the
published table, so the interface builds a picker that cannot produce a value the
compiler would refuse — and reads no prose to do it.

## The session directory

```
.workbench/sessions/<session-id>/
  manifest.json                     the session, its base digests, its authority, its retention
  operations.jsonl                  the one log, appended to, in order
  selections/sel-NNNN.json          the packets           (V0)
  masks/sel-NNNN.pbm                selection masks, over CELLS of the land   (V0)
  commit/<name>.pbm                 commit masks, over PIXELS of a picture    (V1)
  captures/cap-NNNN/…               a capture and its sidecar   (V0)
  preview/candidate-member.tmj      the candidate, replaced by every preview
  preview/candidate_projection.json its logical view
  preview/operations.json           exactly what was replayed
  apply/apply-NNNN/candidate-member.tmj
  apply/apply-NNNN/candidate_projection.json
  apply/apply-NNNN/operations.json
  apply/apply-NNNN/assets/op-NNNN.png
  apply/apply-NNNN/receipt.json
  apply/apply-NNNN.rejection.json   for an Apply that was refused
```

Everything is plain, diffable and agent-writable. The manifest's `authority`
block states what a session may do: `staged_operations` and `apply` are true;
`tracked_content`, `runtime_input` and `promotion` are false.

**Nothing written here can leave.** Every V1 artifact path is made by
`Session.artifact`, which refuses a path that escapes the session directory. That
is what makes "the Workbench cannot write tracked content" a property of the code
rather than of every caller being careful, and it has its own mutant:
`test_the_session_refuses_to_address_anything_outside_itself`.

### One log, six record kinds

V0 established `operations.jsonl` with two kinds and an `operation` field that
was always null. V1 fills that field and adds four kinds beside them. One file,
one order, because **the order is the semantics**: replay is the log, in the
order the log has.

| Kind | What it says |
| --- | --- |
| `selection_recorded` | a packet was written (V0) |
| `owner_comment` | the owner said something, verbatim (V0) |
| `operation_staged` | an operation was proposed |
| `operation_retracted` | a standing operation was withdrawn |
| `apply_recorded` | an Apply happened, and where its record is |
| `candidate_accepted` | the owner accepted a candidate — as intent |

```json
{"schema_version":1,"kind":"operation_staged","record_id":"op-0007",
 "recorded_at":"2026-08-20T18:00:00Z","author":"owner","selection_id":"sel-0001",
 "operation":{"class":"truth","member":"surface","verb":"move_landmark",
              "parameters":{"landmark_id":"fixture_ruin_marker","to":{"x":6,"y":11}},
              "adapter":null},
 "comment":"the ruin marker reads one cell too far west"}
```

**The owner's gesture and the agent's proposal are the same record, differing
only in `author`.** That is the agent-parity law in one field, and it is asserted
structurally.

**Retraction appends; it never deletes.** What was tried and dropped is part of
what happened in a session. The *effective set* — every staged operation still
standing, in log order — is a derivation both preview and Apply use, so "what the
preview showed" and "what Apply did" are one code path. A retraction naming
nothing, or naming something already retracted, is refused rather than ignored.

### Why an operation must name a selection

`selection_id` is required. An operation is an act upon an address: the packet is
what binds the edit to a set of digests that Apply re-verifies, and without one
the shared-pointing product requirement degrades into an agent asserting
coordinates it typed. An agent with no packet writes one first —
`stage.py point` does exactly that, through the same code path the browser uses.

### Retention

Unchanged from V0 and owned by [the working-root
policy](working-root-policy.md#the-retention-ruling): sessions are disposable,
`rm -rf .workbench/` is always safe, and cleanup is fourteen days or the most
recent ten. Candidates and receipts live inside a session and share its
retention — a candidate is a function of the log, and the log is what persists.

## Apply

```bash
python3 tools/workbench/apply.py .workbench/sessions/<id>
python3 tools/workbench/apply.py <session> --json
```

1. **Re-verify every bound digest** — the session's base binding and the full
   bound set of every packet the staged operations derive from. A capture-view
   packet binds eight files and all eight are checked. Any staleness aborts
   before anything is computed.
2. **Replay the complete staged truth set**, in log order, against a copy of the
   accepted master, made by the compiler.
3. **Judge**: truth output through the compiler's own semantics
   (`validate-candidate`), asset output through the project's preservation step.
4. **On success**: write the candidate master, the candidate projection, every
   edited asset, and a receipt naming the base digests, the operation set, the
   outputs and their digests.
5. **On any failure**: write the rejection record, and nothing else.

**Atomicity is a rename, not a promise.** Everything is built inside a pending
directory no reader is told about, and becomes `apply/apply-NNNN` in one
`os.replace`. A failure removes it entirely. There is no partial apply, no "apply
the ones that passed", and no repair pass that silently drops a failing
operation — proven by
`test_a_rejected_apply_writes_the_rejection_record_and_nothing_else`, which
compares the session's whole file set before and after, and by
`test_a_rejected_apply_leaves_the_carried_tree_byte_identical`, which compares
the digest of every regular carried file in the repository. Both that test and
`tools/workbench_demo.py` use `tools/workbench_integrity.py`, which delegates file
selection to `boundary_common.carried_files`: tracked files and nonignored
untracked files. Git owns ignore semantics and nested repository boundaries;
there is no second ignore parser or recursive walk through disposable output.
Symlinks are not followed. The clean-copy runner creates a fresh Git index over
its exported sources before testing, so the same selector applies there.
Inventory failures fail the proof; the demo raises on changed files instead of
merely printing a warning. Scratch-tree mutants prove that tracked and untracked
source edits, additions, and deletions are detected while dependency output and
nested worktrees do not affect the snapshot.

**Apply does not promote.** The receipt carries a `grants` block in which
`promotion`, `tracked_write`, `runtime_input`, `receipt_resigned` and
`digest_constant_changed` are all false, and each of those is asserted.

### What a rejection says

The stage it failed at, the operation, the assertion **in the validator's own
words**, and the whole detail record. Attribution is honest about its own
strength:

- a **replay** refusal names its record, because the replay knows which verb it
  was applying;
- a **validator** refusal does not — the compiler judges the candidate document
  as a whole and has no idea which staged operation put the cell there. So Apply
  replays growing prefixes of the staged set and reports **the earliest prefix
  the compiler will not accept**, and the record says in as many words that a
  later operation can repair what an earlier one broke, so this is where the
  trouble starts rather than necessarily the only place it lives. Bounded at
  sixteen operations; past that the rejection stands unattributed rather than
  costing minutes.

## The compiler bridge

Only `tools/workbench/bridge.py` starts a program: the authoring compiler.
Four subcommands,
each writing one JSON document to stdout:

| Command | Answers |
| --- | --- |
| `tme-authoring describe-operations --land <id>` | the truth-operation vocabulary |
| `tme-authoring validate-candidate --land <id> <document>` | would this candidate pass? |
| `tme-authoring project-candidate --land <id> <document>` | the candidate's logical view |
| `tme-authoring replay --land <id> --operations <ops> --output-dir <dir> --expect-base-sha256 <hex> [--validate] [--project]` | the candidate this log produces |

`--land` is required at every one of them and never defaulted: the compiler
carries more than one land, and a tool that guessed would eventually edit a
document nobody asked about. `--member` is optional, because the compiler derives
it from the land's single candidate entry point — a derivation from the contract,
not a guess. The session's manifest binds the land when the session opens, so
the tool passes what the session is an edit OF rather than what it was last
pointed at.

Exit codes: **0** the answer is yes, **1** the answer is no with the reason on
stdout, **2** the request could not be understood. Distinguishing 1 from 2 is
load-bearing: "the candidate is rejected" and "the validator never ran" are
different facts, and a tool that conflated them would report a clean tree as
proven.

`--expect-base-sha256` is required and never defaulted, all the way down.
Replaying against bytes the caller did not expect is how a stale session quietly
edits a document nobody looked at.

**None of them grants authority.** `validate-candidate` reads no receipt and
consults no reviewed digest. `replay` reads the accepted master as bytes and
writes a candidate wherever its caller says, which is always inside a session.
The one subcommand that writes tracked bytes — the default build — is not
reachable from the Workbench at all, and a test reads the bridge's source to
prove it names none of its flags.

## Image operations and the preservation rule

Five operations, as typed contracts with an adapter block
(`tools/workbench/imageops/`):

| Operation | Served in V1 |
| --- | --- |
| `edit_region` | yes, by `palette_fill` |
| `generate_asset` | declared; no adapter registered |
| `animate_asset` | declared; no adapter registered |
| `normalize_pixel_grid` | declared; no adapter registered |
| `compare_candidates` | declared; accepts no adapter by policy — presenting work for judgement is the project's own act |

An adapter is never an architecture authority: it does not define the operation
set, does not own the candidate lifecycle, does not decide what gets promoted,
and can be replaced without touching anything above it. What it needs beyond the
shared fields rides in its own typed block.

**The rule:**

1. The adapter receives a **context image** — the commit mask's bounding box
   grown by a margin — because context is what makes an edit blend.
2. Only the **commit mask's** pixels may replace accepted source pixels.
3. Everything outside is taken from the source **by construction**: the composite
   starts from a copy of the source and copies in only the mask's pixels. It
   never diffs the adapter's output and repairs what it finds, because a
   compare-and-fix step is correct only if its comparison is, which converts a
   structural guarantee into a code-review question.
4. What the adapter changed outside the boundary is **counted and recorded** on
   the receipt as provenance, and is not a verdict.
5. Widening the commit boundary is an explicit owner act with its own operation.

### Mutant receipts

| Class | Mutant | What caught it |
| --- | --- | --- |
| Preservation, adapter side | an adapter paints the entire context image a garish colour | the result is byte-identical to the source outside the mask, all 78 out-of-mask writes discarded, and the edit stands with exactly the mask's 12 pixels changed |
| **Preservation, project side (the blocking check)** | `preserve.composite` replaced with one that hands back the adapter's output | `preserved_outside` fails and `run_edit_region` refuses to return a result — *"the preservation step did not hold and no result is returned"* |
| Staleness | the source asset's bytes move | refused naming the source and both digests |
| Staleness | the commit mask's bytes move | refused naming the mask, independently |
| Adapter contract | an adapter returns the wrong dimensions | refused |
| Mask | an empty commit mask | refused — nothing may be committed |
| Derived layers | a replay that skips the passability refresh | *"the passability annotation is stale at 2,1"* |
| Derived layers | a replay that skips the footprint refresh | *"structure objects and the structure_footprints layer describe different cells"* |
| Atomicity | an Apply rejected at the validate stage | the carried tree is byte-identical and the session gains exactly one file |
| Atomicity | an Apply whose asset edit fails while its map edit passes | nothing is written but the rejection record |
| Write guard | a caller addressing tracked content through `Session.artifact` | refused — *"outside the session directory"* |
| Type level | `CandidateReport` converted into a `Member` | the `compile_fail` doctest, verified by flipping the fence: `error[E0277]: the trait bound Member: From<CandidateReport> is not satisfied` |

`palette_fill` is deliberately mask-blind: it fills everything it is handed. Run
through the real registry it behaves exactly like the hostile stand-in, which is
the proof that preservation is enforced by the project rather than by an
adapter's good manners.

## Agent parity

Nothing the Workbench can do is unavailable to an agent working on the same
files. Two ways, and both are proven end to end:

**With file tools alone.** A session is plain files. Staging is appending one
JSON line to `operations.jsonl`; retracting is appending another.
`test_an_operation_staged_with_a_text_editor_applies_identically` writes that
line by hand — no CLI, no browser, no import — and asserts it produces exactly
the candidate the Workbench produces for the same operation.

**With one command.** This example uses the default synthetic fixture.
`open` and `verbs` select a land through the global `--projection` option,
placed before the subcommand. `verbs` does not accept `--session`; the other
session commands attach to the session's recorded projection. For example:

```bash
python3 tools/workbench/stage.py \
  --projection content/lands/identity-proof/generated/workbench_projection.json verbs
```

The fixture loop:

```bash
python3 tools/workbench/stage.py open
python3 tools/workbench/stage.py verbs
python3 tools/workbench/stage.py point   --session <session> --click 6,11
python3 tools/workbench/stage.py add     --session <session> --selection sel-0001 \
    --verb move_landmark --parameters '{"landmark_id":"fixture_ruin_marker","to":{"x":6,"y":11}}'
python3 tools/workbench/stage.py mask    --session <session> --rect 10,10,6,4
python3 tools/workbench/stage.py list    --session <session>
python3 tools/workbench/stage.py status  --session <session>   # bindings, staged set, every Apply
python3 tools/workbench/stage.py preview --session <session>
python3 tools/workbench/apply.py <session>
```

Exit codes: `stage.py` — 0 done, 2 refused, 3 unreadable. `apply.py` — 0 applied,
2 rejected, 3 unreadable.

`status` is where an agent handed a session it did not open starts: what it is
bound to, what is staged, and what every Apply said, with the receipt or the
rejection named by path so the next question is a file read. It lives in
`stage.py` rather than in `resolve.py` because `resolve.py` answers one question
— what does this packet point at — and a consumer that also summarised sessions
would be two tools sharing a name.

## The interface

Three views, each labelled, none of them a renderer of the game:

- **logical** — the compiler's projection of the accepted land (V0);
- **capture** — a real client frame with its identity sidecar (V0);
- **candidate** — the compiler's projection of the candidate, drawn by the same
  code as the logical view, with **every differing cell outlined and counted**.

The staging panel builds its verb picker from the compiler's own published
vocabulary, prefills a `cells` parameter from the selection the operation derives
from, and shows each verb's rejection line before the operation is staged. The
log panel lists the standing set with a retract button per row. Apply shows the
receipt or the rejection **verbatim**.

**Recording a packet is disabled in the candidate view**, and the reason is on
screen: a packet binds the exact bytes it was taken against, and a candidate's
bytes are replaced by the next preview, so a packet bound to them would be stale
by design. Gestures over the candidate still resolve — in the candidate's own
frame, with the digests they stand on shown — because seeing what now occupies a
cell is the whole reason to point at a candidate.

## The cost of the V1 loop, measured

On the development machine, 2026-08-20, with the workspace already built.

| Operation | Cost |
| --- | --- |
| Stage one operation (a file append) | ~1 ms |
| Retract one operation | ~1 ms |
| Preview: replay + validate + project | **0.43 s** |
| Apply: re-verify, replay, validate, write, receipt | **0.33 s** |
| The whole V1 Python proof (21 tests, each running the compiler) | 3.8 s |
| The promotion ceremony, mechanically | 5.2 s |

Staging costs nothing because it is a file append. A verdict costs a compiler
invocation, which is a rebuild check when the workspace is current and a real
build when it is not — the bridge allows fifteen minutes for that case and says
so if it runs out.

**A consequence worth stating plainly:** the Workbench's own proof now calls the
authoring compiler, so a change under `tools/workbench/` or `tests/` selects a
lane that invokes `cargo`. That is not the workspace lane — no `fmt`, no
`clippy`, no workspace test — but it is not free either, and `AGENTS.md`'s
description of the fast lane says so rather than continuing to claim otherwise.
Changes under `crates/tme-authoring/`, `content/authoring-fixture/`, and
`content/lands/` now select the Workbench proof as well, because the compiler
owns the vocabulary the Workbench bridges into and an authored land is what a
candidate is an edit OF.

## Section 8.1, proven

Every clause, with what holds it.

| §8.1 clause | Proof |
| --- | --- |
| `promotion::load` remains the sole path to a compiled `Land` for the projection | `Land` is constructed only there; `build_land` takes one |
| the reviewed digest remains the human-reviewed anchor | `test_the_promotion_anchors_are_byte_identical_after_a_full_loop` runs a complete loop and compares both anchors, both members, and every generated projection |
| the receipt's authority block stays bounded | `tests/promotion_gate.rs` kills its mutants against **every** land, and the authority must match the contract exactly in both directions |
| no candidate, log, operation or receipt becomes an input to the build | `test_the_promoted_path_still_passes_its_own_check_afterwards` runs `--check` after an Apply; `tests/test_working_root.py::NoTrackedLoaderReadsTheIgnoredRoot` keeps `crates/` and `web/` from naming a session |
| the Workbench never edits `promotion.json` or the digest constant | `Session.artifact` refuses to address anything outside the session, with a mutant; every Apply output is asserted to be inside it |
| `validate_candidate` returns a report, never a `Member` | the `compile_fail` doctest, with the receipt above |

## Honest gaps

1. **The candidate path validates one member.** `validate_candidate` compiles
   the addressed member; the connectivity graph is checked only through
   `promotion::load`. That is sound for the verb set as it stands — the graph
   invariants a single-member edit could touch (a blocked endpoint) are also
   asserted by `compile_member`, and reciprocity, direction and pairing are
   pinned by the exact transition program. A verb that could change
   `target_member` or `paired_transition` would break that argument, and none
   exists. The argument survives a land growing a second member for the same
   reason it held for the fixture's two, and it is the reason the packet
   recommends against adding such a verb.
2. **A commit mask is stated, not drawn.** There is no asset view, so the mask
   comes from `stage.py mask` as a rectangle. The selection packet an asset
   operation derives from records where the owner was pointing in the *land*
   when they asked for it, which is a weaker link than pointing at the pixels
   themselves. Binding pixels to a gesture is the asset-view slice.
3. **The visual edit's exemplar is lane-authored.** `fixture-swatch.png` is a
   synthetic editable master, not an accepted visual master, and its provenance
   record says so. State 3 for a real asset waits for an owner.
4. **A validator rejection is attributed by prefix, not by cause.** See Apply,
   above. It is honest about what it is claiming; it is still not "which
   operation broke this".
5. **A preview costs a process.** The verdict is the compiler's and there is one
   compiler, so a preview is a subprocess and not a function call. It is 0.4 s
   and off the selection path, but it is not the millisecond loop the pointing
   half has.
6. **`compare_candidates` has no implementation at all**, and presenting
   candidates side by side is exactly the thing a taste round wants. It is a
   declared contract and nothing more in V1.
7. **The empty-log identity depends on the master already being canonical.**
   Replay round-trips the document through the compiler's single serializer, and
   today that reproduces the tracked master byte for byte — which is what makes a
   candidate's diff exactly the edit and nothing else. A member re-authored in
   Tiled would come back with Tiled's own key order and whitespace, and the first
   candidate after that would differ everywhere for reasons that have nothing to
   do with the edit. The property is asserted
   (`test_an_empty_log_reproduces_the_accepted_master_byte_for_byte`), so it
   would be caught rather than discovered — and the repair is to canonicalize the
   member as part of accepting it, not to loosen the test.

## Related

- [Workbench V0](workbench-v0.md) — pointing: packets, captures, staleness
- [The authoring compiler](authoring-compiler.md) — the six properties, the
  double anchor, the mutant corpus
- [Contract-first authoring](authoring-contracts.md) — P9, which every mutant
  receipt above is written under
- [Working-root policy](working-root-policy.md) — what a disposable file may
  influence, and the retention ruling
- [Presentation direction](presentation-direction.md) — why the dressing class
  has no target yet
