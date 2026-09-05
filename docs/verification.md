---
last_updated: 2026-09-05
revision: 4
status: Standing verification usage including native/WebAssembly protocol proof and gated authoritative browser capture.
public_safe: true
summary: Lane usage, capabilities, shared native/browser codec proof, gated capture, exit codes, and local evidence.
routes:
  - tools/run_verification.py
  - tools/verification/**
  - tools/run_rust_tests.py
  - tools/run_clean_clone_proof.py
  - tools/run_server_live_proof.py
  - tools/live_wire_client.py
  - tools/logout_proof.py
  - tools/run_browser_capture_proof.py
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
| Focused checks on what changed | `--scope fast --changed-path <path> ...` | seconds; **no web run unless browser files changed, and no workspace lane unless Rust changed** |
| Exact gameplay preview capture | `--scope capture` (owner-invoked, outside the standing baseline) | minutes, and needs the selected browser packet or database and capture output |
| Complete proof before merge | `--scope full` — what CI runs | the whole workspace |

The fast lane is defined by what it **excludes** and the complete lane by
running everything; neither is allowed to drift toward the other, and a test
asserts the step table's partition. Anything the fast lane does not recognise
escalates to `portable` and says why — a guess is never cheaper than a build.
Escalation is a floor, not a ceiling: the lanes the recognised paths beside it
select (`web`) still run.

One honest qualification, since Workbench V1: the Workbench reaches the authoring
compiler's semantics through one command, because there is exactly one
implementation of them. So its own proof **invokes `cargo run -p tme-authoring`**
— a rebuild check on a current workspace, and a real build on a cold one. That is
not the workspace lane (no `fmt`, no `clippy`, no workspace test), and it is not
free either. See [Workbench V1](workbench-v1.md#the-cost-of-the-v1-loop-measured).

Use `--help` and `--list` for the current scope inventory. The browser proof
is the `web` scope. The retired `client` scope is refused.

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
is missing — CI passes it, because CI has no database or
private denylist. It is a stated limit, not a skip.

## Capabilities

| Capability | Supplied by |
| --- | --- |
| `browsers` | installed Playwright Chromium and Firefox; the actual capture still probes and requires working WebGL2 renderers |
| `node` | a `node` on `PATH` whose major version is 22 or later, plus `npm` — asked, never assumed. Absent, the `web` lane is `UNAVAILABLE` |
| `postgres` | `TME_PG_ADMIN_URL_FILE`, naming a readable file holding a superuser URL used only to create and drop scratch databases, plus `psql` |
| `private-terms` | the private file resolved by `tools/boundary_common.py` (this checkout first, then its main checkout for a linked worktree). Absent, the banned-terms check **degrades** onto the tracked synthetic fixture: the mechanism still runs and still must pass, and the run says the real denylist was not proven |
| `feel-assets` | `TME_FEEL_ASSETS`, an absolute candidate-packet directory outside the checkout |
| `capture-output` | `TME_CAPTURE_OUTPUT`, naming a directory |

## Historical baseline, 2026-08-20

This is a dated receipt, not the current test inventory. On that date,
`--scope full` with every capability supplied: 1363 Rust tests across 32
executables plus 5 doctest runs, 330 Python tests, 147 client tests across 26
suites, five boundary checks, six gated PostgreSQL tests each against its own
fresh migrated database, and a clean copy of the carried set that builds and
tests with every ignored root absent.

## On-demand proofs

`--scope capture` runs two-engine browser movement/captures and records an
authoritative observer frame for the paused presentation experiment. It needs
external inputs; inspect the resolved plan before invoking it. Browser screenshots
do not supply Workbench identity rasters or prove authoritative browser integration.

The live server wire proof is part of `gated`, with a scratch PostgreSQL database:

```bash
python3 tools/run_server_live_proof.py --admin-url-file <file>
python3 tools/run_presentation_adoption_recording.py \
  --admin-url-file <file> --output <directory>
python3 tools/workbench/serve.py
python3 tools/workbench_demo.py
```

The observer recorder consumes the real TLS/WebSocket frame and validates its
semantic barrier. It does not render a client. Old Godot launch, capture, and
`--godot` arguments are retired, with no compatibility aliases.

`tools/run_production_smoke.py` exercises a **deployed** host through public
HTTPS/WebSocket. It needs a running deployment and is not part of any lane.

## Browser evidence

The standing `web` lane proves install, typecheck, unit tests, and build. Its
tests rebuild the Rust WebAssembly codec and consume the shared wire corpus.
The gated browser capture proof additionally exercises both real browser
engines through native WSS, exact replay, GPU/raycast correspondence, and the
Workbench HTTP operation. It requires PostgreSQL and both installed browsers;
missing capabilities are unavailable. Neither lane grants visual acceptance. Candidate-packet
walk and screenshot commands, and their external inputs, are documented in
[browser client](browser-client.md#operation-and-proof).

## Process lifetime

A foreground tool timeout or session cleanup can kill a proof or local server
even after its shell has printed a successful startup. Use the execution
environment's persistent session facility or a local user service when needed,
then verify the process and endpoint from a separate command. A launch message
alone is not proof that a service survived. Keep the exit status and complete
log for a long verification run; do not count an interrupted run as a pass.
