---
last_updated: 2026-09-07
revision: 5
status: Historical gameplay baseline with owner-directed temple resident behavior and explicit provisional values.
public_safe: true
summary: Exact gameplay target, rebuilt-town resident direction, fidelity accounting and provisional values.
routes:
  - crates/tme-rules/**
  - content/**
  - docs/plans/*land*
  - docs/plans/*town*
  - docs/plans/*research*
---

# Gameplay baseline

## Owner direction

**2026-09-06:** reproduce the selected historical game's gameplay exactly on the
modern stack with modern graphics, then change it through deliberate owner
decisions. Familiar classes and their mechanical relationships are the baseline.
Agents must not invent replacements merely because the current documentation
omits them. This replaces the blanket requirement to design every mechanic fresh.

**Owner clarification:** duplicate the player-experienced game in every way
except graphics and technology, while retaining the explicitly requested world
changes. Match observable rules, outcomes, pacing and interactions using our
modern stack. Recreating historical source code, protocols, storage formats,
scheduling internals or server architecture is not the objective. A different
implementation is appropriate when it reproduces the same gameplay. An unknown
internal implementation is not itself a fidelity blocker.

The [public boundary](public-boundary-policy.md#historical-gameplay-reconstruction)
owns the research and derivation permission. This document owns the product
target and fidelity accounting. The [surface brief](plans/2026-09-05-first-land-surface.md#living-town-dead-town-and-dungeon)
owns the new living town, destroyed town playable while dead, initial four
dungeon levels and three deeper levels reserved for later content.

The target applies to classes, promotion, skills, training, combat, magic,
resources, inventory, banking, lockers, social law, progression, services and
other historical gameplay systems. Exactness is a target to prove, not a claim
that the carried implementation already matches. Source release 1.11 is an
available evidence source; it is not an assertion that later releases behaved
identically. Contradictory versions remain visible until selected explicitly.

## Explicit differences and conflicts

- Modern stack, current accepted camera and visual direction remain the
  presentation target. New graphics do not authorize mechanical simplification.
- The living/dead-town arrangement and rebuilt temple are explicit world changes
  owned by the surface brief. The September 7 [resident direction](town-resident-contract.md)
  adds Maude's rear supply corner, direct buying and Tomas's wandering within
  that rebuilt temple. The destroyed settlement is a place to quest while
  dead; a dead-world layer is not another numbered dungeon depth.
- The September 5 [individual-deadline ruling](boundary-map.md#21-authoritative-individual-deadlines-d5)
  describes the current implementation. Its modern scheduler need not resemble
  historical internals. Its observable movement, readiness and pacing must be
  compared against the gameplay target; implementation survival does not excuse
  a behavioral difference. Record discrepancies for a bounded, proven cutover.
- Existing death, lineage and succession direction must be reconciled with the
  baseline and new dead-town play. Do not silently delete those ideas or declare
  their earlier proposals exact historical mechanics. The relevant boundary
  owners continue to own their state transitions.
- Public names and original writing follow the naming policy. A naming decision
  cannot silently change the class or mechanic it labels.

Further deviations require an owner decision and an explicit record of what
changed. Earlier provisional tuning and the candidate's generic trainer signs
are not deviations approved by survival in code or art.

## Evidence and disposition

### First-expedition provisional values

**Owner ruling (2026-09-06):** use documented temporary approximations where
the research does not establish an exact number or rule, and continue building
the playable first expedition. This explicitly permits retaining the carried
combat tuning and training model while their unresolved correspondence is
tracked. Supported historical behavior still binds; this is not permission to
replace known mechanics for convenience or claim numerical parity.

Each approximation records the missing evidence, selected temporary behavior,
owning implementation/content, and evidence or decision required to replace it.
New evidence triggers reconciliation and behavioral proof through the same
modern rules boundary. Provisional choices do not become accepted historical
facts through persistence or passing tests. The class/training owner maintains
its numerical gap; the expedition record tracks implementation work.

### Required records

Every researched feature must distinguish these independent questions:

| Question | Required record |
| --- | --- |
| What did the historical game do? | Exact source identity/version, observation or extraction method, confidence and unresolved conflicts. Client data alone cannot prove a server behavior; exact historical internals are unnecessary when observable equivalence can be established. |
| What has the owner selected? | Baseline match, explicit deviation, deferred content or unresolved version choice. Preserve original prior decisions and their dispositions. |
| What is implemented here? | Owning code/content, observed tests, and actual client availability. Test-only data and rendered props are identified as such. |
| How will correspondence be proved? | Observable cases and expected outcomes; numeric claims need suitable evidence. Missing source remains unknown, never invented parity. |

Research is private and separate from the checkout. Maintained authored specs
record the resulting rule and a neutral evidence identity/digest. They must be
usable without private archives. Raw sources and private narrative records are
neither test fixtures nor runtime fallback inputs.

## Immediate recovery order

The [class and training contract](class-training-contract.md) owns the recovered
class/teacher matrix, focus rules and explicit numerical reconciliation gaps.
The [first-expedition plan](plans/2026-09-06-first-expedition.md) tracks their
implementation through the persistent client.

1. Recover the previous research and decision records into a durable private
   workspace, preserving originals and digests; index missing referenced sources.
2. Reconcile classes, promotion and training tracks, then assign the actual
   teaching services to town locations. There is no implied one-trainer-per-class
   rule; four generic visual stations do not establish a class roster.
3. Inventory surviving implementations against the recovered gameplay baseline.
   Carry forward useful verified work and name behavioral differences explicitly.
4. Finish the living surface, reconstruct the corresponding dead town and map
   temple access to the initial four dungeon levels. Recover connections as well
   as tiles. The three deeper levels are a later content tranche.

The [continuity audit](plans/2026-09-06-design-continuity-audit.md) records the
initial recovery and the trainer-planning error. Its earlier proposed amendment
is superseded by this owner direction. This ruling dispatches research recovery
and design reconciliation; it does not claim completion of the game or promote
the current town candidate as accepted content.
