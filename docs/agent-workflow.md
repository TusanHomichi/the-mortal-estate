---
last_updated: 2026-09-04
revision: 6
status: Working contract from Phase 7; owner acceptance remains at its recorded stop point. Audit clarifies context loading and verification ownership.
public_safe: true
summary: Scope, context loading, document precedence, implementation rules, verification lessons, CI, and closeout.
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
serves: death as play, lineage continuity, the shared pulse, social dependence,
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

The predecessor's workflow contract made the first step of authoring **any**
gameplay spec a research pass over a private corpus of another game's material.
That pass produced the base spec; owner conversation then adjusted it.
Conversation was explicitly not allowed to originate a system.

That rule is **deleted, not adapted.** Two reasons, and the second is the one that
matters:

- The corpus does not exist in this repository and will never be a dependency of
  it. A first-step rule that cannot be followed here is not a rule.
- Anything authored under it is provenance-tainted **by process**, even where the
  output reads perfectly clean. The design started from someone else's expression;
  what the words ended up looking like does not change where they came from.

Deleting a first-step rule and leaving the gap unmarked would have quietly made
"start wherever" the standard, which is worse than the rule that was removed. The
five steps above are the replacement, and they are a rule.

## Implementer autonomy

Within an explicitly authorised slice, the implementer resolves consequential
engineering findings and reports the complete result at the end.

- **The implementer runs to completion.** In-flight findings — fixture and golden
  migrations, version-literal updates, and fixes that follow as a consequence of
  the agreed cut — are resolved by the implementer's judgment and recorded, with
  what was chosen and why.
- **The supervisor adjudicates once, at the end:** the ledger, the full diff, and
  independent verification, before any commit. Disagreement is resolved by fixing
  or discarding the tree, not by re-running the slice.
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

A play session once found a client broken at seams that had passed a complete
green baseline. The production chain from HUD to interaction director to domain
panel had no coverage: the suites built components directly and drove them through
method calls, so a crash that only happened when a control was activated through
its live pressed signal could not be reached by any test.

Constructing a component in a test is not the same as exercising it through the
wiring the product actually uses. **A green baseline that never touches that
wiring is evidence about the test harness, not about the software.**

How this tree honours it: `client/tests/run_all.gd` preloads every suite, so one
unparseable file breaks the harness rather than silently dropping a suite;
`test_support.test_all_client_scripts_parse` additionally loads every script in
the tree; and `tools/run_client_live_proof.py` drives the shipped `ClientRoot.tscn`
against the real server from an empty database rather than a stand-in.

### Assert the real device, session, or backend is in use

The same slice could not judge its audio, because the launcher sandboxed the
runtime directory and the engine fell back to a dummy audio driver **without
complaint**. A silent driver fallback is indistinguishable from a working mix that
happens to be quiet.

Where a proof depends on a real device, display, session, database, or backend,
**assert that the real one is in use.** Never infer it from the absence of an
error.

How this tree honours it: `client/presentation/capture_emitter.gd` asks the
display server first and refuses with a reason rather than writing a blank picture
with a confident sidecar; the PostgreSQL-gated tests require a real fresh database
and are `#[ignore]`d rather than silently passing without one; and every boundary
check exits **3 (FAIL CLOSED)** when its configuration is missing, which is never a
skip and never a pass.

### The single source of truth may not name something that is not there

The predecessor's verification runner listed a Python module that existed
nowhere in its repository. One scope failed on every run, and nothing noticed
until somebody ran that scope. The defect is not the missing module — it is that
**the document which decides what "verified" means was allowed to be wrong about
its own contents.**

So the fix is not "remove the bad line". `tools/verification/targets.py` reads
the step table and asserts that every script, module, binary, client resource,
capability, and owner scope it names exists; it is itself a step
(`meta.step_targets`), it runs in every lane including the fast one, and its P9
mutant is a planted step naming a module that does not exist. The same rule
applies to the Python test inventory: an unclassified `tests/test_*.py` module
makes **every** scope refuse to resolve, so a test nobody runs cannot exist
quietly.

### Prove it in the environment the product configures

Cargo's `[env]` table applies to processes **cargo** launches. A scheduler that
runs the compiled test binaries itself is faster and wrong: the workspace's
`TME_BANNED_TERMS_FILE` never arrives, and the tests run against whichever
denylist the machine happens to have — a red that has nothing to do with the
code, or on another machine a green that means nothing. This was found by the
Phase 8 runner's own first complete run.

The rule generalises past cargo: **a runner may not quietly become a second
place where the product's environment is decided.** `tools/verification/rust_tests.py`
therefore asks cargo to launch every target, and
`tme-rules`'s `cargos_env_table_reaches_this_test_process` is a tripwire that
fails loudly if any future runner stops doing that.

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
the engine's class cache (rebuilt by the client lane or the
[import command](verification.md#capabilities)), and the private denylist
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

Phase 10's first two slices ran as parallel implementer lanes on one machine,
and each of these cost a re-run or a send-back before it was written down.

- **The predecessor's private root must not exist in this checkout.**
  `tools/check_clean_room.py` requires it absent, so a brief that sends capture
  output to that directory name (a predecessor habit) fails the boundary lane.
  Capture output goes to `TME_CAPTURE_OUTPUT` outside the tree.
- **One `--scope full` per host at a time.** The gated PostgreSQL suite carries
  fixed socket timeouts and fails under a parallel lane's build load with an
  `EAGAIN` signature that is green on a quiet box (successor #15). A lane waits
  for the load to fall before its full run, and the supervisor's own full run on
  the final commit is the one that counts.
- **Freeze the carried tree during integrity proof.** Finish documentation and
  source edits before running a suite that compares the tree before and after
  an operation. The Workbench Apply test hashes documents too; an unrelated
  edit during that interval is a real tree change and makes the proof unusable.
  The September 4 documentation audit reproduced this with a workflow edit.
- **The full scope can outlive a tool's foreground limit.** Follow
  [process lifetime](verification.md#process-lifetime); retain its exit code and
  log. A run killed from outside leaves no final step summary.
- **A brief describes the invariant, not the guard.** "Reject a frame whose
  `ready_at` precedes its current time" would have broken idle play: in the
  rules, `ready_at <= now` *is* the ready state. Say "fail closed on an
  inconsistent frame" and let the implementer derive the guards from the rules
  crate.
- **Open the capture before accepting client work.** A width-floor test was
  green while the readiness line rendered as "◆ Ready · beat …". Tests that
  assert a floor are not proof of legibility; the supervisor looks at the PNG.
- **Lanes list, the supervisor files.** An implementer lane has no Git or GitHub
  lifecycle; it reports fix-or-file items with evidence and the supervisor files
  them in the same turn.
- **A lane that goes idle without a report is asked, not assumed.** Messages
  cross; a clean tree at the old commit means the instruction never landed.

## GitHub, CI, and issues

Resolve the current repository from `git remote get-url origin` and confirm it
with `gh repo view --json nameWithOwner` before GitHub lifecycle work.

`.github/workflows/verify.yml` runs on every pull request and on pushes to `main`.
It has **two jobs**, and between them they run the complete lane: one runs
everything this checkout can prove, the other runs the clean-clone proof. Why
two rather than one is [The disk budget](#the-disk-budget) below.

```
python3 tools/run_verification.py --scope portable --scope web --scope client --scope gated --allow-unavailable --report-disk
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
- **`--allow-unavailable` is a declaration, not a skip.** CI has no pinned client
  binary, no PostgreSQL superuser, and no private denylist, so the `client` and
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

### The disk budget

On 2026-08-20, both attempts of run 32438837232 died twelve minutes in with
`System.IO.IOException: No space left on device` — thrown by the runner itself,
so **no step log survived at all**. The lane had just gained the clean-clone
proof, which builds this workspace a second time, and nothing anywhere in the
run had ever said what a build cost or how much room it had.

Two defects, and both are fixed rather than worked around: the builds were
larger than they needed to be, and the run could not say so.

**Measured**, in a fresh `CARGO_TARGET_DIR` on rustc 1.96.0, running
`cargo build --workspace --locked`, then `cargo clippy --workspace --locked
--all-targets -- -D warnings`, then `python3 tools/run_rust_tests.py`, with
`du -sm` after each:

| after | default profile | disposable-build profile | no debuginfo at all |
| --- | --- | --- | --- |
| `cargo build` | 3,679 MiB | 1,334 MiB | 951 MiB |
| `cargo clippy --all-targets` | 4,607 MiB | 1,653 MiB | 1,075 MiB |
| the test suite | **21,245 MiB** | **5,910 MiB** | **3,467 MiB** |
| wall clock, all three | 893 s | 689 s | 628 s |

Of the default figure, 9,426 MiB is `debug/incremental` — state whose entire
purpose is making a build nobody will run faster — and most of the rest is
DWARF the test binaries carry. The disposable-build profile
(`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`,
`CARGO_PROFILE_TEST_DEBUG=line-tables-only`) drops the first entirely and keeps
exactly enough of the second to resolve a backtrace to a file and a line. It is
also 23% faster, because writing 15 GB is not free.

The third column is the option that was measured and **not** taken.
`CARGO_PROFILE_*_DEBUG=0` saves a further 2,443 MiB and would let the complete
lane fit one runner — at the price of test failures whose backtraces name no
line. A proof lane exists to say what broke; buying a runner by blinding it is
the wrong trade when a second runner is free.

**It is set in the environment, not in `Cargo.toml`.** The tracked `[profile]`
tables are the *developer's* build, and quietly degrading everybody's debugger
to make a runner fit is paying for CI out of everyone's tooling. The
environment is where a caller declares "this build is disposable", and the two
callers that declare it are the workflow's `env:` block and
`tools/run_clean_clone_proof.py`. A local `--scope full` still builds the way
this tree is configured to build.

**The peak is bigger than the leftovers, and the peak is what has to fit.** The
table above is `du` at rest between steps. `python3 tools/run_clean_clone_proof.py`
samples while it runs, and on 2026-08-20 two runs reported
`TME_CLEAN_CLONE_PEAK_MiB=6831` and `=6792` for the same work that leaves
5,910 MiB behind — about 15% more, because cargo holds superseded artifacts
alongside their replacements before dropping them. A budget computed from the
resting figure is a budget that is 900 MiB wrong in the direction that kills a
job.

**The arithmetic, and why two jobs.** A GitHub-hosted `ubuntu-latest` runner
offers on the order of 14 GB free before the job reclaims anything. One lean
cold build peaks at 6,831 MiB, so a single job running `full` needs two of them
— 13.3 GiB — plus the toolchain, the cargo registry, and the checkout. That is
not a tight fit; it is the same failure again with a smaller number in front of
it. One runner per cold build leaves roughly 7 GiB spare in each, and the two
finish in parallel rather than in series. So CI runs `full` as two jobs, and
the test described above holds them to covering it exactly.

Elapsed, measured on the same day: the clean-clone proof takes 12m 04s
end to end, of which the inner `portable` lane is 12m 01s.

Each job's own log carries the numbers this argument rests on: `df -h /` before
and after the reclamation, the target directory's size after every step that
built, and the clean-clone proof's sampled peak. If the workspace grows past
what this fits, the log says so long before a runner dies of it.

`.github/disk-budget.sh` runs first in both jobs: it prints `df -h /`, removes
the preinstalled payloads this repository has no use for (a .NET SDK, an
Android SDK, a Haskell toolchain, a CodeQL bundle), and prints `df -h /` again.
That is margin, not the fix — the lane is measured to fit without it — and it
is why the reclamation never fails the job.

**And the run says what it is spending.** `--report-disk` prints the target
directory's size and its filesystem's free space after every step that builds,
and the clean-clone proof samples its own build directory while it runs and
prints `TME_CLEAN_CLONE_PEAK_MiB` whether it passed or failed. A run that dies
of a full disk now leaves a log that says how it got there — which is the part
of 2026-08-20 that made the failure expensive.

**GitHub Issues are the working index** for defects and deferred findings. An
issue is where a fix-or-file finding lands. Issue order does not reorder product
priority, and an open issue is not a plan.

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
9. Hand the complete packet to the supervisor for all Git, pull request,
   required-check, and merge-state work.

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
