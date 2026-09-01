---
last_updated: 2026-08-20
revision: 2
status: Authored at genesis plan Phase 7, executing owner rulings D5 and D2; pending owner acceptance at the phase stop point.
public_safe: true
summary: Who owns which fact class — the standing seams verified against this tree, the fact classes death-as-play requires and the predecessor's map never contemplated, the authoritative pulse and its ruled value, and the line AI may not cross.
always: true
---

# The boundary map

## What this document is

This map answers one question per entry: **who alone may change this fact?**

Every entry is a seam, and a seam is four lines:

- **Owner** — the module, crate, or table that holds the fact.
- **Rule** — *X alone may mutate Y.*
- **Never** — the things that are specifically not allowed to happen, named,
  because a boundary stated only in the positive is a boundary nobody can fail.
- **Proof** — where a violation would be caught.

It is an **ownership map, not a description of the implementation.** The
predecessor's equivalent document had grown to 953 lines and had become a second,
prose copy of the code: it re-narrated module internals, so it drifted every time
a module moved and nobody could tell which half was true. Here a seam names the
module that owns the fact and the test that proves it. The module is the
description.

Entries come in two kinds:

- **Standing** (Part 1) — implemented. Every one was verified against this tree
  while this document was written, and each cites the file that holds it.
- **Authored** (Part 2) — this project's central fact classes, which have an
  owner named here **before** the code exists. Death as continued play, lineage,
  succession, departure, and world memory are the game; a fact class with no
  named owner is one that drifts into three places and is then owned by nobody.

## What an authored seam does and does not settle

An authored seam names the owner, the fact classes, and the invariants that hold
whatever the mechanics turn out to be. It settles no mechanic.

Owner ruling **D2** reopened, for fresh successor design, **all** exact
mechanics, names, timings, penalties, and routes. Nothing carries forward as
settled. Each authored seam below therefore ends with an explicit **Reopened**
list, and that list is binding: an agent that finds a value in an authored seam
has found a defect in this document, not a decision.

The one ruled value in this map is the pulse: **3.0 seconds**, ruling D5,
recorded at [The authoritative pulse](#21-the-authoritative-pulse-d5) and
implemented as `GAMEPLAY_PULSE` in `crates/tme-server/src/scheduler.rs`.

## Where this sits

The charter's decision order governs: explicit owner decisions, then the charter
and later owner-approved product direction, then architecture and content
ownership records, then implementation and tests, then plans and history, then
agent proposals. This document is an ownership record — the third rung. An owner
ruling overrides it; it overrides an implementation that disagrees with it, and
the disagreement is a defect to be reported rather than a fact to be adopted.

Related owners, none of which this document duplicates:

| Document | Owns |
| --- | --- |
| [Agent workflow](agent-workflow.md) | how work is scoped, authored, proven, and closed out |
| [Public boundary policy](public-boundary-policy.md) | external reference material, provenance, and the public cut |
| [Boundary checks](boundary-checks.md) | the five fail-closed public-boundary checks and their mutant qualification |
| [Working-root policy](working-root-policy.md) | the ignored working root — what may live there, what may never depend on it, retention, and the promotion path (owner ruling D6) |
| [Authoring contracts](authoring-contracts.md) | the contract-first authoring standard and its principles |
| [Authoring compiler](authoring-compiler.md) | how an authored document becomes proven runtime content |
| [Server notes](server-notes.md) | the one-world decision, credentials, persistence, external-boundary policy |
| [Client architecture](client-architecture.md) | the client's state domains, codec, reconciliation, and proof |
| [Client notes](client-notes.md) | the client decisions actually implemented |
| [Presentation direction](presentation-direction.md) | the visual target |
| [Workbench V0](workbench-v0.md) | the owner-agent spatial reference tool |
| [Settled conclusions](settled-conclusions.md) | what is closed and should not be re-litigated |

## The boundaries

| Boundary | Where | Owns |
| --- | --- | --- |
| Rules | `crates/tme-rules` | gameplay truth: legality, resolution, timing semantics, life state, projection |
| Content | `content/`, validated by `crates/tme-rules/src/content` | the authored facts the rules consume |
| Server runtime | `crates/tme-server` | sessions, admission, wall-clock scheduling, durable authority, the wire |
| Client | `web/`, `client/` | input, presentation, accessibility, discardable local state |
| Authoring | `crates/tme-authoring`, `content/lands/` | authored documents to proven runtime content |
| Simulation | `crates/tme-sim` | fast deterministic gameplay proving over the same rules boundary |
| Tools | `tools/` | boundary checks, the Workbench, proof harnesses |
| AI layer | not implemented | bounded judgment, timing, and voice — never truth ([Part 3](#part-3-ai-is-never-game-authority)) |

---

# Part 1: The standing seams

## 1.1 The authored/runtime contract seam

**Owner.** Four contracts, one owner per kind of fact:

| Contract | Owns | Defined in |
| --- | --- | --- |
| Catalog | immutable gameplay definitions: terrain, actors, items, spells, loot, spawn groups, lairs, services, quests, and the rules profile | `crates/tme-rules/src/content/catalog.rs` |
| World template | immutable realms, levels, layered cells, arrivals, topology, law zones | `crates/tme-rules/src/content/world_template.rs` |
| World seed | authored initial *mutable* state: actor instances, item placement, service instances, merchant stock | `crates/tme-rules/src/content/world_seed.rs` |
| Simulation scenario | harness-owned orchestration: graph references, profile, RNG seed, typed script | `crates/tme-sim/src/fixture/mod.rs` (`SimulationScenarioV1`) |
| Land contract | which lands exist, which members each carries, their envelopes, vocabularies, programs, receipts, and outputs | `crates/tme-authoring/src/contract/` |
| Served world | which catalog, profile, compiled template, and seed make one land's world | `content/lands/<land>/world.json`, composed into a bootstrap manifest by whoever runs a server ([server notes](server-notes.md#which-world-the-one-process-serves)) |

**Rule.** Each fact is authored in exactly one of the four. A fact that is
immutable belongs in the catalog or the template; a fact that starts somewhere
and then changes belongs in the seed.

**Never.** No contract restates another's fact. No runtime type is the authoring
format — the runtime never loads the editor's document; the compiler produces
what the runtime reads ([authoring compiler](authoring-compiler.md)).

**Proof.** `crates/tme-rules/tests/content_validation.rs`, including
`four_contract_decoders_are_strict_and_core_documents_reject_scripts`.

## 1.2 Compile once, then it is immutable

**Owner.** `GameDefinition` (`crates/tme-rules/src/engine/definition.rs`)
compiles the selected catalog and template exactly once. `ValidatedWorldSeed`
(`crates/tme-rules/src/engine/setup/mod.rs`) binds checked initial state to that
exact definition. `World` (`crates/tme-rules/src/model.rs`) contains mutable
gameplay state and nothing else.

**Rule.** After compilation, definition data is read-only for the life of the
world.

**Never.** No engine module mutates a definition, caches a second compiled copy,
or reconstructs a definition fact from mutable state.

**Proof.** The type split itself: the compiled definition is not reachable
mutably from the engine, and the workspace suite is the standing check.

## 1.3 Rust is the sole gameplay-semantic validator, and it fails closed

**Owner.** `crates/tme-rules/src/content/validation/`.

**Rule.** Content is validated in Rust before it can reach the engine. A
validation that cannot run does not pass — it fails with a diagnostic naming the
cause.

**Never.** No tool, script, or authoring path may declare content valid on the
rules boundary's behalf. No validator degrades to a warning when its input is
missing.

**Proof.** `crates/tme-rules/tests/content_validation.rs` and the negative
fixtures beside it. The load-time boundary-term check
(`crates/tme-rules/src/content/validation/boundary/terms.rs`) is the
fail-closed pattern in its sharpest form: a missing, unreadable, or empty term
file fails every clean-content validation rather than passing quietly. See
[boundary checks](boundary-checks.md).

## 1.4 Deterministic logical time

**Owner.** `crates/tme-rules/src/model/timing.rs` defines `LogicalTime` —
deliberately independent of wall-clock seconds — and `ActionCost` in whole
rounds. `crates/tme-rules/src/engine/timing.rs` is the only place `World.timing.now`
advances, in `advance_realtime_boundary_transaction`.

**Rule.** Each living actor holds an absolute next-ready time and a stable order.
An actor-addressed committed intent schedules that actor, resolves ready
automatic actors in stable order, and completes the logical boundary before
returning the next controlled opportunity. Read-only intents are free and advance
none of it.

**Never.** No boundary outside the rules crate advances gameplay time. No
subsystem derives a gameplay decision from wall-clock elapsed time. Presentation
may interpolate between beats; it may not tick.

**Proof.** `crates/tme-rules/tests/` timing coverage; the client asserts
readiness comes from the frame's `logical_time` / `ready_at` / `can_act` and
never from wall time (`client/tests/test_pointer_movement.gd`).

The wall-clock cadence at which those boundaries are struck is a separate fact
with a separate owner and a ruled value: [2.1](#21-the-authoritative-pulse-d5).

## 1.5 Fail-closed terrain composition

**Owner.** `crates/tme-rules/src/engine/navigation.rs` (`terrain_at`).

**Rule.** A cell is a stack of nullable terrain layers, and composition resolves
in exactly one place: any unresolved or blocked layer blocks; otherwise swim
outranks walk; move cost is the maximum layer cost; and any sight-blocking or
unresolved layer blocks sight. The same owner resolves walking, swimming, doors,
stairs, pits, climbs, passages, and portals.

**Never.** No caller re-derives walkability, cost, or sight from raw layers.
Presentation must never imply a walkability it did not receive.

**Proof.** `crates/tme-rules/tests/` navigation coverage; the client's reach grid
is drawn only from authoritative squares (`client/tests/test_grid_world_view.gd`).

## 1.6 Visibility is one observer policy

**Owner.** `crates/tme-rules/src/engine/visibility.rs`;
`PLAYER_OBSERVATION_RADIUS = 7`.

**Rule.** One centred radius defines what a controlled player observes, and the
same set gates sight-dependent targeting. Geometry-only queries stay
observer-independent.

**Never.** No second visibility implementation, no client-side sight derivation,
no observed projection that leaks a fact outside the observed set.

**Proof.** `crates/tme-rules/tests/` visibility coverage; the observer projection
tests below.

## 1.7 One item, one location, one transaction

**Owner.** `crates/tme-rules/src/engine/inventory.rs`; the location type is
`ItemLocation` in `crates/tme-rules/src/model/items.rs`.

**Rule.** Every item instance has exactly one registry row and exactly one
location — ground, carried, corpse, merchant, locker, or offered. Relocation is
one atomic validated commit across all of them.

**Never.** No caller copies an item to represent a move, reconstructs a location,
keeps a second collection, or bypasses the reservation an offer holds. Corpse
contents are real locations, not a copy of an inventory.

**Proof.** `crates/tme-rules/tests/` inventory coverage; the rollback case in
`service_transactions::late_reward_failure_rolls_back_costs_and_all_world_state`.

## 1.8 The shared transaction planner

**Owner.** `crates/tme-rules/src/engine/transactions.rs`, with the authored shape
validated by `crates/tme-rules/src/content/validation/transactions.rs`.

**Rule.** Requirements are evaluated in authored order; every cost and reward is
captured and preflighted **without mutation**; costs are coordinated before
rewards; and a failure at any point leaves no partial world change. Domain owners
keep their own fact authority — the planner coordinates them, it does not write
their facts.

**Never.** No provider adds an arbitrary predicate or effect callback, a parallel
common mutation path, or a second receipt ledger.

**Proof.** `crates/tme-rules/tests/service_transactions.rs`, in particular
`late_reward_failure_rolls_back_costs_and_all_world_state`.

This seam is load-bearing for [succession](#24-succession) — an inheritance is
exactly the transaction class that must never half-commit.

## 1.9 Automatic actors decide; they do not resolve

**Owner.** `crates/tme-rules/src/engine/ai/` for the decision;
`crates/tme-rules/src/model/ai.rs` for actor-local state;
`crates/tme-rules/src/engine/social.rs` as the sole hostility assessor.

**Rule.** An automatic actor may select one assessed target and commit one typed
decision per ready opportunity. Everything that decision *does* runs through the
same timing, movement, weapons, combat, damage, reward, spell, and social owners a
player's command uses.

**Never.** No AI-owned readiness, movement legality, damage, reward, spell
effect, hostility matrix, or second RNG. No engine-global awareness or threat
table. No simulator-only actor implementation.

**Proof.** `crates/tme-rules/tests/` automatic-actor coverage.

## 1.10 One lifecycle owner for creature ecology

**Owner.** `crates/tme-rules/src/engine/ecology.rs`.

**Rule.** Ecology alone observes vacancy, schedules a positive logical-time
reset, defers materialisation while a site is observed, and recreates slots in
stable order through the shared actor constructor and the single RNG.
Materialisation is capped per site per logical boundary
(`MAX_SITE_MATERIALIZATIONS_PER_BOUNDARY`).

**Never.** Ecology is not an AI, clock, movement, combat, death, corpse, reward,
inventory, or scheduler authority, and holds no second RNG. It retains prior
corpses and ground state rather than clearing them.

**Proof.** `crates/tme-rules/tests/` ecology coverage.

## 1.11 One mutation boundary per actor-state family

**Owner.** One module per family, and only that module writes it:

| Fact | Owner |
| --- | --- |
| live HP / MP / stamina, burden tier, exertion, cadence recovery | `crates/tme-rules/src/engine/resources.rs` |
| banked XP, level application, growth rolls | `crates/tme-rules/src/engine/progression.rs` |
| skill ledger entries, learning rate, critique | `crates/tme-rules/src/engine/skills.rs` |
| the read-only training plan and its commit | `crates/tme-rules/src/engine/training.rs` |
| item knowledge and enchantment state | `crates/tme-rules/src/engine/items.rs` |
| class promotion | `crates/tme-rules/src/engine/promotion.rs` |
| per-character quest ledger (`World.quest_states`) | `crates/tme-rules/src/engine/quests.rs` |

**Rule.** Other domains compute their own consequences and then commit them
through the owning seam.

**Never.** No mirror, no second ledger, no recovery counter, no pending-state
cache, no previewing of a random growth result, and no presentation-side copy of
any of it.

**Proof.** `crates/tme-rules/tests/` per-domain coverage; the workspace suite is
the standing check.

## 1.12 Social identity, law, and karma

**Owner.** `crates/tme-rules/src/engine/social.rs` is the sole assessor and
mutator of hostility, apparent identity, attack safety, alignment consequence,
and karma. The account-wide historical record is
`tme.player_kill_marks` in `crates/tme-server/migrations/`.

**Rule.** Rules assess; the durable store records. The server supplies injected
time, identity mapping, transaction order, and idempotency. **Rules alone change
karma or alignment.**

**Never.** The durable store never parses events or edits checkpoint state to
reach a gameplay conclusion. No caller stores a disguise flag or a detector
switch — apparent identity is derived.

**Proof.** `crates/tme-rules/src/engine/social/tests.rs`;
`crates/tme-server/tests/postgres_persistence.rs` (PostgreSQL-gated).

This split — *typed rules assessment in, durable account-scoped record out* — is
the precedent the [lineage](#23-lineage) seam builds on.

## 1.13 Death and corpse state

**Owner.** `crates/tme-rules/src/engine/damage.rs` is the sole lethal trigger and
passes typed cause and credit to `crates/tme-rules/src/engine/death.rs`. That
owner alone changes `ActorLifeState`
(`crates/tme-rules/src/model/death.rs`), allocates corpse and gold identities,
validates same-square corpse search, and applies a caller-supplied resurrection.

**Rule.** Life state is one enum with one writer. Corpse contents are real item
locations owned by inventory; death asks inventory to relocate them.

**Never.** No second `alive` flag anywhere in the tree. No boundary infers death
from prose, from an event it happened to see, or from a missing actor. **No owner
outside this one may invent a recovery policy.**

**Proof.** `crates/tme-rules/tests/` death and resurrection coverage; the client
carries life state as a projected fact only (`client/adapter/wire_codec.gd`).

What this seam does **not** yet contain is the game: today a non-`Alive` actor
cannot act at all — `crates/tme-rules/src/engine/timing.rs` rejects every intent
from one with `cannot step after actor death`, and `Ghost` is a bookkeeping
state, not a place. Reopening that is [2.2](#22-death-as-continued-play).

## 1.14 The observer / debug projection split

**Owner.** `crates/tme-rules/src/engine/view/snapshots.rs` — `snapshot()` is the
omniscient diagnostic view; `actor_observed_snapshot()` and
`actor_observed_frame()` are the observer-specific views.
`crates/tme-rules/src/engine/action_context/` owns the consumer projectors.

**Rule.** Two projections, deliberately different. The debug snapshot may carry
everything. The observed projection carries only what that observer may know:
derived attack safety without raw alignment or karma, the controlled character's
own quest log and offers and no one else's, no foreign inventory.

**Never.** The observed path must not be built by filtering the debug snapshot at
the call site, and no consumer may parse names or labels, read authored service
internals, run a second eligibility check, or mutate state during discovery.

**Proof.** `crates/tme-rules/tests/` projection coverage; the wire fixture corpus
carries privacy cases (`tests/fixtures/wire/`).

## 1.15 One rules-side read-model facade

**Owner.** `crates/tme-rules/src/view/mod.rs`; every contract version lives in
`crates/tme-rules/src/view/contract_versions.rs`.

**Rule.** Private domain modules own their payloads; the facade is the only
public surface. A contract version is a single constant in a single file.

**Never.** No consumer imports a private view child module as an alternative
contract. No superseded contract shape retains a reader or an adapter — see the
no-compatibility-adapters rule in [agent workflow](agent-workflow.md).

**Proof.** the workspace suite; `client/tests/test_wire_codec.gd`
(`test_protocol_constants_are_pinned`).

## 1.16 Strict wire codec discipline

**Owner.** `crates/tme-protocol` is the wire-schema authority
(`PROTOCOL_MAJOR` / `PROTOCOL_MINOR`, `CONTROL_API_VERSION`).
`client/adapter/wire_codec.gd` and `client/adapter/strict_json.gd` are a
**verified mirror**, never a second schema authority.

**Rule.** Decoding rejects unknown fields and variants, wrong types, missing
required fields, invalid required-nullable distinctions, out-of-range or
non-finite values, non-canonical UUIDs, and non-canonical decimal strings. Wide
signed quantities cross the wire as canonical strings and never touch a float.
Both sides are proven against **one shared fixture corpus**, `tests/fixtures/wire/`,
read rather than copied — two copies of a contract drift, and the drift shows up
as a passing test on each side.

**Never.** No fallback parser, ignored-field mode, alias, or previous-version
path exists before the external boundary activates.

**Proof.** `crates/tme-protocol/src/client_fixture_tests.rs` and
`crates/tme-protocol/tests/protocol_v1.rs` (the boundary corpus at 2^53−1, 2^53,
2^53+1, and `i64::MAX`); `client/tests/test_wire_codec.gd` and
`client/tests/test_strict_json.gd` assert the same inventory and verdicts.
Details in [client architecture](client-architecture.md).

## 1.17 The server owns wall-clock reality, and only that

**Owner.** `crates/tme-server`.

**Rule.** The server owns authentication, sessions, admission, connection
lifecycle, wall-clock scheduling, durable persistence, recovery, and the
exhaustive rules-to-wire conversion. It calls into the rules boundary for every
gameplay decision. One process serves one world instance; one database holds one
world row, enforced by a singleton unique index.

**Never.** The server does not compute a gameplay outcome, reorder authoritative
rules events, or hold a second copy of world state outside the checkpoint. It
never edits gameplay through ad hoc SQL.

**Proof.** `crates/tme-server/tests/`, including the PostgreSQL-gated durability
suite. The one-world decision, its enforcement, and the line a scaling change may
not cross are owned by [server notes](server-notes.md).

## 1.18 The client owns nothing authoritative

**Owner.** `client/`.

**Rule.** The client owns input, presentation, audiovisual pacing, accessibility,
application lifecycle, strict wire consumption, and discardable local state. The
latest complete authoritative frame is replaced atomically; events never patch
it.

**Never.** The client imports no rules type, infers no legality, advances no
gameplay time, and keeps no gameplay ledger. Presentation may explain an
authoritative event; it may not patch game truth.

**Proof.** [Client architecture](client-architecture.md) holds the contract and
its five proof layers; [client notes](client-notes.md) holds what is implemented.

## 1.19 Simulation exercises the same boundary; it is not a second game

**Owner.** `crates/tme-sim`.

**Rule.** The harness loads content, seeds a world, drives typed intents, and
emits deterministic traces through the same rules boundary the server uses.

**Never.** No simulator-only gameplay mutation, no parallel execution loop, no
second rules implementation in any language. Behaviour that works only in the
harness is a design defect, not a harness feature.

**Proof.** `crates/tme-sim/tests/` and the golden trace corpus.

---

# Part 2: The authored fact classes

These are the seams the game is made of. The predecessor's map contains none of
them: it declared death-time attrition and ancestoring *absent runtime domains*,
forbade the death owner from inventing a recovery policy, and had no lineage,
succession, aging, or ancestry owner anywhere in roughly forty seams. This
project does not extend that map. It authors the fact classes that map never
contemplated, and reopens the seam it deliberately locked.

Each entry names an owner and the invariants that survive whatever the mechanics
become. **No entry settles a mechanic.**

## 2.1 The authoritative pulse (D5)

The charter makes one authoritative pulse a product invariant. A product
invariant with a ruled value and no named owner is still a fact class waiting to
drift, so this is where it is named.

**The ruling (D5, owner, 2026-08-19).**

- **One authoritative gameplay pulse begins at 3.0 seconds.**
- Player readiness, automatic actors, spell preparation, recovery, and
  pulse-owned environmental processes **derive from that clock**.
- Networking, animation, rendering, interpolation, telemetry, and server
  housekeeping may run more frequently but **may not become a second gameplay
  clock**.
- **Presentation may remain fluid between authoritative beats.**
- A later cadence change requires an **explicit owner ruling backed by a
  side-by-side play-feel test**. It may not drift through implementation or
  prose.
- **The one-second statement is not product authority.**

**Owner — the beat and the striking of it are two facts.**

| Fact | Owner |
| --- | --- |
| what one beat *means* — one logical round, and everything scheduled in rounds | `crates/tme-rules/src/engine/timing.rs` (`advance_realtime_boundary`), over `LogicalTime` in `crates/tme-rules/src/model/timing.rs` |
| *when* a beat is struck — the wall-clock cadence | `crates/tme-server/src/scheduler.rs`, the sole wall-clock source, which sends one boundary request into the single world-instance mailbox. The value is the constant `GAMEPLAY_PULSE` there, and nowhere else. |
| applying a struck beat exactly once | `crates/tme-server/src/facet.rs` (`advance_facet_tick`) |

**Rule.** The cadence is one value in one place. Everything timed derives from
the beat by counting rounds, never by reading a clock of its own.

**Never.** No second scheduler, timer, or background task may submit a gameplay
boundary. No subsystem may reach for wall time to decide a gameplay outcome. No
presentation layer may infer readiness from elapsed seconds — readiness arrives
in the frame. The cadence may not be changed by editing a constant: it changes by
owner ruling with a play-feel test, and this document changes with it.

**Proof.** `crates/tme-server/src/scheduler.rs` unit tests hold the cadence
contract: one asserts `GAMEPLAY_PULSE` is the ruled 3.0 seconds, the other that
the scheduler strikes exactly one boundary per elapsed pulse and none for an
elapsed second. End to end, `tools/run_client_live_proof.py` drives a real
client against a real server and judges the beats it observes — one round of
logical time per pulse of wall time — against the ruled value it names in its
own right, so the proof cannot pass by agreeing with the constant it tests. That
verdict is itself held by `tests/test_live_proof_pulse.py`, which feeds it
recorded observations — including a one-second cadence — and checks what it
refuses. The client's frame-only readiness is proven in
`client/tests/test_pointer_movement.gd`.

The client now also **presents** the beat, under the fluid-between-beats
permission above and nothing wider. It is never told the cadence — nothing on
the wire carries one — so it measures the interval between two authoritative
rounds and interpolates inside it, while readiness stays the frame's `can_act`.
`client/tests/test_pulse_clock.gd` holds the refusals that keep that from
becoming a second clock: no fill before an interval has been observed, no
extrapolation past one, no readiness from elapsed time, and no measurement of an
interval that skipped a round. `tools/run_pulse_capture.py` photographs a real
client's meter advancing inside one round and judges the beat that client
measured for itself against this ruling's value — two independent statements of
one fact agreeing, rather than a constant compared with itself. The presentation
contract is [client notes](client-notes.md#the-pulse-made-visible).

## 2.2 Death as continued play

**Owner.** The rules boundary — `crates/tme-rules/src/engine/death.rs`, extended.
Life state stays one enum with one writer
(`ActorLifeState`, `crates/tme-rules/src/model/death.rs`).

**The fact classes this seam owns.**

1. **Life state.** Which of the coherent states a character is in, and every
   transition between them. Today the enum is `Alive`, `Ghost`,
   `AwaitingResurrection`, `Dead`; the successor's set of states is part of the
   reopened design, but there is exactly one enum and one writer whatever it
   becomes.
2. **The dead world as a place.** Geography, inhabitants, and reachability while
   dead are **world state**, authored through the same four contracts and
   resolved by the same navigation and visibility owners as the living world.
   Living and dead geography correspond; the differences are authored.
3. **Return to embodiment.** Every route back, its preconditions, and its
   consequences — one authoritative transition, applied by this owner.
4. **The body.** Corpse identity, its contents as real item locations, who may
   act on it, and what recovering it means socially.

**Invariants, whatever the values become.**

- **One writer.** No second `alive` flag, no parallel life-state cache, no
  presentation-side inference. A boundary that needs to know whether a character
  is dead reads the projected fact.
- **Death changes where a character acts. It never changes who adjudicates.**
  A dead character's actions are ordinary typed intents through the ordinary
  command path, resolved by the ordinary owners, on the same pulse.
- **No automatic recovery.** Return is an achieved, authored transition. Nothing
  outside this owner may invent a recovery policy, and no timer, disconnect,
  reconnect, or convenience path may perform one.
- **The dead are not omniscient.** Whatever the dead perceive is an observer
  projection with its own rules, produced by the observer path in
  [1.14](#114-the-observer--debug-projection-split) — not the debug snapshot, and
  not a widened radius applied at a call site.
- **Stable identity across every state.** One character's identity survives body,
  corpse, ghost, ancestor, successor, and lineage record. Identity is never
  re-derived from a display name.
- **Corpses are locations, not copies** ([1.7](#17-one-item-one-location-one-transaction)).

**Reopened (D2).** Entry conditions for each state; any attrition, penalty, or
loss and its ladder; corpse persistence, decay, and recovery timing; which routes
return a character and what each costs; what the dead perceive, where they may
go, and what they may do; whether and how the living and dead communicate; and
every in-world name for any of it. The implementation language in this document —
*dead state*, *return*, *succession*, *departure* — is provisional and
source-neutral, exactly as the charter says.

**Owed proof.** The seam is not built. When it is, the tests that prove it are
named here, and the first of them is the one that fails today: an intent from a
non-`Alive` actor is currently rejected outright by
`crates/tme-rules/src/engine/timing.rs`.

## 2.3 Lineage

**Owner — split, deliberately.**

| Fact | Owner |
| --- | --- |
| lineage transitions and every gameplay consequence of them | the rules boundary |
| the durable lineage record and its identity | the server's durable store, `crates/tme-server/src/store/` and the schema in `crates/tme-server/migrations/` |

**Rule.** *Account-level continuity belongs to the lineage; character-level
identity belongs to each life* (charter §2.2). The schema already carries that
shape: `tme.accounts` is the account, and `tme.characters` holds one row per life
bound to it. A lineage record is account-scoped durable state and belongs beside
them.

The precedent for the split is standing and proven:
[1.12](#112-social-identity-law-and-karma). The durable store consumes a **typed
rules assessment** and records it; the server supplies identity mapping,
transaction order, and idempotency; **rules alone** change the gameplay fact.
Lineage follows the same shape.

**Invariants.**

- **One durable record per lineage, in one place.** Whether a given lineage fact
  lives in the account-scoped tables or in the world checkpoint is a design
  decision this seam does not make — but it lives in exactly one of them, and the
  other side holds no copy. The distinction matters concretely: the world
  checkpoint can be restored to an earlier point, and account-scoped rows are not
  restored with it.
- **The rules boundary never reads the account tables directly.** It receives
  typed facts and returns typed assessments, exactly as the karma path does.
- **No lineage state is client-authoritative.** The client displays what it is
  sent.
- **Nothing is inferred from names.** Not display names, not slots, not ordering.

**Reopened (D2).** What a lineage is called; what it accumulates and what it
grants; whether it is visible to other players and at what resolution; how many
lives it may hold at once — the present eight-slot character model is a slot
model, not a lineage model, and it carries no successor authority; and whether
lineage facts ever affect balance.

**Owed proof.** Not built. The durability question above is the first thing a
lineage slice must answer with a test, not with prose.

## 2.4 Succession

**Owner.** The rules boundary owns the inheritance transaction, and it is **one
transaction**, planned and committed through
[1.8](#18-the-shared-transaction-planner).

**Rule.** Succession is the transaction class that must never half-commit. An
ancestor made with no successor seated, or a successor seated holding an
inheritance the ancestor never gave up, is an unrecoverable state — there is no
compensating action a player or an operator can take. So it inherits the
planner's discipline exactly: requirements evaluated in authored order, every
cost and reward captured and preflighted **without mutation**, costs coordinated
before rewards, and any failure leaving no partial change.

Where it also writes durable account-scoped state, that write commits **in the
same database transaction** as the gameplay commit. The pattern is already in the
tree and already ruled: a deferred player-kill consequence is written in the same
transaction as the mark it belongs to, and exactly-once falls out of the
atomicity rather than being defended by a status column
([server notes](server-notes.md)).

**What it may read.** The ancestor's own character-scoped state, through the
owners that hold it; the lineage record, as a typed fact; the authored content
that defines the rite.

**Never.**

- It may not copy an item. An inherited object is a **relocation** through
  [1.7](#17-one-item-one-location-one-transaction), so the instance stays
  singular and its history stays intact.
- It may not write skills, experience, resources, or alignment directly. It
  delegates to the owners in [1.11](#111-one-mutation-boundary-per-actor-state-family).
- It may not read another account's state.
- It may not be reachable by accident. Succession is explicit and distinct from
  an accidental death at the **type** level, not by a flag on a shared event — so
  no code path can turn one into the other by mistake.
- It may not reduce the ancestor to a stat package: the ancestor persists as a
  person in the world ([2.6](#26-ancestry-and-world-memory)).

**Reopened (D2).** What is inherited and how much; any alignment, karma, or
level limits; whether the rite requires a place, a service, or a witness; its
cost; any cooldown; and every name for it.

**Owed proof.** Not built. The atomicity claim is provable the day the seam
exists, against the pattern in
`service_transactions::late_reward_failure_rolls_back_costs_and_all_world_state`.

## 2.5 Aging and departure

**Owner.** The rules boundary owns the transition; the durable store records that
it happened.

**Rule — the one-way invariant.** Leaving the land restores ordinary linear age
and final mortality. It is **one-way**: after it completes, no route returns that
character to play. Because it is irreversible, it is also **explicit**:

- exactly one authored route reaches it;
- it cannot be reached by timeout, inactivity, disconnection, session expiry,
  account closure, or any operator convenience path;
- it is distinct from ordinary death at the type level, so no failure mode
  upgrades a death into a departure;
- and it is the most consequential transition in the game, so it is confirmed
  deliberately rather than triggered by a single input.

**Aging.** If age is a gameplay fact, it has one owner and one advance site, on
the same discipline logical time already has
([1.4](#14-deterministic-logical-time)): one counter, advanced in one place,
derived from the pulse. No second age counter anywhere, and no client-side aging.

**Never.** No irreversible transition without an authored route to it. No
reversal by operator edit — an irreversible transition that operators quietly
undo is not irreversible, it is undocumented.

**Reopened (D2).** Whether age advances inside the land at all, and at what rate;
what departure costs and what it grants; whether there is a point of no return
before it completes and where that point sits; and its name.

**Owed proof.** Not built.

## 2.6 Ancestry and world memory

**Owner.** The rules boundary owns the memory ledger.

**Rule.** *Former characters remain people in the world rather than becoming
deleted save records* (charter §2.2). That is a fact class: **event-keyed world
memory**, keyed by stable identity and typed event, mutated in exactly one place.

The shape already exists in the tree and is the model to follow.
`World.quest_states` is keyed by stable `CharacterId`, quest id, and typed stage
id; only `crates/tme-rules/src/engine/quests.rs` may read or mutate it; a missing
entry means unstarted; and every commit rechecks the state it captured. World
memory is the same discipline over a different key.

**Never — and this one is named because it is the tempting shortcut.**

- **No affinity meter is authority.** A scalar reputation, affection, or standing
  number is a summary, not a memory: it cannot be authored against, cannot be
  audited after the fact, and quietly becomes the real mechanic. A presentation
  layer may *display* a derived summary; nothing may branch on one as the
  authoritative fact. If the project ever wants a meter to be authority, that is
  a separate owner ruling, and this line is what it has to overturn.
- No boundary outside the owner keeps a memory flag, and no NPC infers memory
  from carried items, names, or dialogue text.
- A hidden memory stays hidden in the observed projection. The debug snapshot may
  carry the whole ledger; the observed context carries only what that observer
  may know ([1.14](#114-the-observer--debug-projection-split)).
- Nothing is inferred from prose. Dialogue is never a hidden eligibility
  protocol.

**Reopened (D2).** Which events are remembered; for how long, and whether memory
decays; who perceives a memory and how it surfaces; how the living and dead
differ in what they remember; and whether memory ever affects balance.

**Owed proof.** Not built.

---

# Part 3: AI is never game authority

```text
AI may add judgment, timing, voice, and preparation.
The engine owns truth.
```

This is a boundary, not a roadmap. Nothing here is approved for implementation.

**Rule.** AI may assist with concepts, dialogue drafts, visual candidates,
criticism, encounter preparation, and other bounded creative work. Runtime truth,
legality, identity, rewards, persistence, and irreversible transitions remain
deterministic system responsibilities.

**Never.**

- An AI layer is an **intent source, never a state mutator.** Any AI-originated
  action enters as a typed intent through the same path as any other actor's, and
  may not mutate an engine, world, checkpoint, receipt, or store directly.
- It may not decide whether an attack hits, whether a move is legal, whether an
  item exists, whether a spell succeeds, or whether a player may act.
- It may not author authoritative content. It selects, times, and parameterises
  reviewed human-authored content.
- No quest, mechanic, safety-critical warning, or required world fact may depend
  on it. The world stays fully playable and fair when the layer is absent,
  degraded, slow, or over budget.
- Any wall-clock AI trigger enters through the same single boundary path as
  everything else. It may not become a second gameplay clock
  ([2.1](#21-the-authoritative-pulse-d5)).
- Generated text is subject to the same expression boundary as any other external
  material: a model's knowledge of other games is not a clean content source. See
  [public boundary policy](public-boundary-policy.md).
- Accepted content is reviewed and committed as tracked content before it reaches
  a player.

**Not yet.** No LLM runtime, agent service, protocol, persistence model,
privileged read model, or AI-controlled character behaviour is added until an
approved slice defines the exact boundary and its verification.

---

# Part 4: The external boundary is not active

No externally distributed client, real persistent player data, released save or
content format, public API consumer, or deployed service interface exists. Until
an explicit project decision records one, only the exact current contract is
supported and internal contracts are replaced atomically with all their callers.

The policy that takes effect **when** that boundary activates — version domains,
support windows, migration, deployment order, and rollback — is owned by
[server notes](server-notes.md#the-external-boundary-when-it-activates). It is
recorded there rather than here because it is operational policy about the
server's own external surfaces, and this map names owners and invariants rather
than procedures.

The default rule before activation is **no compatibility adapters**, and it is
owned by [agent workflow](agent-workflow.md).

---

# Part 5: What this map does not decide

- **Any reopened mechanic.** D2 reopened every exact mechanic, name, timing,
  penalty, and route. Part 2 names owners and invariants; it chooses no values.
- **The visual target.** [Presentation direction](presentation-direction.md).
- **Content.** What lands, settlements, creatures, or services exist is authored
  content, bounded by [authoring contracts](authoring-contracts.md).
- **Process.** How work is scoped, proven, and closed out is
  [agent workflow](agent-workflow.md).
- **Anything an owner has not ruled.** A gap in this map is a gap, and the honest
  response is to name it — not to fill it with the most plausible-sounding
  answer.
