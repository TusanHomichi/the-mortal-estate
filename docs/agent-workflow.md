---
last_updated: 2026-09-05
revision: 9
status: Standing workflow; selective context loading and single-agent/delegated closeout clarified. Historical disk measurements moved to a linked receipt.
public_safe: true
summary: Scope, selective context loading, ownership, implementation, verification method, CI, and closeout.
always: true
---

# Agent workflow

This repository should be easy for a coding agent to inspect, resume, and change
without reopening settled decisions or widening a slice by accident.

The contract is deliberately light. It exists to answer four questions: what owns
this fact, where does this work start, what proves it, and what does finishing
mean.

## Context loading

Read `AGENTS.md` once, then use `tools/agent_context.py --path <path>` for the
files in scope. The router prints navigation metadata, not document bodies.
Read each selected owner's relevant sections and expand into code, tests, and
linked evidence as the task requires. Standing (`always`) documents remain
applicable; that does not require copying their whole bodies into every brief.

For resumption, read the current checkpoint in the
[genesis ledger](plans/genesis-ledger.md), then the specific active plan only
if this task touches it. A historical plan, a local memory, or an agent-specific
configuration does not override a maintained owner or authorize a paused slice.

Keep agent-specific entry files as pointers to this shared contract. Machine
paths, credentials, installed MCP servers, and disposable lab state belong in
local configuration, not repository-wide instructions. A handoff names the
objective, owners, evidence, and next step; it does not repeat the manual.

## Slice workflow

For anything non-trivial:

1. Clarify intent and scope, and write the scope down.
2. Read the owners the work touches — start from [AGENTS.md](../AGENTS.md).
3. Write or update the owning document when behaviour, architecture, a content
   model, or the workflow changes.
4. Implement only the agreed slice.
5. Verify with the commands that match what changed.
6. Close out (below), and leave a clean handoff.

Tiny typo fixes and mechanical corrections may skip the ceremony. They may not
skip verification.

Prefer durable repository truth over chat-only decisions. If a decision will
matter later, it goes in the document that owns the fact — not in a commit
message, and not in a conversation.

## Document families and precedence

Every document belongs to exactly one family:

- **Contract** — [AGENTS.md](../AGENTS.md), this file, and
  [verification usage](verification.md). They own the entry rules, detailed
  workflow, and how to run the baseline respectively. `CLAUDE.md` imports the
  entry point and adds no rule. The runner owns the executable step table.
- **Canonical** — maintained fact owners listed in
  [the routing table](../AGENTS.md#read-first). Each fact has one asserting
  owner; the router locates it. The settled-conclusions index links to those
  owners and does not restate their detailed rulings.

- **Planning** — a spec bounds the intended target; its plan owns execution order
  and proof. Neither is implemented truth before closeout, and a plan cannot
  widen its spec.
- **History** — records of what happened: drill records, provenance records,
  evaluations. History is time-bounded evidence and never present authority. A
  history document must say so in its own status line.

Resolve conflicting claims in this order:

1. an explicit owner ruling;
2. the charter and later owner-approved product direction;
3. the Contract family — `AGENTS.md`, then this workflow and verification usage
   in their respective scopes;
4. the sole Canonical owner of the fact;
5. an approved spec and its plan, for bounded in-flight work;
6. History, as evidence of what happened — never an override.

### The drift rule

If two same-level owners assert the same present fact, that is **drift**. Fix the
ownership conflict; do not pick whichever prose was written more recently, and do
not "reconcile" them into two agreeing copies. Two agreeing copies are a future
disagreement with a longer fuse.

A document that needs a fact it does not own links to the owner. It does not
restate the value.

## Where authoring a gameplay spec starts

This is the rule that answers "where does this begin?" for any spec that touches a
gameplay system. It is followed in order, and step 5 is not reachable by skipping
step 2.

**1. The charter.** Name which of the product's identity truths this system
serves: death as play, lineage continuity, authoritative individual timing, social dependence,
dangerous geography, or the world's memory. A system that strengthens none of
them is a scope question for the owner, not a spec.

The checkout's product identity is in [README.md](../README.md); carried
charter rulings are located through the [boundary map](boundary-map.md) and
[genesis ledger](plans/genesis-ledger.md). There is no standalone charter file
in this tree. If a needed ruling is not carried, ask the owner to supply that
decision; a private archive is not an implicit prerequisite for starting work.

**2. The reopened list.** Check [boundary-map.md](boundary-map.md) Part 2 and the
charter's open-decisions list. If this system's values sit on either, the spec is
**authoring them fresh**. It may not import a value — not from the predecessor,
not from another game, not from an agent's recollection, and not from a number
that happens to already be in this tree because a port carried it. Ruling D2
reopened every exact mechanic, name, timing, penalty, and route; a spec that
quietly reuses one has broken a ruling, not saved time.

**3. The owner of every fact it changes.** Before designing a mechanic, name the
seam that will own each fact the system mutates, from
[boundary-map.md](boundary-map.md). If a fact has no owner, **naming that owner is
the first deliverable of the slice**, not a detail to be settled during
implementation. A mechanic designed before its ownership is a mechanic that will
be implemented in three places.

**4. What this repository already proves.** Existing typed contracts, tests,
fixtures, and goldens are the strongest available evidence of intended behaviour —
stronger than any prose, including prose in this repository. Read them before
writing. A spec that contradicts a green test is proposing a deliberate break, and
must say so in those words, name the tests, and migrate them in the same slice.

**5. Original design work, with the owner.** Then design. Owner conversation
originates the mechanic; it is not an adjustment pass over something inherited.
Where nothing settles a question, present the options and their consequences and
let the owner rule — then record the ruling in the document that owns the fact.

**Labelling is part of the rule.** A spec marks every claim as decided, proposed,
or open. An unlabelled claim reads as settled by the next agent, and that is how a
guess becomes a fact. Low-confidence areas belong on the open list where the owner
can see them, not flattened into confident prose.

**Evidence.** What may be cited as the basis of a design claim: owner rulings, the
charter, this repository's own contracts and tests, original design work, and
observed play. External reference material is never the basis of a spec. It may
inform a human-authored conclusion, and it never appears as authority in a
document — the full rule, including quarantine and provenance, is
[public-boundary-policy.md](public-boundary-policy.md).

### What this replaced, and why

The retired workflow required private-corpus research before any gameplay spec.
The authoring steps above replace that dependency: owner conversation can
originate design; external reference payload cannot enter the tree. Follow the
[public boundary](public-boundary-policy.md#the-quarantine-flow).

## Implementer autonomy

Within an explicitly authorised slice, the implementer resolves consequential
engineering findings and reports the complete result at the end.

- **The implementer runs to completion.** In-flight findings — fixture and golden
  migrations, version-literal updates, and fixes that follow as a consequence of
  the agreed cut — are resolved by the implementer's judgment and recorded, with
  what was chosen and why.
- **For delegated work, the supervising agent reviews the complete result:**
  findings, full diff, and independent verification before any commit. A single
  agent performs its own review; this rule does not require delegation.
- **Hard stops are ownership questions, not engineering ones:** the owner's play
  and gate verdicts, Git operations, boundary crossings, external-boundary
  activation, spending, and product decisions.
- Plans say "resolve and record" for in-scope engineering findings, not "stop and
  report".

Pair this with blast-radius enumeration in the plan — named search commands for
every surface that encodes a changed behaviour, including tests, goldens, and
version pins — so most findings never arise mid-run.

**Fix or file, immediately.** Anything found mid-work is either fixed in that
slice or filed as a durable issue in that slice, with exact evidence, an owner,
and the proof it will need. Nothing is silently deferred, and "I noticed it" is
not a record.

## The refactor threshold

The test is readership: **does an agent routinely read this file to orient?** The
rule applies to `AGENTS.md`, the agent-facing documents in `docs/`, and code.
Planning and history documents are exempt — they are opened for a bounded purpose,
not for routine orientation. The distinction is readership, not durability.

For an in-scope owner, size is a review signal with tiers, not an automatic
failure:

1. Adding behaviour or guidance to a file already over **1,000 lines** means
   performing an ownership-based decomposition in the same slice.
2. Changing behaviour or guidance above **1,500 lines** means naming the ownership
   boundary preserved or improved before editing. A routinely read owner at
   **1,600 lines or more** is decomposed in the same round when changed.
3. Before major work above **2,500 lines**, record a reviewed decomposition path
   unless the task is an urgent fix.

Split by responsibility, not by line count. Migrate tests, fixtures, and goldens
in the same cut, under the no-half-migration rule.

Existing oversized owners are debt, not exemptions. Check the size of the
actual file being touched and its recorded decomposition work; a dated count
of oversized files is not the current inventory.

## No compatibility adapters

This project is **pre-external-boundary**. No externally distributed client, real
persistent player data, released save or content format, public API consumer, or
deployed service interface exists.

While that is true, architectural coherence and one clean current contract take
priority over compatibility with obsolete internal shapes. The default is **no
compatibility adapters**: no aliases, dual parsers, fallback selectors, behavioural
legacy fallbacks, deprecated fields, translation layers, or shims kept alive
solely to keep an old internal shape working.

When a slice identifies a better design:

1. name the intentional break in the spec or plan;
2. change the authoritative contract directly;
3. migrate every owned caller, fixture, test, tool, and golden in the same slice;
4. delete the superseded path rather than retaining a branch;
5. finish with the affected focused checks and the full baseline green.

Tests remain required. They are evidence for intended current behaviour; they are
not a reason to preserve an inferior shape. Replace an obsolete expectation with a
test for the agreed design, and keep a historical assertion only where the current
design still values that behaviour. Where a retired shape must stay *refused*,
prove the refusal — the wire fixture corpus carries retired envelope shapes as
explicit reject cases for exactly this reason.

Migration during this phase is **one atomic cutover followed by fast validation**,
never dual schemas or a staged internal compatibility period.

Compatibility becomes a real requirement only when an explicit project decision
records a real external boundary. Do not infer activation from the existence of a
version number. The policy that applies after activation is owned by
[server-notes.md](server-notes.md#the-external-boundary-when-it-activates).

## Verification

`tools/run_verification.py` owns the step table and the lanes;
[verification usage](verification.md) owns how to drive it. The lessons below are
owned here because they concern method rather than commands. Their evidence
remains with the relevant proof.

### Prove the real path, not a reconstruction of it

Exercise shipped wiring as well as isolated components. Browser proof drives real canvas input and inspects scene output.
`tools/run_server_live_proof.py` drives the real wire on a scratch database;
production browser UI/transport integration still needs its own proof. A component test
alone cannot establish that a player's click reaches it.

### Assert the real device, session, or backend is in use

Probe required devices, displays, databases, and renderers. A successful launch
can silently use a dummy backend. Browser proof logs the actual WebGL renderer; gated tests require fresh
PostgreSQL databases. Missing capabilities report UNAVAILABLE, never PASS.

### The single source of truth may not name something that is not there

`tools/verification/targets.py` checks every script, module, client resource,
capability, and scope named by the step table. Its missing-module mutant proves
refusal. Every Python test module must be classified in the table or all scope
resolution fails. Update callers and the inventory together when moving proof.

### Prove it in the environment the product configures

Use the product's configured environment. In particular, cargo must launch Rust
test binaries so `.cargo/config.toml` reaches them; a direct binary scheduler
silently loses those settings. `tools/verification/rust_tests.py` preserves that
path, and `cargos_env_table_reaches_this_test_process` proves it. Use the runner's
resolved environment for Python groups too.

### A check earns its blocking status

A fail-closed check runs **advisory** until it has killed a deliberate mutant, and
the kill is recorded with the exact test that performs it. An unexercised
fail-closed path is an assumption, not a guarantee. This is principle P9 of
[authoring-contracts.md](authoring-contracts.md); the qualification table lives in
[boundary-checks.md](boundary-checks.md).

### Working in a linked worktree

Check `pwd`, `git status --short`, and `git rev-parse --show-toplevel` before
editing. A session may use the main checkout or a linked worktree; no agent
provider or directory name establishes which one. A fresh worktree lacks three
things the lanes need: web dependencies (`npm ci`, run by the web lane),
Rust build outputs (rebuilt by the Rust lane), and the private denylist
(resolved from the main checkout when absent locally). Build and deploy only
from your own worktree, never from a checkout another agent is editing;
work there can mix with someone else's unfinished changes. Creating a
worktree and all other Git lifecycle work remain owner-authorized actions.

### Helpers that cannot open a socket or a browser

A sandboxed helper agent that writes code well may still be unable to listen
on loopback or launch a browser. Split such work: the helper **writes** the
harness, proof, or capture script and proves what it can (type checks, unit
tests, pure-Python treatment), and the session that can open a browser
**runs** it. Say which half is whose in the brief. Two process rules from the
same day: a served proof spawns its server detached and stops the whole
process group, because a dev server's children outlive a plain kill of the
wrapper; and never signal processes by a pattern that also appears in your
own command line — it matches your own shell.

### Running lanes on one host — observed, 2026-08-21

- Write captures outside the checkout through `TME_CAPTURE_OUTPUT`.
- Run one full verification per host. Gated PostgreSQL tests have fixed socket
  timeouts and can fail under competing build load (issue #15).
- Freeze source and documentation during integrity proof. Workbench Apply hashes
  the carried tree; an unrelated concurrent edit invalidates that proof.
- Do not run `npm ci` concurrently with a browser proof using that dependency tree.
- Preserve logs and exit status for long runs; use the
  [process-lifetime guidance](verification.md#process-lifetime).
- Describe invariants rather than guessed guards: `ready_at <= now` is valid for
  an idle character. Derive legality from the owning rules.
- Open captures before claiming visual verification. Size assertions alone do
  not prove legibility.
- With delegated work, collect explicit reports and resolve every fix-or-file
  item. Silence or an unchanged tree is not a completion report.

## GitHub, CI, and issues

Resolve the current repository from `git remote get-url origin` and confirm it
with `gh repo view --json nameWithOwner` before GitHub lifecycle work.

`.github/workflows/verify.yml` runs on every pull request and on pushes to `main`.
It has **two jobs**, and between them they run the complete lane: one runs
everything this checkout can prove, the other runs the clean-clone proof. Why
two rather than one is [The disk budget](#the-disk-budget) below.

```
python3 tools/run_verification.py --scope portable --scope web --scope gated --allow-unavailable --report-disk
python3 tools/run_verification.py --scope cleanclone --allow-unavailable --report-disk
```

**CI lists no steps of its own.** It names lanes; the runner resolves them. That
is what makes "CI passes" and "it passes locally" the same claim rather than two
claims that happen to agree today. To see exactly what CI will run, run
`python3 tools/run_verification.py --list --scope full`.

**And the two jobs are asserted to be `full`, exactly.** Splitting a lane across
jobs opens a way to drift that neither job would look wrong for: a dropped
`--scope cleanclone` is a merge gate that silently stopped running.
`tests/test_ci_workflow.py` reads the workflow, resolves the scopes each job
names against the step table, and asserts the union equals `full` and that no
step is paid for twice. It also asserts the workflow's `env:` block still
matches `tools/verification/footprint.py`, which is where the build profile is
actually decided.

Five things about it are deliberate and should not be "fixed" without
understanding them:

- **The pins are verified, not assumed.** The workflow records the exact run that
  established each one.
- **`--allow-unavailable` is a declaration, not a skip.** CI has no PostgreSQL superuser or private denylist, so the
  `gated` steps report UNAVAILABLE with their reasons and the run prints an
  INCOMPLETE verdict before exiting 0. A step that *fails* still turns CI red, and
  a boundary check with a missing input still exits 3 from inside itself.
- **The banned-terms check degrades rather than skipping.** Absent the private
  denylist it runs its real matching, scanning, and fail-closed machinery against
  the tracked synthetic fixture, and the verdict says so. It asserts nothing about
  the real list — that is a local owner run, on a machine that has `.boundary/`.
  A deleted or emptied fixture exits 3 and turns CI red.
- **No `TME_*` variable is set in CI.** `.cargo/config.toml` supplies the synthetic
  terms file with `force = false`, so an environment value wins where one is set;
  the PostgreSQL-gated tests are `#[ignore]`d and skip cleanly under a plain
  `cargo test` rather than passing vacuously — and the `gated` lane, which runs
  them properly against fresh databases, is honest about being unavailable here.
- **Every job states its disk budget before it spends anything.** See below.

`cargo fmt --all -- --check` and `cargo clippy --workspace --locked --all-targets
-- -D warnings` are both steps in the `rust` lane and therefore both run in CI.
They were absent until the whole-tree reformat landed on its own commit
(private-archive issue #5), which is the order that issue asked for.

Branch protection is an owner action and is not configured from the repository.
The required setup names **both** jobs; a rule naming only `verify` would let
the clean-clone proof fail without blocking a merge. The tracked workflow and
its tests do not prove the current remote branch-protection settings.

**GitHub Issues are the working index** for defects and deferred findings. Issue
order does not reorder product priority, and an open issue is not a plan.

### The disk budget

`tools/verification/footprint.py` owns disposable-build settings, used by CI and
the clean-copy proof. Disable incremental artifacts and retain line-table debug
information there; do not change developer Cargo profiles to fit a CI runner.
CI separates the normal and clean-copy builds onto two jobs and reclaims unused
preinstalled payloads with `.github/disk-budget.sh`.

Budget for peak usage, not just artifacts left afterward. `--report-disk` logs
size/free space after build steps; the clean-copy proof samples its peak.
The [August 20 measurements](plans/2026-08-20-verification-footprint.md) explain
the split and retained debugging information. They are historical evidence,
not today's build size or runtime guarantee.

## Closeout

Complete these in order. An item may be `N/A` only with a stated reason.

1. Confirm the agreed scope, the finding ledger, and the exact ancestry the work
   was done against.
2. Classify every new or changed document into one family and name its sole fact
   owner before adding prose.
3. Refresh `last_updated`, `revision`, `status`, and `summary` on every changed
   Canonical document.
4. Run the fact-class search: does any other document now assert a fact this
   change made this document's? If so, that is drift — fix the ownership, not the
   wording.
5. Audit links: every relative link in every changed document resolves to a file
   this repository carries.
6. Run every verification surface the change touches, and record the observed
   result — not the expected one.
7. Record the decisions that will matter later in the document that owns them.
8. File or fix every finding that is not in scope. Nothing carries forward
   unrecorded.
9. Report the complete result. The responsible agent handles Git, pull requests,
   required checks, and merge state when the owner has authorized that lifecycle.
   With delegated work, that responsibility stays with the supervising agent.

## Handoffs

A handoff should let the next agent resume without reopening settled decisions or
widening the work. Include: current branch; `HEAD`; clean or dirty; the documents
to read first; the exact next objective; explicit non-goals; the verification
commands already run and their observed results; and known blockers.

Keep it concrete. "Continue the architecture work" is not a handoff.

## Boundary conflicts

If a request conflicts with an owner ruling, the charter, an architecture
boundary in [boundary-map.md](boundary-map.md), or
[public-boundary-policy.md](public-boundary-policy.md), say so plainly, explain
the risk, and get explicit confirmation before proceeding. Flagging a conflict is
not obstruction; discovering it after the merge is.

## What not to add yet

- A public client, outside-player admission, or external-boundary activation
  without the decision that owns it.
- Multi-host orchestration, a second database, a broker, a distributed lock, or an
  additional implementation-language family.
- Generated-documentation tooling, freshness scoring, or heuristic prose
  deduplication.
- CI beyond the workflow above, scheduled automation, or repository hooks.
- An AI runtime of any kind ([boundary-map.md](boundary-map.md#part-3-ai-is-never-game-authority)).
