---
last_updated: 2026-09-11
revision: 3
status: Activated in expedition content; configured full verification and a deterministic native incoming-block scenario both pass on the development machine. Owner visual acceptance and the saved-state cutover remain outstanding.
public_safe: true
summary: Activate existing martial hand defense in expedition content, qualify its original provisional curve, and track remaining class gaps.
---

# First-expedition martial defense

## Dispatch and boundary

The owner requested the next useful Martial Artist slice while away from the
local development installation, continuing the historical gameplay target. This
slice is independent of the closing-jumpkick change: it starts at `1ab656d` and
does not change movement, attacks, timing, renderer code or assets.

The actual expedition catalog selected **no profession actions**. Hand blocking
existed in `engine/professions.rs` and the profession-action test fixture, but
that was not a class capability available to the expedition's characters.

Activate the existing mechanism through validated authored content, and prove
it using the real expedition catalog, authored world and character-creation
path. Do not copy the test fixture's deliberately extreme block configuration
into the played land. The [class contract](../class-training-contract.md#martial-artist-defense)
owns the selected behavior and original provisional tuning. The
[gameplay provenance](../../content/lands/first-expedition/gameplay.provenance.md)
records its evidence boundary. No schema, shared combat formula, equipment
restriction or starting profile is changed.

The catalog is a typed data registry, not a routinely read code/orientation
owner. Its two localized registry/selection edits do not introduce a source-file
split or new loader. All edited code and Canonical prose remain below the
workflow's decomposition threshold.

## Implementation and proof scope

- Select one class-specific passive hand-block configuration in the played
  profile. No active Block command or new learned ability is added.
- Retain the existing class/item/skill checks, armor and combat-add penalties,
  candidate selection, block-before-damage resolution and event projection.
- Add integration tests loading the accepted authored land and its actual
  catalog. Use normal character creation, stage isolated test duels, and test
  content activation, class gating, skill thresholds, equipment transitions,
  incoming weapon penetration, deadline/resource behavior and observed events.
- Add a disposable test-only armor entry to exercise the existing worn-versus-
  stowed penalty. It is explicitly not production gear or a historical item.
- Prove ordinary checkpoint recovery rejects the changed definition, and test
  an explicit no-retirement/no-relocation content migration plus deterministic
  replay. This is a scratch proof, not a migration of the owner's saved world.

The test harness may stage player positions and skill levels for a controlled
comparison. It does not replace the block configuration, roll comparator or
combat resolver with a test implementation. Duel commands explicitly authorize
unsafe attacks in their disposable worlds; no player's live consent is inferred.

## Deployment boundary

Selecting the profession action changes the authoritative definition digest.
Existing saves must use the [offline content cutover](../server-notes.md#offline-content-cutover),
with actual before/after bootstrap digests and the operator's usual stopped-writer,
backup, preservation and rollback proof. Ordinary hydration must continue to
refuse an old checkpoint. No reset, automatic migration, deployment, Git merge
or visual acceptance is part of this source-only slice.

## Remaining class work

| Finding | Owner and replacement proof |
| --- | --- |
| The block curve and shared armor/penetration numbers are original provisional tuning; the read sources do not supply an exact formula. | Class contract and gameplay provenance; reconcile selected-version evidence and exhaustive roll/threshold cases before claiming numerical fidelity. |
| `barefoot_full_effect` is descriptive only: damage resolution does not consume it, and ordinary Kick currently always reports full effect. | Gameplay baseline and physical attack owner; decide and implement a supported footwear effect for both kick modes, then test actual outcomes and metadata together. |
| Hand skill currently contributes to hit chance, reach and blocking; the raw physical damage formula has no independent hand-skill damage scaling. | Gameplay baseline and combat owner; recover the selected damage behavior or author explicit temporary tuning, with attacker/defender/equipment outcome cases. |
| Low-skill kick mishaps, gauntlet effects, defensive practice and advanced instruction remain unimplemented or unreconciled. | Class contract and combat/training owners; separate evidence-backed slices. Do not infer formulas from guide advice. |
| Hand-block eligibility currently ignores held gold and action-suppressing effects. Existing behavior is retained, not endorsed as final historical correspondence. | Profession/weapon owners; recover hand-occupancy and incapacitation semantics, then change eligibility and all callers together. Do not silently extend this content activation into a global blocking rewrite. |
| The expedition has no wearable armor item; only the underlying worn-armor rule can be exercised here with a labeled fixture. | Authored content owner; original, proven gear authoring is a separate delivery. |
| Recorded: `defense-male`, `defense-female` and `defense-occupied-hand` PASS on all three rostered engines against an immutable release; see the native incoming-block receipt below. | Owner/browser; Peter supplies visual acceptance of those captures. Passing scenario assertions are not acceptance, and the wire outcome names no block source, so the martial-hand attribution rests on the unarmed, unarmoured, shieldless fixture plus its occupied-hand control. |

Separately, the closing-jumpkick draft's first CI attempt stopped at formatting
in both jobs. Its build, lint, Rust and browser results remain unobserved. Fix
that in its own branch; do not mix the two changes or treat the earlier motion
baseline's proof as verification of either.

## Verification receipt

The implementation handoff records commands actually observed in the receiving
environment. Full verification, pinned formatting/compilation, native playback,
private-denylist and actual saved-state proof remain required before merge.
Missing capabilities and an interrupted lane are not passes.

Observed in the source-only environment:

- The source archive reconstructed the pinned base tree exactly. The working
  Git root is a synthetic local snapshot, not recovered commit ancestry.
- The complete six-path fast plan resolved. Its step-target check passed; the
  formatter could not start because Cargo was absent. The selected lane failed
  and did not compile, lint or execute Rust.
- Documentation routing, whitespace and links passed. Boundary mechanisms passed
  with the synthetic denylist; that lane exited incomplete, not fully verified.
- Four Python groups ran through the repository's `run_step` implementation:
  366 passed and four skipped. Bytecode compilation passed. Workbench's Python
  group and the complete Python lane were not run for this slice.
- Eighteen supplemental data/structure/arithmetic assertions passed. They check
  the activation's scope and declared curve; they do not run the combat engine.
- Eight new Rust integration test functions are written but uncompiled and
  unexecuted. Their saved-state migration and replay assertions remain pending.

The two source patches are checked for application in either order in the
handoff. File-level independence is not combined gameplay or native proof.
No remote branch, pull request, merge, deployment or saved-state mutation was
performed for this slice.

## Development-machine receipt

The receipt above records the source-only publication phase and is preserved as
history. The following was observed later on the development machine, at
revision `f1f7fc6` (branch `chat/martial-defense`, based on `1ab656d`). It
supersedes nothing above; it adds what that environment could not run.

Two test-only defects were found and fixed before any of this could run:

- `crates/tme-authoring/tests/martial_defense.rs` matched
  `ObservedEventV1::PhysicalCombat`, which is not a variant of that enum, and the
  file therefore did not compile at all. Observed feedback arrives as
  `ObservedEventV1::Feedback { cue }` with `ObserverFeedbackCueV1::PhysicalCombat`
  inside it; the case now matches that documented nesting with no assertion change.
- `armor_definition()` authored an all-zero armor rating, which the shared armor
  validator rejects, so the definition panicked before any case ran.
  `block_rating` cannot be raised to satisfy the validator instead, because
  `crates/tme-rules/src/engine/combat.rs` evaluates an armor-sourced block
  candidate whenever `block_rating > 0`, which would add a second block event and
  contradict the single-candidate assertions. The fixture now carries the minimal
  crushing reduction (the kick damage kind is crushing) and records why the rating
  stays zero. Encumbrance 5 with a 30 percent threshold is unchanged.

Commands and results actually observed:

- `cargo fmt --all` applied; `cargo fmt --all -- --check` and `git diff --check`
  are clean.
- `cargo test --locked -p tme-authoring --test martial_defense`: **8 passed, 0
  failed**. All eight new integration functions now execute.
- `python3 tools/run_verification.py --scope full --report-disk`: **COMPLETE —
  every selected step ran and passed**, including `rust: clippy` under
  `-D warnings`, `gated: PostgreSQL suite, one fresh migrated database per test`,
  `server: trusted TLS sign-in, admission, individual cooldowns, reconnect, and
  logout`, `browser: authoritative Workbench capture, native WSS, replay and
  pointer correspondence`, and `clean clone: builds and tests with no private
  root`. The banned-terms lane ran against the real private denylist.

Remaining limits, stated plainly:

- The blocking curve remains the explicit provisional integration choice recorded
  above. Nothing here promotes it to recovered historical fidelity, and the owner
  still supplies visual acceptance for the captures below.
- This receipt establishes the rules and content activation only. It is not
  saved-state preservation proof: a passing disposable migration test does not
  demonstrate preservation of the owner's actual saved world, and activating this
  content against existing saves still requires the save-preserving offline
  cutover owned by the server notes.
- The installed preview, its database and its saved characters were not touched,
  and this slice was not deployed.

## Native incoming-block receipt

The limit recorded in the first version of this section — no deterministic native
defender-block receipt — is now closed by scenario work in
`tools/run_dungeon_proof.py` and `web/proof/dungeon-block-proof.mjs`.

**Why the fixture colocates the two actors.** `Fight` is legal only at distance
zero (`crates/tme-rules/src/engine/physical_attacks.rs`), and the authored
`cellar_scavenger` is `hold_ground`, so it never closes distance itself. An
earlier probe placed the monster on an *adjacent* tile and observed no attack; that
was the range rule, not a hostility or target-selection defect. Colocating them is
what makes the existing automatic attack path reachable. No gameplay was retuned,
no monster was taught to chase, and no combat event is manufactured.

**What the scenario asserts.** The controlled character is the defender and
**no command is sent** — the server's own automatic attack produces the swing. The
run asserts the monster is present, assesses the observer `open_hostile`, and
shares the defender's tile; that an incoming `fight` is reported `blocked`; that
the incoming-block clip is the reported clip **at capture time**; that the defender
does not travel; and that it returns to the ordinary stance. Captures are written
outside the checkout.

**Negative control.** The same fixture with the starting `weathered_staff` left in
the right hand, so no martial hand candidate exists. Observed: **seven incoming
swings, all `missed`, zero blocked**. That the attacks still occurred and produced
non-blocked outcomes is what makes the control non-vacuous.

**Attribution.** The wire `Blocked` outcome deliberately names no block source, and
the client's cover pose is documented as never claiming one. Attribution therefore
rests on the pair: the defender is an unarmed, unarmoured Martial Artist holding no
shield or weapon, so the martial hand is the only candidate, and occupying that
hand removes the block. Whether that pair is *sufficient* is the owner's judgement;
this record does not award it.

**Observed result.** `defense-male`, `defense-female` and `defense-occupied-hand`
each PASSed on chromium, firefox and webkit — nine scenario passes, every renderer
reporting hardware GPU execution — against an immutable release staged from this
line of work. The matrix was run twice to check for flakiness after an initial
single-engine failure was traced to a proof-side race (an assertion on the starting
clip that the monster's cadence-1 first swing could beat) and removed.
