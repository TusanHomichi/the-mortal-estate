---
last_updated: 2026-09-10
revision: 11
status: Historical gameplay baseline with four-floor reconstruction, Martial Artist combat-sequence direction, remaining traversal gaps and a hunting/safe-area timing proposal.
public_safe: true
summary: Historical gameplay target, local-door movement, Martial Artist jump-kick entry and fist attacks, fidelity accounting and remaining proposals.
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

**September 9 continuation:** the owner directs sustained implementation toward
class and gameplay correspondence, using the restored private research library,
its underlying archive and live sources as needed. The installed contemporary
reference client is authorized for comparative observation. Bind those
observations to the actual client/version and conditions; similarity does not
silently turn a modern observation into an original-release fact. Existing
source conflicts and numerical gaps still require their stated evidence or an
explicit rule selection. Bounded implementation and supervised delegated work
continue without repeatedly requesting permission for routine decisions.

**Dungeon continuation:** the owner directs all four selected dungeon floors,
omitting the fourth-floor locker-room cutout because storage already has a town
location. The [execution plan](plans/2026-09-09-dungeon-one.md) owns source selection,
encoding and proof. Remaining fidelity work includes concealed-door discovery,
cavern access under current corner rules, water/pit traversal, environmental hazards, darkness, garden access and reserved
surface or external portal destinations. Their geography is retained; current traversal does
not establish historical behavior.

The owner also directs a separate concealed or peripheral thieves-guild entrance,
with a dungeon route that avoids the main streets and their sheriff. The recovered revision-1.11 client help independently supports hostility toward
neutral thieves and disguise exceptions for knights and advanced thieves. The
recovered thief field guide explicitly confirms sheriff disguise detection and
two secret dungeon exits from the guild. The sheriff’s crossbow response still
needs corroboration; none of this response is implemented yet. The [class contract](class-training-contract.md#thief-guild-continuation--september-9)
owns the guild trainer, skill and spell venue. Detection,
AI hostility and combat belong to rules. Preserve the basement route and reserved
surface stair while recovering the applicable evidence. The exact town building
or hidden-door position remains unauthored.

## Local-door movement

**Owner clarification, September 10:** entering a closed local dungeon door opens
it and ends that movement on the doorway tile, including when the requested
three-cell sprint had remaining steps. An already-open local door allows the
remaining steps to continue, subject to the existing movement budget, terrain
and other authoritative restrictions. Opening does not queue the unused steps
for a later action. This is an explicit owner-selected gameplay rule; no new
historical source claim is implied.

The shared rules evaluator owns both preview and commit. Browser proposals may
target a closed doorway but may continue through a local doorway only when its
observed state is open. Paired transitions between distinct endpoints retain
their existing behavior; this clarification concerns local corridor doors.

## Martial Artist combat sequence

**Owner clarification, September 10:** a Martial Artist can open combat with a
jump kick that closes distance to the target, travelling up to three tiles, and
lands the kick within one round. Approach and attack belong to that single
action; do not require a separate movement action before the kick. Once engaged
in combat, the selected attack sequence uses punches, with varied fist-attack
animations. The jump kick is an available opener, not a compulsory entry into
every fight.

This is an explicit owner-selected future gameplay requirement, informed by
the owner's historical account. It is not an independently verified release
comparison or a claim that the current rules/client implement the sequence.
The [class contract](class-training-contract.md#class-and-teacher-relationships)
owns class capabilities and teaching relationships.

The animation work must support takeoff, airborne kick, impact, landing and a
transition into the punching guard across the supported approach distances.
Animation does not grant movement, decide hit outcomes or introduce extra
attacks. Rules own travel legality, the landing position, attack resolution and
action deadlines; the client presents their authoritative result. Exact impact
placement within the action and obstacle/target-change cases require the later
combat slice's specification and proof. The owner's one-round requirement does
not restore a shared pulse or settle a new duration in seconds; the current
[individual-deadline ruling](boundary-map.md#21-authoritative-individual-deadlines-d5)
continues to own timing architecture.

## Hunting and safe-area proposal

The September 9 owner discussion proposes a tactical, turn-based presentation
for hunting/combat areas and possibly more real-time movement outside combat or
in safe areas. The fixed 7-by-7 dungeon view and preference for the 60-degree
art study are concrete presentation directions, owned by
[presentation direction](presentation-direction.md#projection-and-surface-ruling).
The area classification and timing split remain a proposal. No change to action
durations, NPC deadlines, server sight range or the current
[D5 ruling](boundary-map.md#21-authoritative-individual-deadlines-d5) is implied.
A later gameplay slice must settle which areas qualify, what “turn based” means
with multiple players and automatic actors, and how crossing area boundaries
preserves pending actions and deadlines. This comparison changes only rendering.

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
