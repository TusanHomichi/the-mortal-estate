# Agent guide

**The Mortal Estate** is an original persistent online tactical role-playing
game about life, death, inheritance, and memory. The player is a lineage; death
is another place to play; the server owns individual action deadlines without a shared gameplay pulse.

<!-- house:rules v1 begin. Copied from FieldmouseWorks/burrow house/rules.md, verbatim. Change it there, not here: burrow's house sync rewrites everything between these markers. -->
## House rules

These hold in every repo. This file's own sections add to them and win where
they say something more specific.

- Fix the whole cause. No workarounds, shims, fallbacks or half-migrations:
  when you replace something, delete the old path in the same change.
- One fact, one home. Each rule or fact lives in one place; link to it instead
  of restating it.
- Before you edit, say what done looks like and how it will be checked. Simple
  work gets a short plan, not a ceremony.
- Report only the checks you actually ran, and what they said. Skipped,
  unavailable or "should pass" is never a pass. Keep local and CI results
  apart.
- A new check comes with proof that it fails on the bug it's for.
- Touching a file over about 1,000 lines? Split it in that change.
- The code is the memory. When something goes wrong, make the code, a type, a
  test or a check stop it happening again; a new rule in AGENTS.md is the last
  resort. Say in the PR which one you did.
- Keep going on work you were asked to do. Don't stop to ask again for what's
  already been asked for.
- Never commit what the repo keeps private: secrets, tokens, personal data, and
  anything listed under this file's "Private" section.
- `Closes #N` only when the issue is fully done; otherwise `Refs #N`.
- Tracked files never name a model. Which model does what is the machine's
  business, and model names go stale in weeks.

### Commits

Subjects are `area: plain sentence`, where the area is the module, directory or
tool the change is about: `autoland: merge only the head that was checked`.
Lowercase after the colon, no trailing period, no `feat:`/`fix:` prefixes. The
body, when there is one, says why.
<!-- house:rules end -->

This file is the Contract and the entry point. It owns routing and the essential
operating rules; detailed workflow and verification usage have linked owners.
`CLAUDE.md` imports this file and adds only what Claude alone needs.

## Operating rules

These add to the house rules above; each says where it is stricter.

- **Engineer complete causes.** Stricter than the house rule: no duplicate
  mutable truth either. Name weak architecture and unsafe assumptions early;
  resolve them in the slice or record a durable finding with evidence, an
  owner, and the proof it needs.
- **Read the boundary first.** Read the relevant [boundary](docs/boundary-map.md)
  before changing behaviour, and record consequential decisions in the owner
  it names.
- **Owner rulings bind.** The owner-directed historical gameplay
  baseline supersedes D2's blanket reset; surviving code is evidence, not proof
  of exact correspondence.
  D5 owns authoritative timing. Read the [rulings](docs/boundary-map.md#what-an-authored-seam-does-and-does-not-settle)
  before designing either.
- **Atomic internal cutovers.** Check the [server-owned activation status](docs/server-notes.md#the-external-boundary-when-it-activates).
  Before activation, follow the [no-compatibility policy](docs/agent-workflow.md#no-compatibility-adapters):
  migrate every caller and proof together; prove retired shapes refused.
- **Keep raw references out.** No quarantine directory or raw external reference
  payload belongs in this checkout. Authorized derived geography follows the
  [first-land map-template ruling](docs/public-boundary-policy.md#first-land-map-template).
  Follow the [public-boundary policy](docs/public-boundary-policy.md)
  and the [working-root policy](docs/working-root-policy.md). The private
  denylist is owner-provisioned out of band; its absence must never become a
  false pass ([verification](docs/verification.md#capabilities)).
- **Keep slices bounded and context lean.** Fix or file every finding and
  document lessons. Decomposing a large file follows the
  [refactor threshold](docs/agent-workflow.md#the-refactor-threshold).
- **Prove what changed.** Use the runner below, not only the flake gate.
  Complete the [closeout](docs/agent-workflow.md#closeout) before handing off.
- **Work from evidence.** Use the [standing workflow](docs/agent-workflow.md#slice-workflow)
  and one [canonical graph](docs/agent-workflow.md#task-graph-and-continuation)
  for dependent work. Continue ready, authorized work through verified
  closeout and preserve its evidence.

**Agent roles:** one role owns planning, architecture, review and integration;
one handles complex implementation; one handles bounded routine work. Which
model fills each role, at what effort, is machine configuration, not tracked.
Use explicit routing when delegation helps, report unavailable routes, and keep
the parent responsible for verification. Detailed
[delegation](docs/agent-workflow.md#models-and-delegation) and
[authority and effort](docs/agent-workflow.md#authority-and-effort) live in the
workflow; these instructions do not change an already running model.

**Stack:** Rust for rules, protocol, simulation, authoring, and one
PostgreSQL-backed server; TypeScript with Three.js world and character rendering, Canvas 2D material and label preparation for the browser client (under the [visual ruling](docs/presentation-direction.md#3d-reopening)); Tauri
for its ruled desktop shell; Python for checks and tools. Widening it needs an
owner decision. Keep adjustable facts in validated content and each mutation in its
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

Documents declare path ownership in front matter. The docs lane validates these
routes and the table below. Read relevant sections; follow history links only
when the task needs their evidence. Keep machine setup and session logs local.

| Start here | When |
| --- | --- |
| [docs/gameplay-baseline.md](docs/gameplay-baseline.md) | historical gameplay, recovered decisions, fidelity and explicit deviations |
| [docs/town-resident-contract.md](docs/town-resident-contract.md) | resident circuits, service providers and temple interactions |
| [docs/class-training-contract.md](docs/class-training-contract.md) | classes, promotion, skill focus and teaching services |
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
| [docs/browser-client.md](docs/browser-client.md) | browser source map, movement, rendering, packet and proof contracts |
| [docs/browser-reference-tools.md](docs/browser-reference-tools.md) | retained local 3D study tools and historical packet/capture contracts |
| [docs/presentation-direction.md](docs/presentation-direction.md) | the visual target |
| [docs/pixel-art-production.md](docs/pixel-art-production.md) | historical pixel-art source, sprites, sampling and native review |
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
| `web/` | browser feel surface; implementation and proof in browser client notes |
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

Repeat `--changed-path` for every changed path. `nix flake check` is the
house gate (rustfmt, clippy, the Cargo tests and ruff's error rules, through
[`flake.nix`](flake.nix)); `nix develop` gives the toolchains. It is a subset of
the runner's `full` scope, not a substitute for it. Use `--scope full` for complete
proof before merge; live Workbench use requires no verification run. Capability
setup, incomplete verdicts, capture, and CI are in
[verification](docs/verification.md). **UNAVAILABLE is never PASS.**

## What needs an owner decision

Reopened product decisions; action timing; multiple live worlds; external
product-boundary activation or outside players; publication; visual acceptance
and accepted masters; adding an AI runtime; Git lifecycle work; spending; and
effects outside this machine. Timing changes follow the current D5 ruling.
Routine private-preview refresh already has standing authorization under the
[latest-build ruling](docs/server-notes.md#private-development-deployment).

If a request conflicts with an owner ruling, the charter, a boundary, or the
public-boundary policy, state the conflict and risk and get explicit owner
confirmation before proceeding. In-scope implementation follows
[implementer autonomy](docs/agent-workflow.md#implementer-autonomy).

## Private

Never committed, never quoted in issues, PRs or logs:

- `.boundary/`: the real banned-term denylist the boundary checks read.
- The predecessor-private roots `.gitignore` names, which
  `tools/check_clean_room.py` asserts ignored and absent.
- Raw external reference payloads, under the
  [public-boundary policy](docs/public-boundary-policy.md).
- Credentials: database URLs, the PostgreSQL superuser URL file, deploy
  secrets ([server notes](docs/server-notes.md)).
