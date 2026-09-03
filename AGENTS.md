# Agent guide

The first stop for anyone — human or agent — working in this repository.

**The Mortal Estate** is an original persistent online tactical role-playing game
about life, death, inheritance, and memory. The player is a lineage, not a
character; death is another place to play rather than a loading screen; and the
world advances on one authoritative pulse.

This file is the Contract. It owns routing, the operating rules, and the
verification baseline. It does not restate what another owner already holds — it
points.

## The engineering standard

Engineer complete causes. No workarounds, no temporary fixes, no silent shims, no
half-migrations. When the correct solution is a rewrite, propose or perform the
bounded rewrite rather than hiding the need.

- **Fix or file, immediately.** Anything found mid-work is fixed in that slice or
  filed as a durable issue in that slice, with exact evidence, an owner, and the
  proof it needs. Nothing is silently deferred.
- **State weak architecture, unsafe assumptions, and better approaches plainly and
  early.** Flagging a problem before the merge is the job; discovering it after is
  the failure.
- **Record consequential decisions in the document that owns the fact** — not in a
  commit message, not in a conversation.
- **Claim verification only for commands actually run and results actually
  observed.** An unexplained failure is evidence, not noise.
- **Touching a routinely read file over ~1,000 lines means decomposing it in the
  same slice.** The tiers are in
  [agent workflow](docs/agent-workflow.md#the-refactor-threshold).
- **Maintain the workflow as a product too.** A lesson that lands undocumented is
  a deferred fix.

## Read first

Ask, or read the table — they are the same answer, and a check keeps them that way:

```bash
python3 tools/agent_context.py --path crates/tme-server/src/store/mod.rs
python3 tools/agent_context.py --list
```

Each document below declares in its own front matter which paths it owns.
`tools/agent_context.py --validate` asserts this table names exactly the
documents `docs/` carries, and that no document claims a path the tree does not
have — it runs in the `docs` lane, so the table cannot drift from the tree
without turning a run red.

| Start here | When |
| --- | --- |
| [docs/boundary-map.md](docs/boundary-map.md) | before changing behaviour — who owns the fact you are about to move |
| [docs/agent-workflow.md](docs/agent-workflow.md) | before authoring a spec, and at closeout |
| [docs/settled-conclusions.md](docs/settled-conclusions.md) | before re-deciding something — check whether it is already closed |
| [docs/public-boundary-policy.md](docs/public-boundary-policy.md) | before touching provenance, naming, external material, or anything public-facing |
| [docs/boundary-checks.md](docs/boundary-checks.md) | when a boundary check fires, or when adding one |
| [docs/working-root-policy.md](docs/working-root-policy.md) | before letting anything local, ignored, or disposable influence a build, a test, or content |
| [docs/authoring-contracts.md](docs/authoring-contracts.md) | when authoring geography, artifacts, or conformance proof |
| [docs/authoring-compiler.md](docs/authoring-compiler.md) | when working on `crates/tme-authoring` or authored content |
| [docs/server-notes.md](docs/server-notes.md) | server, persistence, credentials, migrations, the external boundary |
| [docs/client-architecture.md](docs/client-architecture.md) | the client's standing contract |
| [docs/client-notes.md](docs/client-notes.md) | what the client actually does today |
| [docs/presentation-direction.md](docs/presentation-direction.md) | the visual target |
| [docs/workbench-v0.md](docs/workbench-v0.md) | the owner-agent spatial reference tool — pointing |
| [docs/workbench-v1.md](docs/workbench-v1.md) | the same tool's editing half — staged operations, candidates, Apply |
| [docs/test-corpus-provenance.md](docs/test-corpus-provenance.md) | what `content/test-corpus/` is and is not |
| [docs/deploy-drill-2026-08-20.md](docs/deploy-drill-2026-08-20.md) | deployment evidence — history, not present authority |

The layout:

```text
crates/tme-rules      gameplay truth: legality, resolution, timing, projection
crates/tme-protocol   the wire schema — the authority both sides mirror
crates/tme-server     sessions, admission, scheduling, durable authority
crates/tme-sim        deterministic gameplay proving over the same rules
crates/tme-authoring  authored documents -> proven runtime content
web/                  the browser client — the feel surface, browser first
client/               the retained Godot shell, cold; the desktop is the web client in a shell
content/lands/        the authored lands, and the compiled world a server serves
content/              validated authored content and the test corpus
tools/                the verification runner, boundary checks, the Workbench,
                      proof harnesses
tests/                the Python suite for tools, the runner, and the Workbench
deploy/production/    the single-host deployment reference
```

## Operating rules

**Boundaries.** [docs/boundary-map.md](docs/boundary-map.md) names the owner of
every fact class. One fact, one owner. If two documents assert the same present
fact, that is drift — fix the ownership rather than the wording.

**The reopened list binds.** Owner ruling D2 reopened **all** exact mechanics,
names, timings, penalties, and routes for fresh design. A value that survives in
this tree because a port carried it is not authority. The one ruled gameplay value
is the pulse: **3.0 seconds** (D5).

**There is no quarantine directory in this checkout, and there will not be one.**
External reference material does not live here, is not a dependency, and never
enters the tree as payload — in any form, under any renaming. What crosses is a
conclusion a human wrote, in this project's own words. The `clean-room` check
proves the private roots are both absent from disk and uncommittable. The full
rule, including the marker contract, the evidence ladder, and the mandatory clean
break, is [docs/public-boundary-policy.md](docs/public-boundary-policy.md).

**The private denylist must be provisioned.** The banned-terms mechanism is
public and carried; the terms live in the git-ignored `.boundary/banned-terms.txt`
and are never committed. A fresh clone has no `.boundary/` and every run of that
check exits **3 (FAIL CLOSED)** until the owner provides the file out of band.
That is the intended first experience, not a setup bug.

**Pre-external-boundary.** No externally distributed client, real persistent
player data, released save or content format, public API consumer, or deployed
service interface exists. Until an explicit project decision records one:

- the default is **no compatibility adapters** — no aliases, dual parsers,
  fallback selectors, legacy behavioural fallbacks, deprecated fields, or
  translation layers kept alive for an obsolete internal shape;
- a contract change is one atomic cutover with every owned caller, fixture, test,
  and golden migrated in the same slice;
- and a retired shape that must stay refused is proven refused, not merely
  deleted.

After activation, the wire and durable data are protected by the policy in
[docs/server-notes.md](docs/server-notes.md#the-external-boundary-when-it-activates).

**The stack is chosen.** A Rust workspace for rules, wire DTOs, simulation,
authoring, and one PostgreSQL-backed authoritative server; one thin browser
client in TypeScript on Three.js (owner ruling 2026-09-02 — browser first);
the retained Godot shell; Python for repository checks, tools, and proof
harnesses. Do not widen it without a decision that owns the widening.

**Design for adjustment.** One authoritative source per fact; adjustable values in
validated content rather than code where practical; each calculation and mutation
centralised in its owning boundary; no duplicated mutable state and no parallel
implementations left alive after a migration.

**Small slices, durable records, narrow diffs.** And verify with the commands that
match what changed.

**If a request conflicts** with an owner ruling, the charter, a boundary, or the
public boundary policy — say so, explain the risk, and get explicit confirmation
before proceeding.

## Verification baseline

`tools/run_verification.py` **is** the baseline. It owns the step table, the
lanes, and what each one proves. This section explains how to use it and does
not restate the commands it resolves — a hand-copied command list is a second
source of truth, and a second source of truth is a drift nobody has noticed yet.

Ask it what it will do before asking it to do anything:

```bash
python3 tools/run_verification.py --list --scope full
python3 tools/run_verification.py --capabilities   # what this machine can prove
python3 tools/run_verification.py --scope full --report-disk   # and what it costs
```

`--report-disk` prints the cargo target directory's size and its filesystem's
free space after every step that builds. It is what CI runs with, and why: see
[agent workflow](docs/agent-workflow.md#the-disk-budget).

### The four lanes

The charter's four loops, as scopes:

| Loop | Command | What it costs |
| --- | --- | --- |
| Live Workbench iteration | **no verification run at all** — using the Workbench requires none | milliseconds |
| Focused checks on what changed | `--scope fast --changed-path <path> ...` | seconds; **no client or web run unless that client changed, and no workspace lane unless Rust changed** |
| Exact gameplay preview capture | `--scope capture` (owner-invoked, outside the standing baseline) | minutes, and needs a display and a database |
| Complete proof before merge | `--scope full` — what CI runs | the whole workspace |

The fast lane is defined by what it **excludes** and the complete lane by
running everything; neither is allowed to drift toward the other, and a test
asserts the step table's partition. Anything the fast lane does not recognise
escalates to `portable` and says why — a guess is never cheaper than a build.
Escalation is a floor, not a ceiling: the lanes the recognised paths beside it
select (`web`, `client`) still run.

One honest qualification, since Workbench V1: the Workbench reaches the authoring
compiler's semantics through one command, because there is exactly one
implementation of them. So its own proof **invokes `cargo run -p tme-authoring`**
— a rebuild check on a current workspace, and a real build on a cold one. That is
not the workspace lane (no `fmt`, no `clippy`, no workspace test), and it is not
free either. See [Workbench V1](docs/workbench-v1.md#the-cost-of-the-v1-loop-measured).

The owner scopes that compose them, each runnable alone: `docs`, `boundary`,
`python`, `rust`, `workbench`, `web`, `client`, `gated`, `cleanclone`,
`meta`, `capture`.

### What the exit code means

| Code | Meaning |
| --- | --- |
| **0** | every selected step ran and passed |
| **1** | a step failed |
| **2** | usage error |
| **3** | **INCOMPLETE** — nothing failed, but a step could not run, or ran in a reduced form |

**UNAVAILABLE is never PASS.** A step whose capability is absent does not run,
is reported with the reason, and makes the whole run incomplete. `--allow-unavailable`
turns 3 into 0 and is how a caller declares out loud that it cannot supply what
is missing — CI passes it, because CI has no client binary, no database, and no
private denylist. It is a stated limit, not a skip.

### Capabilities

| Capability | Supplied by |
| --- | --- |
| `node` | a `node` on `PATH` whose major version is 22 or later, plus `npm` — asked, never assumed. Absent, the `web` lane is `UNAVAILABLE` |
| `godot` | `TME_GODOT`, naming a binary whose version is exactly `4.7.2.stable.official.ed1daf0bf` — asked, never assumed from the path |
| `postgres` | `TME_PG_ADMIN_URL_FILE`, naming a readable file holding a superuser URL used only to create and drop scratch databases, plus `psql` |
| `private-terms` | `.boundary/banned-terms.txt`. Absent, the banned-terms check **degrades** onto the tracked synthetic fixture: the mechanism still runs and still must pass, and the run says the real denylist was not proven |
| `display` | `DISPLAY`, or `xvfb-run` |
| `capture-output` | `TME_CAPTURE_OUTPUT`, naming a directory |

The engine's class cache is not tracked. A fresh checkout or worktree has none,
and every new `class_name` invalidates it; in both cases the client lane and the
live proof fail to parse scripts until it is rebuilt:

```bash
cd client && "$TME_GODOT" --headless --path . --import
```

### Observed, on 2026-08-20

`--scope full` with every capability supplied: 1363 Rust tests across 32
executables plus 5 doctest runs, 330 Python tests, 147 client tests across 26
suites, five boundary checks, six gated PostgreSQL tests each against its own
fresh migrated database, and a clean copy of the carried set that builds and
tests with every ignored root absent.

### On-demand proofs

Outside the standing baseline because each needs something a checkout does not
carry. `--scope capture` runs the first two.

```bash
# The live proof: real client against real server, from an empty database.
TME_GODOT=<binary> python3 tools/run_client_live_proof.py --admin-url-file <file>
# The capture route: photograph the frame the real server sends. Needs xvfb-run.
TME_GODOT=<binary> python3 tools/run_fixture_land_capture.py \
    --admin-url-file <file> --output <directory>
# The Workbench itself, and a scripted tour of the selection loop.
python3 tools/workbench/serve.py
python3 tools/workbench_demo.py
```

`tools/run_production_smoke.py` exercises a **deployed** host through public
HTTPS/WebSocket. It needs a running deployment and is not part of any lane.

## What needs an owner decision

Not an implementer's call, in any slice:

- any reopened mechanic, value, name, timing, penalty, or route (D2);
- the pulse cadence — it changes by ruling with a side-by-side play-feel test,
  never by editing a constant (D5);
- more than one live world instance (D4);
- activating the external boundary, admitting outside players, or any public
  release claim;
- publishing anything, or crossing the public boundary;
- visual acceptance and the accepted masters;
- adding an AI runtime of any kind;
- Git lifecycle work, spending, and anything with an effect outside this machine.
