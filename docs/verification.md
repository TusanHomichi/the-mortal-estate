---
last_updated: 2026-09-04
revision: 1
status: Standing verification usage, moved from AGENTS.md during the documentation audit; no lane or acceptance rule changed.
public_safe: true
summary: Verification commands, lane selection, capabilities, exit codes, and on-demand proof; the runner owns the executable step table.
routes:
  - tools/run_verification.py
  - tools/verification/**
  - tools/run_rust_tests.py
  - tools/run_clean_clone_proof.py
  - tools/run_client_live_proof.py
  - tools/run_fixture_land_capture.py
  - tools/run_pulse_capture.py
  - tools/run_presentation_adoption_recording.py
  - tests/test_verification_*.py
  - tests/test_ci_workflow.py
  - .github/**
---

# Verification

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
[agent workflow](agent-workflow.md#the-disk-budget).

## The four lanes

Choose the loop that matches the work:

| Loop | Command | What it costs |
| --- | --- | --- |
| Live Workbench iteration | **no verification run at all** — using the Workbench requires none | milliseconds |
| Focused checks on what changed | `--scope fast --changed-path <path> ...` | seconds; **no client or web run unless that client changed, and no workspace lane unless Rust changed** |
| Exact gameplay preview capture | `--scope capture` (owner-invoked, outside the standing baseline) | minutes, and needs the pinned client, a display, a database, and capture output |
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
free either. See [Workbench V1](workbench-v1.md#the-cost-of-the-v1-loop-measured).

Use `--help` and `--list` for the current scope inventory. The browser proof
is the `web` scope; the retained Godot shell uses `client`.

## What the exit code means

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

## Capabilities

| Capability | Supplied by |
| --- | --- |
| `node` | a `node` on `PATH` whose major version is 22 or later, plus `npm` — asked, never assumed. Absent, the `web` lane is `UNAVAILABLE` |
| `godot` | `TME_GODOT`, naming a binary whose version is exactly `4.7.2.stable.official.ed1daf0bf` — asked, never assumed from the path |
| `postgres` | `TME_PG_ADMIN_URL_FILE`, naming a readable file holding a superuser URL used only to create and drop scratch databases, plus `psql` |
| `private-terms` | the private file resolved by `tools/boundary_common.py` (this checkout first, then its main checkout for a linked worktree). Absent, the banned-terms check **degrades** onto the tracked synthetic fixture: the mechanism still runs and still must pass, and the run says the real denylist was not proven |
| `display` | `DISPLAY`, or `xvfb-run` |
| `capture-output` | `TME_CAPTURE_OUTPUT`, naming a directory |

The engine's class cache is not tracked. A fresh checkout or worktree has none,
and every new `class_name` invalidates it; in both cases the client lane and the
live proof fail to parse scripts until it is rebuilt:

```bash
cd client && "$TME_GODOT" --headless --path . --import
```

## Historical baseline, 2026-08-20

This is a dated receipt, not the current test inventory. On that date,
`--scope full` with every capability supplied: 1363 Rust tests across 32
executables plus 5 doctest runs, 330 Python tests, 147 client tests across 26
suites, five boundary checks, six gated PostgreSQL tests each against its own
fresh migrated database, and a clean copy of the carried set that builds and
tests with every ignored root absent.

## On-demand proofs

Outside the standing baseline because each needs something a checkout does not
carry. `--scope capture` includes the live, fixture-land, pulse, and presentation-adoption
proofs; inspect its resolved plan before running it.

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

## Browser evidence

The standing `web` lane proves install, typecheck, unit tests, and build. It
does not claim a real-tab capture or visual acceptance. Candidate-packet
walk and screenshot commands, and their external inputs, are documented in
[client notes](client-notes.md#browser-operation-and-proof).

## Process lifetime

A foreground tool timeout or session cleanup can kill a proof or local server
even after its shell has printed a successful startup. Use the execution
environment's persistent session facility or a local user service when needed,
then verify the process and endpoint from a separate command. A launch message
alone is not proof that a service survived. Keep the exit status and complete
log for a long verification run; do not count an interrupted run as a pass.
