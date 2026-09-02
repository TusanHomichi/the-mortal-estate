---
last_updated: 2026-09-02
revision: 15
status: Authored at genesis plan Phase 7; a live index that grows as slices close. Revision 15 records the owner-ruled wall-tile occupancy model.
public_safe: true
summary: The anti-rework index — closed conclusions about this project's code, method, visual projection, cursor readiness, and relational scale; their owners; and what reopening one costs.
always: true
---

# Settled conclusions

This is the anti-rework index. Every row is a question this project has already
answered **about its own code, method, or accepted presentation direction**,
with the document that owns the answer.

It exists because the expensive failure mode for an agent-worked repository is not
a wrong decision — it is the same decision being re-derived every few sessions,
slightly differently each time, with no record of why the last version was
discarded.

**Scope, strictly.** Only conclusions about this project's own implementation,
working method, and owner-accepted presentation direction belong here. **No
gameplay mechanic, value, name, timing, penalty, or route** appears on this list.
Ruling D2 reopened all of them for fresh design, and a "settled" list is exactly
the wrong place for something that is deliberately open — see
[boundary map](boundary-map.md#what-an-authored-seam-does-and-does-not-settle).

## Closed

| Topic | Conclusion | Owner |
| --- | --- | --- |
| Implementation stack | A Rust workspace for rules, wire DTOs, simulation, authoring, and one authoritative server; one thin browser client (TypeScript, Vite, Three.js on WebGL2); the retained Godot shell; Python for repository checks, tools, and proof harnesses. Widening the stack is a decision, not a convenience — the browser toolchain entered by owner ruling on 2026-09-02. | [AGENTS.md](../AGENTS.md) |
| Gameplay authority | One reusable rules boundary owns gameplay truth. The server calls into it; the client consumes its projections. | [boundary map](boundary-map.md) |
| Client shape | Protocol-first and thin: no rules types, no legality inference, no gameplay ledger, no second clock. | [client architecture](client-architecture.md) |
| Engine baseline | The browser client's baseline is Three.js on WebGL2 with TypeScript and Vite on Node 22, dependencies pinned by lockfile and no other runtime dependency without a decision. The retained Godot shell keeps its `4.7.2` / `gl_compatibility` / typed-GDScript baseline unchanged while it stands. Changing either is a contract change with full re-proof. | [client architecture](client-architecture.md#the-web-client) — owner ruling 2026-09-02 |
| Feel surface | **Superseded 2026-09-02:** the browser client is the feel surface — movement, cursor, camera, lighting, and layering feel are judged in a browser tab at the 1280 × 800 minimum play surface, rendered by real WebGL geometry and lights. The earlier finding stands only for what it tested: a DOM-based board is not a feel surface and is not to be rebuilt as one. The Godot client is no longer the feel surface; it is retained, cold, until its desktop role is decided. | this document — owner ruling 2026-09-02 |
| World-view projection | A fixed orthographic 2:1 dimetric projection: 45-degree yaw, 30-degree elevation, no perspective convergence, and square world cells projected as 2:1 screen diamonds. The screen-square high-oblique and true-isometric candidates are superseded. | [presentation direction](presentation-direction.md#projection-and-surface-ruling) — owner ruling 2026-08-31 |
| World-up standing geometry | Actors, walls, doors, posts, props, trees, and monsters use ordinary volumetric world-up construction. The accepted camera applies no camera-facing shear and no shared northwest lean; facing rotates around world up. | [presentation direction](presentation-direction.md#projection-and-surface-ruling) — owner ruling 2026-08-31 |
| Tile assembly | A restrained material joint may remain visible between logical square world cells after they project as 2:1 diamonds. Adjacent cell interiors share material and scale grammar but need not continue pixel-perfectly. Exact raster source resolution, native diamond bounds, tile-edge length, and joint width remain representative-scene calibration; the retired 64 × 64 screen-cell contract carries no authority. | [presentation direction](presentation-direction.md#tile-assembly-ruling) — owner rulings 2026-08-30 and 2026-08-31 |
| Cutaway building assembly | Building floors, wall runs, and openings use the same cell ruler. Active play is roof-off by default; a separately controllable roof layer never owns footprint, wall height, door scale, occupancy, interaction, or sorting. | [presentation direction](presentation-direction.md#tile-assembly-ruling) — owner ruling 2026-08-30 |
| Walls occupy tiles | A wall owns its tile and is drawn as a thin strip on the tile's camera-facing edge; no character ever occupies a wall tile. A door is a wall tile that can be crossed. Sight is blocked by walls. | [presentation direction](presentation-direction.md#tile-assembly-ruling) — owner ruling 2026-09-02 |
| Ground-contact anchoring | Actors keep one authoritative feet/contact anchor at the centre of one logical square ground cell. Their volumetric world-up bodies may cover neighboring projected diamonds. A doorway threshold remains a separate presentation point. Visual extent and draw order never redefine occupancy, collision, walkability, or semantic targeting. | [presentation direction](presentation-direction.md#tile-assembly-ruling) — owner rulings 2026-08-30 and 2026-08-31 |
| Movement lands on the beat | A move is a player-authored direct route of one to three squares that lands whole on the next strike of the beat; the figure stands on its square until then, and nothing slides between squares. A route that cannot be authored is refused, never re-routed. The route allowance and pace remain reopened under D2. One, two, or three squares is a walk, run, or sprint; the camera stays centred on the player and re-centres on the strike. | [presentation direction](presentation-direction.md#the-in-engine-feel-scene) — owner ruling 2026-09-02 |
| Readiness is the cursor | The pointer tells the player whether they can act again; there is no visible pulse outside combat — no meter, bar, or tick. Whether combat surfaces a pulse is open. | [presentation direction](presentation-direction.md#the-in-engine-feel-scene) — owner ruling 2026-09-02 |
| Relative visual scale | The logical terrain cell is one shared ruler, not one object size. Adult humans are the review baseline; buildings must read as occupiable volumes larger than people, and subjects intended as large must be visibly taller/more massive when measured under the accepted projection at the same ground contact and display scale. Exact dimensions remain authored and test-calibrated. | [presentation direction](presentation-direction.md#relative-scale-ruling) — owner rulings 2026-08-30 and 2026-08-31 |
| One world | One canonical persistent world; no player-selectable divergent copies. Enforced in the schema by a singleton index, not only in code. | [server notes](server-notes.md#the-world-instance-and-what-it-is-for) — owner ruling D4 |
| Logical time | The rules clock is logical rounds, deliberately independent of wall-clock seconds. Wall time belongs to the server, and the cadence is one value in one place. | [boundary map](boundary-map.md#14-deterministic-logical-time) |
| Presenting the pulse | The client is never told the cadence — nothing on the wire carries one — so it **measures** the beat from consecutive frame arrivals and interpolates inside it. Readiness stays the frame's `can_act`, the wait stays the frame's arithmetic, and the fill is never extrapolated past what was measured. Adding a cadence field to the wire would put a one-place value in two places. Where it is presented is ruled by "Readiness is the cursor". | [client notes](client-notes.md#the-pulse-made-visible) — under owner ruling D5 |
| Content is data | Gameplay facts live in validated content, not in Rust logic. Rules consume content; they do not secretly define it. | [boundary map](boundary-map.md#11-the-authoredruntime-contract-seam) |
| Content validation | Rust is the sole gameplay-semantic validator, and it fails closed. A validator that cannot run does not pass. | [boundary map](boundary-map.md#13-rust-is-the-sole-gameplay-semantic-validator-and-it-fails-closed) |
| Authoring format | The runtime never loads the authoring document. A compiler turns authored input into proven runtime content, deterministically. | [authoring compiler](authoring-compiler.md) |
| One wire corpus | Both sides of the wire are proven against one shared fixture corpus, read rather than copied. Two copies of a contract drift, and the drift shows up as a passing test on each side. | [client notes](client-notes.md#the-wire-fixture-corpus-is-read-not-copied) |
| Blocking checks | A fail-closed check runs advisory until it kills a deliberate mutant, and the kill names the exact test. (P9.) | [boundary checks](boundary-checks.md#qualification) |
| Denylist convention | The matching mechanism is public and carried; the terms are private and ignored. A missing term file fails the check closed, and a fresh clone therefore fails it until the owner provisions the file. That is intended, not a setup bug. | [boundary checks](boundary-checks.md#the-private-terms-convention) |
| Public source | This repository begins at the audited allowlisted public-source cut. The private development history remains in an owner-held read-only archive; no private Git or collaboration object crossed. Publishing source does not activate the external product boundary or satisfy G10/G11. | [public boundary policy](public-boundary-policy.md#the-clean-public-successor-and-the-two-publication-cuts) — owner ruling 2026-08-27 |
| Two enforcement points | Repository scanning cannot see content that never enters the tree, so the rules crate carries the same convention at content-load time. | [boundary checks](boundary-checks.md#the-second-enforcement-point-the-content-validator) |
| Migration pattern | While pre-external-boundary: one atomic cutover with every caller, fixture, test, and golden migrated together. No dual schemas, no adapters, no staged compatibility. | [agent workflow](agent-workflow.md#no-compatibility-adapters) |
| Test architecture | Do not equate one source file with one test process, and do not duplicate gameplay semantics in a second language. Rust harnesses follow behaviour owners; repository, process, and deployment checks are Python's. | [agent workflow](agent-workflow.md#verification) |
| Proof method | Prove the real path, not a reconstruction of it; and assert the real device, session, or backend is in use rather than inferring it from the absence of an error. | [agent workflow](agent-workflow.md#verification) |
| Retired pipelines | Earlier presentation pipelines carry no automatic authority. Re-entry is through the accepted micro-scene gate, not through a migration step that copies a file across. | [presentation direction](presentation-direction.md#retired-pipelines-carry-no-authority) |
| One verification source of truth | `tools/run_verification.py` owns the step table and the lanes. Documentation names the runner and its `--list`; CI names a lane. Nothing hand-copies a command list, because a second source of truth is a drift nobody has noticed yet. | [AGENTS.md](../AGENTS.md#verification-baseline) |
| The lane split | The fast lane is defined by what it **excludes** and the complete lane by running everything. A change nothing recognises escalates to the portable baseline and says why. Neither lane may drift toward the other; a test asserts the partition. | [agent workflow](agent-workflow.md#verification) — charter §8 |
| UNAVAILABLE is never PASS | A step whose capability is absent does not run, is reported with the reason, and makes the run INCOMPLETE (exit 3). `--allow-unavailable` is a caller declaring a limit out loud, and is the only way an incomplete run exits 0. | [AGENTS.md](../AGENTS.md#what-the-exit-code-means) |
| The ignored working root | Kept, and non-authoritative by construction: no tracked proof reads it, clean clones carry tracked synthetic fixtures instead, missing private fixtures produce an honest unavailable, sessions carry source digests and can never be runtime input, retention is ruled, and nothing enters tracked content except through the promotion path. | [working-root policy](working-root-policy.md) — owner ruling D6 |
| Where a spec starts | The first-step rule is charter, then the reopened list, then the fact's owner, then what the tree already proves, then original design work with the owner. The predecessor's research-corpus first step is deleted, not adapted. | [agent workflow](agent-workflow.md#where-authoring-a-gameplay-spec-starts) |
| A land's member count is content | The compiler compiles a table of lands, each declaring its own members. One member and three run the same code, so growing a land is a declaration plus a re-attestation — never a change to a type. | [authoring compiler](authoring-compiler.md#what-it-compiles) |
| Which world is served | Two documents, two owners: a land's `world.json` says which content makes its world, and a bootstrap manifest binds that to a deployment's accounts. There is no default manifest, no built-in land, and no fallback — a process with neither refuses and says which it wanted. | [server notes](server-notes.md#which-world-the-one-process-serves) |

## Traps already paid for

Measured, reproduced, and recorded — each cost real time once and should not cost
it again.

| Trap | Owner |
| --- | --- |
| A length gate above an equality test silently deletes that test for every value shorter than the gate. Context checks belong below the gate, and must match the way the value can actually appear. | [server notes](server-notes.md#the-defect-this-policy-was-written-against) |
| The engine normalises `res://..` inside directory access while file access follows it, so a relative fixture path lists the wrong directory and an inventory assertion passes against it. Bind shared fixtures by absolute project-root path. | [client notes](client-notes.md#the-wire-fixture-corpus-is-read-not-copied) |
| A headless run cannot capture a viewport. Ask the display server and refuse with the reason, rather than writing a blank picture with a confident sidecar. | [client notes](client-notes.md#captureemitter-the-presenter-obligation-discharged) |
| Each PostgreSQL-gated test assumes a **fresh** database. Two gated tests against one database fail on cross-test state — observed, not hypothetical. | [server notes](server-notes.md#one-fresh-migrated-database-per-gated-test) |
| A gated test with no runner compiles on every run and executes on none. An inventory that fails closed on an unrun `#[ignore]` is what stops it recurring. | [server notes](server-notes.md#the-inventory-fails-closed) |
| A test runner that launches the compiled binaries directly bypasses cargo's `[env]` table, so `.cargo/config.toml` never applies and the tests silently run against a different configuration than they were written for. Cargo owns the test environment; ask it to launch each target. | [agent workflow](agent-workflow.md#verification) — `tools/verification/rust_tests.py`, and the Rust tripwire `cargos_env_table_reaches_this_test_process` |
| The single source of truth for verification can be wrong about its own contents. The predecessor's runner named a Python module that existed nowhere, and one scope failed on every run until somebody ran it. | [agent workflow](agent-workflow.md#the-single-source-of-truth-may-not-name-something-that-is-not-there) |
| Bare dotted names in source are indistinguishable from hostnames by shape alone. The hostname check trims identifier vocabulary from its tier-3 set by measurement, and proves the trimmed labels still die in URL form. | [boundary checks](boundary-checks.md#hostname-tier-3-and-identifier-vocabulary) |

## Reopening one

A row here is closed, not permanent. Reopening one takes:

1. **New evidence** — a measurement, a failure, or a requirement that did not
   exist when the row was written. "A different approach also exists" is not new
   evidence.
2. **The owner's decision**, where the row cites an owner ruling. Rows citing D4
   or D5 cannot be reopened by an implementer at all.
3. **The migration**, in the same slice. Reopening a conclusion means changing
   every caller, fixture, test, and golden that depends on it — the
   no-half-migration rule applies to decisions as much as to code.
4. **The record.** The row is updated or removed here, with what replaced it. A
   silently stale row is worse than no list, because the next agent trusts it.

## Adding one

Add a row when a slice closes a question that would otherwise be re-argued: name
the conclusion in one sentence, and link the document that owns it. **Do not
restate the reasoning here** — this is an index, and an index that grows its own
explanations becomes a second copy of the documents it points at.
