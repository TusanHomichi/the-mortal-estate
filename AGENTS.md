# Agent guide

**The Mortal Estate** is an original persistent online tactical role-playing
game about life, death, inheritance, and memory. The player is a lineage; death
is another place to play; the world advances on one authoritative pulse.

This file is the Contract and the entry point. It owns routing and the essential
operating rules; detailed workflow and verification usage have linked owners.
`CLAUDE.md` imports this file and adds no rules.

## Operating rules

- **Engineer complete causes.** No workarounds, silent shims, half-migrations,
  or duplicate mutable truth. Name weak architecture and unsafe assumptions
  early; resolve them in the slice or record a durable finding with evidence,
  an owner, and the proof it needs.
- **One fact, one owner.** Read the relevant [boundary](docs/boundary-map.md)
  before changing behaviour. Record consequential decisions in their owner;
  other documents point there.
- **Owner rulings bind.** D2 reopens inherited mechanics, names, values,
  timings, penalties, and routes; surviving code is not design acceptance.
  D5 owns the pulse. Read the [rulings](docs/boundary-map.md#what-an-authored-seam-does-and-does-not-settle)
  before designing either.
- **Atomic internal cutovers.** The external product boundary is not active.
  Follow the [no-compatibility policy](docs/agent-workflow.md#no-compatibility-adapters):
  migrate every caller and proof together; prove retired shapes refused.
- **Keep private payloads out.** No quarantine directory or external reference
  payload belongs in this checkout. Follow the [public-boundary policy](docs/public-boundary-policy.md)
  and the [working-root policy](docs/working-root-policy.md). The private
  denylist is owner-provisioned out of band; its absence must never become a
  false pass ([verification](docs/verification.md#capabilities)).
- **Keep slices bounded and context lean.** Fix or file every finding and
  document lessons. Touching a routinely read file over roughly 1,000 lines
  requires decomposition in the same slice; follow the
  [refactor threshold](docs/agent-workflow.md#the-refactor-threshold).
- **Prove what changed.** Use the runner below; claim only commands and results
  actually observed. Complete the [closeout](docs/agent-workflow.md#closeout)
  before handing off.

**The stack is chosen:** Rust for rules, protocol, simulation, authoring, and one
PostgreSQL-backed server; TypeScript and Three.js for the browser client; Tauri
for its ruled desktop shell; the retained Godot shell; Python for checks and tools. Widening it needs an owner
decision. Keep adjustable facts in validated content and each mutation in its
owning boundary.

## Read first

Start with the [current checkpoint](docs/plans/genesis-ledger.md), the
[settled-conclusions index](docs/settled-conclusions.md), and the routes for the
files you will touch. Read the relevant owner sections before editing; expand
to their proof and references when needed. `always` marks standing guidance
for every task, not a request to load every document in full.

The table and router locate the same owners:

```bash
python3 tools/agent_context.py --path <repository-relative-path>
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
| [docs/agent-workflow.md](docs/agent-workflow.md) | scoping, design, verification lessons, and closeout |
| [docs/verification.md](docs/verification.md) | choosing and running proof |
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

## Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/tme-rules` | gameplay truth: legality, resolution, timing, projection |
| `crates/tme-protocol` | wire-schema authority |
| `crates/tme-server` | sessions, admission, scheduling, durable authority |
| `crates/tme-sim` | deterministic proving through the same rules |
| `crates/tme-authoring` | authored documents to proven runtime content |
| `web/` | browser feel surface; current implementation limits in client notes |
| `client/` | retained Godot shell and authoritative-client proof |
| `content/lands/`, `content/` | authored lands, validated content, test corpus |
| `tools/`, `tests/` | verification, Workbench, proof harnesses and Python tests |
| `deploy/production/` | single-host deployment reference and runbooks |

## Verification baseline

`tools/run_verification.py` owns the step table and lanes. Inspect the selected
plan before running it:

```bash
python3 tools/run_verification.py --list --scope fast --changed-path <path>
python3 tools/run_verification.py --scope fast --changed-path <path>
```

Repeat `--changed-path` for every changed path. Use `--scope full` for complete
proof before merge; live Workbench use requires no verification run. Capability
setup, incomplete verdicts, capture, and CI are in
[verification](docs/verification.md). **UNAVAILABLE is never PASS.**

## What needs an owner decision

Reopened product decisions; pulse cadence; multiple live worlds; external
product-boundary activation or outside players; publication; visual acceptance
and accepted masters; adding an AI runtime; Git lifecycle work; spending; and
effects outside this machine. A cadence change also needs the side-by-side
play-feel test required by D5.

If a request conflicts with an owner ruling, the charter, a boundary, or the
public-boundary policy, state the conflict and risk and get explicit owner
confirmation before proceeding. In-scope implementation follows
[implementer autonomy](docs/agent-workflow.md#implementer-autonomy).
