---
last_updated: 2026-09-05
revision: 3
status: Audit and Godot retirement verified; owner authorized commit, PR, merge, and branch cleanup. Private preview is unchanged.
public_safe: true
summary: Retirement scope, retained proof, lean context, remaining integration gaps, and observed server finding.
---

# Code and context audit

## Scope and authority

Starting HEAD is `9f3f2846c0cbdafa2d162b0365d8eb16e5cc2d81` on `main`.
The owner requested obsolete-code removal and lean, current agent context, then
explicitly retired Godot. The earlier uncommitted scene 06 routing/cursor work
and browser GPU proof launcher are preserved. The private preview is unchanged.

[Client architecture](../client-architecture.md) owns the retirement and standing
contracts; [browser client](../browser-client.md) owns implemented behavior;
[D5](../boundary-map.md#21-authoritative-individual-deadlines-d5) owns timing.
No reference-derived mechanic, visual acceptance, publication, or paused-plan
resumption is introduced by this audit.

## Findings and resolutions

| Finding | Resolution and owner |
| --- | --- |
| Unused Godot runtime, assets, tests and commands consume context and verification capacity | Retired `client/`, launch/capture harnesses, Godot capability and client lane. The retired lane is refused, with no alias. Client architecture owns the cutover. |
| Useful server proof depended on Godot | Python wire observer reuses the production smoke transport. Live proof covers sign-in, correct land, independent deadlines, cooldown rejection, reconnect and connected logout. The semantic recorder uses the same authoritative frames and existing validator. |
| Workbench offered a fresh capture its remaining runtime cannot produce | Removed the command and button. Existing image/identity correspondence and selection remain tested. Browser authoritative capture is an explicit gap in Workbench V0. |
| Browser movement retained beat names and duplicate pace classification | Renamed the module and proof artifacts to movement; retained one strict classifier. Timing and route values are unchanged. |
| TypeScript allowed unused declarations | Removed unused scene bindings and enabled unused-local/parameter checks. |
| Resume context mixed current facts, retired client details and old measurements | Added a browser implementation owner, shortened checkpoint/workflow context, moved measurements to dated evidence, and removed the Godot implementation document. Routing and links remain enforced. |
| Session-root guard covered the retired client | Migrated runtime coverage to `web/` alongside `crates/`. |
| Scratch TLS certificates failed Python strict verification | Added CA signing key usage and explicit server certificate constraints/usages. Live proof passes strict TLS without weakening the client. |
| Disconnect followed by logout returned HTTP 503 | Open server finding in [server notes](../server-notes.md#open-logout-finding-september-5-audit), with required reproduction and regression proof. Connected logout passed; waiting for the close handshake did not resolve disconnected logout. |

## Coverage and remaining work

Inspected server scheduling and deadline application, rules timing conversion,
browser route/movement/presentation, proof producers and judges, verification
inventory, Workbench process boundaries, and active documentation routes.
Server housekeeping does not align action starts. Deterministic test ticks and
authored duration units remain legitimate; no blanket vocabulary rewrite was
applied to unrelated content schemas or dated receipts.

Authoritative browser integration, strict decoder corpus parity, production
chrome, and fresh identity-addressed capture remain open implementation gates.
Distance-matched stride and early visual arrival remain browser presentation
work. The private test server remains issue #40. No private reference payload
entered this checkout. Historical capture receipts retain their original hashes
and producer names; they do not claim a current rendered client proof.

## Verification evidence

External local evidence is under
`/data/dev/home/tme-visual-lab/code-context-audit-20260905/`.
The pre-retirement focused run passed in 43.957 seconds; its subsequent full run
was interrupted by the owner scope change and is not a baseline pass.
The retirement focused run passed 130 tests. The replacement live wire proof
passed with connected logout. Separate close-before-logout runs failed with 503;
`server-wire.log` and `server-wire-handshake.log` preserve those observations.
The full baseline passed in 1175.014 seconds (`final-full.log`): 537 Python tests,
Rust formatting/build/Clippy/workspace tests, 152 browser tests plus typecheck and
build, six PostgreSQL certification cases, live wire proof, and clean-copy proof.
The clean copy intentionally uses the synthetic denylist; the main run passed
the provisioned private denylist. Its disposable build peaked at 6407 MiB and
was removed.

The capture lane passed in 149.143 seconds (`capture-final.log`). The Python
recorder matched the tracked normalized semantic projection. An initial run was
stopped when its barrier did not match: static scene context is a sibling of the
frame on the wire and must be paired in the recording format. A diagnostic run
identified the omission; the corrected recorder preserves both server payloads.
Neither interrupted nor diagnostic run is counted as a pass.

Chromium and Firefox both passed the movement proof using Intel UHD 630 hardware
rendering. Chromium commit/ready and Firefox ready/interior images were opened
and inspected. This checks the candidate scene, not production acceptance or a
performance benchmark. Captures are in the external evidence directory.

The full run's clean copy preceded the final recorder pairing and documentation
cleanup. Those final changes passed the live capture lane, 113 targeted
regressions (`final-regressions.log`), and final docs/boundary checks. Historical
tracked recordings were not promoted or rewritten. Temporary scratch databases,
credential, and role were removed; the existing PostgreSQL cluster is preserved.
At audit closeout, `main` was still at the starting HEAD with the audit and scene
work uncommitted. The owner subsequently authorized Git delivery and branch
cleanup. The private preview remains scene 06.

## Delivery follow-ups

[Issue #42](https://github.com/TusanHomichi/the-mortal-estate/issues/42) tracks
disconnected logout; [issue #43](https://github.com/TusanHomichi/the-mortal-estate/issues/43)
tracks fresh authoritative browser capture. Both remain open obligations; this
cleanup does not claim to implement them.
