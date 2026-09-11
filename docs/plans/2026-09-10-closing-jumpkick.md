---
last_updated: 2026-09-11
revision: 2
status: Configured full verification and the native three-engine motion proof both pass on the development machine. Exact takeoff/impact visual phase alignment remains the one open owner acceptance question.
public_safe: true
summary: Closing jumpkick scope, implementation choices, regression coverage and receiving-environment limits.
---

# Closing jumpkick

## Scope and authority

The owner dispatched the distance-closing kick after the completed martial-motion
handoff. Base source: `1ab656d520b8838192269d6a88d39c2e1b2199e4`, tree
`8507e9872379d011571a2df448ce95b8597208f4`. Work starts from the tracked source ZIP,
not a development checkout; no original Git ancestry or private resources are
present in this environment.

This slice serves dangerous geography and authoritative individual timing:
validate and commit approach plus physical attack in one existing action, then
make that accepted movement and kick present together. The
[gameplay baseline](../gameplay-baseline.md#martial-artist-combat-sequence) owns the
rule and labels provisional obstacle handling. The
[boundary map](../boundary-map.md) owns navigation, resources, combat, projection
and timing; [browser client](../browser-client.md#dungeon-character-motion) owns
playback. This is a Planning document and execution record, not a second rule owner.

Non-goals: historical fidelity claims, changing damage/range progression/stamina
or action-duration tuning, introducing automatic player attacks, changing class
eligibility, new animation assets, a wire or checkpoint schema, content/geometry
migration, GitHub writes, deployment, or visual acceptance.

## Implementation choices

- Plan the approach read-only through the existing movement evaluator. Movement
  remains the only writer of approach positions. Its step commit is shared with
  ordinary movement; the kick pays physical stamina once, not walking exertion.
- Capture the complete route and re-evaluate before committing it. Commit the
  captured social authorization before relocation; all work remains inside the
  existing engine transaction, including rollback of RNG and durable effects.
- Use existing movement and combat events. Preview, player execution and automatic
  actor selection use the same physical-attack plan. New approach failures carry
  existing typed action-block reasons rather than adding error-text heuristics.
- Stretch a moving flying-kick clip to the server-bounded travel interval. Do not
  infer a route, defer damage to animation completion, or hold input for a clip.

Intentional behavioral break: successful jumpkicks relocate the attacker, including
on misses and blocks. The automatic-actor regression previously moved its attacker
by direct test mutation between kick and punch; remove that shortcut and assert the
real committed movement. The native motion proof now requires a full three-step
server-reported kick and a punch from its landing tile. Its disposable two-target
fixture shares that tile so a fatal kick cannot erase the follow-up proof target;
no production actor, skill or geography is edited. Other carried combat tuning
remains provisional.

## Blast-radius searches

Run these against the full tree, including tests and historical records:

```sh
rg -n -i 'jumpkick|jump.kick|flying.kick' crates web content docs tools
rg -n 'PhysicalAttackPlan|commit_actor_path|MovementStepOutcome' crates/tme-rules
rg -n 'combatCues|movementRoute|movementSeconds|endsAt' web/src web/tests
rg -n 'in place|distance.closing|future gameplay|remains gameplay' docs
```

Canonical owners are updated; prior dated integration receipts remain history.
No trace scenario was found dispatching a jumpkick; metadata-only occurrences do
not require manually inventing replacement goldens. Configured full proof must
still verify that conclusion.

## Required proof

Rules: one-, two- and three-tile approaches in all octants; immediate landing
before outcome; one readiness schedule and stamina charge; subsequent unarmed
fight; a miss/block still lands; unchanged weapon selection; bow-unload movement
side effect; skill-limited reach; no extra attack at distance zero; read-only and
matching preview; late invalidation/rollback; walls, corner cutting, closed/open
local doors, water and paired/automatic transitions; not-ready/suppressed/no-sight
cases; target moved, removed or killed; automatic kick-to-fight without test
teleportation; complete observer movement chain.

Browser: complete authoritative kick routes for both bodies; clip lasts the whole
approach; guard at landing; duplicate updates do not replay; readiness interrupts;
partial routes and initial snapshots never invent movement; punches still rotate.

Configured fast/full lanes, native three-engine motion proof, actual private
boundary scan and saved-state/preview proof remain required on the development
machine. Missing capabilities cannot earn PASS. No publication or deployment is
part of this patch.

## Findings and evidence

1. The receiving environment has no Rust/Cargo on PATH and no private assets,
   database, browsers or real boundary denylist. Dependency-host DNS resolution
   also failed. Keep proof limitations in the hand-back rather than altering
   the pinned compiler, dependency lock or verification lane to manufacture green.
2. Exact takeoff/impact phase alignment in the existing stock clip is not established
   from source. Owner: browser/presentation; proof: native review at each supported
   distance, hit/miss/block and both bodies. Duration synchronization alone does
   not grant visual acceptance.

## Receiving-environment proof

The archive reconstructed Git tree `8507e9872379d011571a2df448ce95b8597208f4`
exactly. A scratch Git index with that original tree and intent-to-add entries for
new files supports tracked-file/diff checks; it supplies no commit history, remote
or original checkout ancestry. Original executable bits are preserved.

The selected fast plan includes every changed path. Its run passed the step-target
inventory, then failed to start `cargo fmt` because Cargo is absent. No Rust build,
format, Clippy, test or WASM-codec pass is claimed. The new 23 Rust test functions
and 23 browser cases are written but unexecuted here.

The separate docs lane completed (routing, diff whitespace and Markdown links).
The boundary lane's review references, hostnames and clean-room checks passed;
its banned-terms mechanism passed only against the synthetic fixture. That lane
correctly reported INCOMPLETE with no real private denylist.

Four Python groups completed: boundary 99 cases (4 skipped), capture 55, harness
58 and verification 158. Both full Python attempts were interrupted by container
execution timeouts in the Workbench group; no whole Python-lane success is claimed.
The actual changed native-fixture builder separately passed assertions for both
bodies, shared landing targets, three-tile skill reach and retained inventory/gold.
These disposable-fixture checks are not database or save-preservation proof.

Node accepted the modified native script's syntax. Supplemental execution of the
actual pure motion module passed 20 checks; changed TypeScript parsed/transpiled
using available TypeScript 5.8.3, not the pinned 5.9.3. Neither operation is the
configured browser test/typecheck/build lane or Three.js/native playback proof.

The hand-back includes complete logs, baseline/changed-file hashes, a patch-apply
receipt and instructions for the required development-machine proof. Do not merge
or refresh the private preview on the strength of the earlier PR's green results.

## Development-machine receipt

The section above records the source-only publication phase and is preserved as
history. The following was observed later on the development machine, at revision
`44a0b21` (branch `chat/closing-jumpkick`, based on `1ab656d`). It supersedes
nothing above; it adds what that environment could not run.

Defects found and fixed, none in the rule path:

- `cargo fmt --all` had never run, so `rust: formatting` failed in both CI jobs and
  cascaded every later step to UNAVAILABLE. Formatting is applied in two
  formatting-only commits so it stays distinguishable from the fixes below; every
  non-whitespace hunk there is rustfmt line reflow.
- **None of the 21 new case functions had ever executed.** They compiled once
  formatted, and then all 21 failed in the shared fixture rather than in the
  behaviour under test. Three setup defects: `profile/skill_progression` resolves
  to a rules base authoring only the fighter growth profile, so a case playing
  `martial_artist` failed content validation before any assertion (the canonical
  martial_artist profile is now taken from `rules/first_room`, as the shared
  action-context helper does); terrain `move_cost` is owned by `navigation`, so two
  cases writing it at the terrain root failed to deserialize; and the swim case
  wrote the removed `traversal` key and then used the real water cost of 2, which
  exhausted the three-point budget before the walkability check and rejected for
  `InsufficientMovementPoints` instead of the traversal kind under test. No
  assertion, expectation, rule, threshold or reach was changed to obtain green.

Commands and results actually observed:

- `cargo fmt --all -- --check` and `git diff --check`: clean.
- `cargo test --locked -p tme-rules --test combat`: **70 passed, 0 failed**,
  including all 21 closing-jumpkick cases.
- `python3 tools/run_verification.py --scope full --report-disk`: **COMPLETE —
  every selected step ran and passed**, including `rust: clippy` under
  `-D warnings`, `gated: PostgreSQL suite, one fresh migrated database per test`,
  `server: trusted TLS sign-in, admission, individual cooldowns, reconnect, and
  logout`, `browser: authoritative Workbench capture, native WSS, replay and
  pointer correspondence`, and `clean clone: builds and tests with no private
  root`. The banned-terms lane ran against the real private denylist.
- `python3 tools/run_dungeon_proof.py` against an immutable release staged from
  this revision (`58358f17`), for `martial-male` and `martial-female` on the full
  chromium, firefox and webkit roster: **six PASS**, every renderer reporting
  hardware GPU execution. The run is a real signed-in session over native WSS with
  a freshly migrated scratch database per engine; captures, per-engine reports and
  `verification.json` (verdict PASS) were written outside the checkout.

What that native run actually established, and what it did not:

- The kick really closes: the harness asserts the server lands the kick on the
  target's tile and reports **exactly three** `actor_moved` receipts for the
  attacker, each with contiguous `from`/`to` and `navigation: walk`, the last
  ending on the destination. Position is server-owned; the client invents no
  displacement.
- The follow-up punch is offered and accepted from the landing tile, and the
  server reports `jab_left` for it. Both bodies (`male` and `female`) were
  exercised and both required-asset-refusal and reconnect guards passed, with no
  page errors.
- Visual review of the captures confirms the airborne flying-kick pose rendered
  over the approach. **It does not settle Finding 2.** The `fight` capture is taken
  immediately after the action is accepted, while the mixer is still crossfading
  from the kick clip, so that frame shows the end of the kick rather than a settled
  punch pose. Exact takeoff, impact and landing phase alignment in the stock clip
  remains an owner acceptance question, and this run grants no visual acceptance.

Remaining limits, stated plainly:

- Finding 2 above is still open and is the only outstanding acceptance question:
  exact takeoff, impact and landing phase alignment. The native run proves
  authoritative movement, single resolution and kick-to-punch continuity at the
  rules and receipt level; it does not prove animation phase quality, and no
  capture in it was reviewed as an accepted master.
- The native run needed a private release staged from this revision. That release
  was written to the development installation's `releases/` store; `current` was
  read before and after and did not change, so nothing was activated and the
  installed preview, its database and its saved characters were not touched. This
  slice was not deployed.
