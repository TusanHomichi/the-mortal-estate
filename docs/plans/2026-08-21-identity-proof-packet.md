---
last_updated: 2026-08-27
revision: 4
status: Owner-ruled Phase 10 packet. Section 13 records the 2026-08-21 rulings; S1 and S8 are implemented; the remaining S3-first order is bracketed by the presentation experiment, and the clean public-source ruling clarifies that its public-snapshot debt binds G11 release rather than source publication.
public_safe: true
routes:
  - crates/tme-rules/src/engine/death.rs
  - crates/tme-rules/src/model/death.rs
summary: The ruled design packet for the charter's first identity proof — its shape, reopened values, ten-slice plan, 2026-08-21 owner rulings, implemented S1/S8 evidence, and the S3-first fallback order now bracketed by the live genesis ledger.
---

# The identity proof — design packet

## §0 How to read this

This document was written for the owner to rule on **before** Phase 10 code.
Section 13 records the 2026-08-21 ruling: every recommendation was accepted,
with R5 amended to 20 beats / 60 seconds. S1 and S8 later landed. Proposal labels
remain in the earlier sections to preserve what was presented for judgment; they
must now be read with the section 13 disposition.

**Every claim carries its original label.** `decided` meant the owner had already
ruled it or the charter stated it as an invariant. `proposed` marks what this
packet recommended before section 13 ruled it; the label is retained as history,
not as a present claim that the ruling is absent. `open` means the packet declined
to guess and section 13 did not supply a value.

**D2 was applied.** Every exact mechanic, name, timing, penalty, and route was
reopened while this packet was authored. Its starting values came from original
design work argued from this project's own contracts — never imported. The pulse
was already ruled at **3.0 seconds** (D5); section 13 later ruled the packet's
recommendations, with its recorded R5 amendment.

**No names are asserted.** `settlement`, `beneath`, `threshold_keeper` and
`devourer` are **placeholder role labels**. They claim nothing about the
settlement's name, the dead world's name, or the vocabulary for death, return,
succession, ancestors, and departure — all left open by charter §15.

**The engine audit.** Each item states what exists with a `file:line`, what is
missing, and which [boundary map](../boundary-map.md) owner it belongs to. The
audit read the code, not the documentation.

### §0.1 The four owner rulings of 2026-08-21 — DECIDED

1. **The dead world here is the old burned town, beneath** — the living
   settlement's own footprint, one layer down.
2. **Death keeps its shape: ghost first.** Self-visible ghost, corpse others can
   carry, and **only being eaten forces the descent**. Not simplified.
3. **Succession pays a tied weapon and a bounded share of skill and
   experience.** The numbers are reopened — propose, do not settle.
4. **Item 9's visual edit waits for a real first editable asset.** The synthetic
   swatch is not an accepted target. The **map** half is not gated.

### §0.2 What the charter already fixes — DECIDED

Death never becomes a loading screen; the player is a lineage; one authoritative
pulse; the world remembers (charter §2). Return is achieved, never automatic.
Succession is explicit and distinct from accidental death **at the type level**.
Identity survives body, corpse, ghost, ancestor, successor, and lineage record
([boundary map §2.2–§2.4](../boundary-map.md#22-death-as-continued-play)).

### §0.3 Two defects this audit found

Both are reported here with exact evidence rather than left in a conversation.

1. **`AwaitingResurrection` is terminal.** `engine/death.rs:337` emits
   `ResurrectionRequested` with `ResurrectionMethod::Gods` when a player dies
   with no corpse, and **nothing in the tree ever applies it**: the only
   production callers of `apply_resurrection_request` are
   `engine/restoration.rs:331` and `engine/spellcasting/effects.rs:793`, and both
   require a corpse. A player who dies to fire today is stuck forever. Owner:
   rules, [§2.2](../boundary-map.md#22-death-as-continued-play). Slice **S4**
   closes it as part of the eaten route; file it now so it is not closed by
   accident.
2. **A ghost is visible to everyone.** `engine/death.rs:320` leaves the defeated
   actor in `World.actors` at its square, and the observed projection
   (`engine/view/snapshots.rs:410`) filters by the one observation radius
   (`engine/visibility.rs:8`) with no life-state rule — while ruling 2 requires a
   *self-visible* ghost. Owner: rules, §2.2 and
   [§1.14](../boundary-map.md#114-the-observer--debug-projection-split). Closed
   by **S3**.

---

## §1 The proof's shape

### The world — one settlement, one layer beneath

`decided` (ruling 1). One living settlement area stands over the burned town
beneath it, same footprint, one layer down.

`proposed` — **the layer beneath is a third authored member of the same land, in
the same realm.** The engine already resolves movement, sight, transitions, and
arrivals across levels within one realm (`content/world_template.rs:19-45`,
`engine/navigation.rs`), and the runtime world is exactly that: one realm, six
levels (`content/test-corpus/world_templates/first_land_structure.json`).

| Option | Consequence |
| --- | --- |
| third member, same realm (**recommended**) | one arrival table, one topology graph, correspondence is a coordinate identity |
| second realm | duplicate arrivals and topology per realm; every cross-layer query grows a realm term |
| one level with a "layer" flag on cells | forks fail-closed terrain composition ([§1.5](../boundary-map.md#15-fail-closed-terrain-composition)); rejected |

`proposed` — **the two layers share one envelope, cell for cell**, so a
coordinate means the same place in both and correspondence is testable rather
than asserted. Starting size **48 × 32**: large enough for a settlement, a
service, a route out, and a ruin mouth; small enough that one person can author
both layers and a rescuer can cross it inside a release window.

`proposed` — **the two layers must disagree** in at least **20%** of cells, by
terrain, structure, or passability. A dead layer that is a recolour of the living
one proves nothing about the world remembering (charter §5).

### The minimum cast — `proposed`, four roles and no more

| Role | Placeholder id | Why it is in the minimum |
| --- | --- | --- |
| the player's first life | `player` | item 2, and the lineage's first member |
| the hostile | `devourer` | item 2's enemy **and** the only way to be eaten (ruling 2) |
| the service relationship | `threshold_keeper` | item 2's service **and** item 6's route back — one NPC, two proof items |
| the resident below | `the_unreleased` | item 5's inhabitant, item 8's neighbour |

Collapsing the service and the return route into one NPC is deliberate: the
charter asks for *one* service relationship, and a keeper who takes your body
back from the dead is a stronger relationship than a shopkeeper.

### The minimum geography — `proposed`

- **`settlement`** (48 × 32, overworld, town law): the keeper, the descent
  marker, the route out.
- **`beneath`** (48 × 32, same envelope): the burned town, holding
  `the_unreleased` and, after a succession, the ancestor.
- **the dangerous route** is a corridor of `settlement`, not a third level — the
  settled cells out to a ruin mouth where `devourer` lairs. Danger is authored by
  distance from the keeper, not by a level boundary.

`open` — **whether a living character may descend voluntarily.** The proof does
not need it; the dead reach `beneath` by being eaten. A voluntary door for the
living is a real feature with its own cost, and belongs in its own ruling
(**R11**).

---

## §2 Item 1 — a living settlement area above a corresponding dead-world layer

**What it proves.** The world remembers, and the two states of one geography are
authored rather than reskinned (charter §2.4, §5).

**What exists.**

- Multi-realm, multi-level worlds with typed transitions:
  `crates/tme-rules/src/content/world_template.rs:13-45` (`WorldTemplateV3`,
  `RealmDef`, `LevelDef`) and `:126-146` (`TopologyKindDef` — door, stairs, pit,
  climb, passage, portal).
- A runtime world already using six levels of one realm and eleven non-door
  transitions: `content/test-corpus/world_templates/first_land_structure.json`,
  loaded at `crates/tme-server/src/production.rs:285-291` and asserted loadable
  as the canonical world by `tracked_first_land_loads_as_the_one_canonical_world`.
- An authoring compiler that emits exactly that runtime shape:
  `content/authoring-fixture/generated/world_template.json` is a valid
  `WorldTemplateV3` with two levels and a reciprocal stair pair.

**What is missing.**

- A land whose members include a **correspondent layer beneath**. The compiler's
  contract fixes the member set at two (`MemberRole::{Surface, Interior}`,
  `crates/tme-authoring/src/contract.rs:51-62`) and the transition program at
  `contract.rs:142`. Owner: the authoring compiler
  ([authoring-compiler.md](../authoring-compiler.md)) and content.
- **The runtime does not load authoring output.** `content/authoring-fixture/promotion.json`
  records `runtime_loads_authoring_source: false`; the served world is the
  hand-authored corpus template. Until that changes a Workbench edit cannot reach
  play — item 9's whole point. Owner: content, and the server's bootstrap config.

**Proposal.** `proposed` — the proof's land is **authoring-compiled**: the owner
edits it in the Workbench, the compiler emits the world template, the server
loads that file. This is slice **S1**, and it is why item 9 is achievable at all.

**Reopened values.** Envelope 48 × 32; divergence floor 20%; whether `beneath`
carries its own law zone (`proposed`: yes, lawless — the dead layer is not
policed by the living settlement's rules).

**Proof.** A test asserting every `settlement` cell has a `beneath` cell at the
same coordinate and that the two disagree by at least the floor; the compiler's
connectivity and reciprocity assertions extended to the third member; a capture
of each layer from the same coordinates.

---

## §3 Item 2 — one player, one enemy, one service relationship, one dangerous route

**What it proves.** The loop has a subject, a threat, a dependence, and a place
that punishes carelessness (charter §4).

**What exists.**

- A seeded player actor, five NPCs with typed interactions, and one service
  instance (a bank) in
  `content/test-corpus/simulation_seeds/first_land_structure.json`.
- Ecology: 38 sites across five levels, 19 member locations on the surface,
  driven by `crates/tme-rules/src/engine/ecology.rs`, with spawn groups and reset
  policies in the catalog.
- A restoration service **definition** carrying a `priest_resurrection` outcome
  (`content/test-corpus/catalogs/prototype_catalog_v6.json`), and the wire intent
  to use it (`crates/tme-protocol/src/lib.rs:2974-2982`,
  `Intent::UseRestorationService`, with `corpse_id`).

**What is missing.** A restoration service **instance** in the proof world's seed
— the seed instances only the bank — and a hostile whose kill consumes the body
(item 4). Owner: content, validated by
[§1.1](../boundary-map.md#11-the-authoredruntime-contract-seam).

**Proposal.** `proposed` — the cast in §1, with `devourer` seeded as a lair on
the route, one instance, on the ecology owner's existing slot-reset policy.

**Reopened values.** `proposed`: the `devourer` kills a starting character in
**four to six** exchanges — long enough that death is a decision the player
watches arrive, short enough that the route is respected. The keeper charges
**nothing** in the proof; pricing the return is separate (**R6**).

**Proof.** A simulation trace: the player walks the route, is engaged, dies, and
the keeper's service appears in the observed action set.

---

## §4 Item 3 — the authoritative pulse visible through movement, combat, preparation, feedback

**What it proves.** One clock, felt (charter §2.3).

**What exists — most of it.**

- The cadence in one place: `crates/tme-server/src/scheduler.rs:21`
  (`GAMEPLAY_PULSE = 3000 ms`), with a test at `:60-64` asserting the ruled value
  so a silent edit fails.
- Logical time advanced in exactly one place,
  `crates/tme-rules/src/engine/timing.rs`
  ([§1.4](../boundary-map.md#14-deterministic-logical-time)).
- Readiness read from the frame, never from wall time:
  `client/scenes/world_shell_screen.gd:896-898` renders
  `◆ Ready · world T… · ready T…` / `◇ Waiting`.
- Presentation pacing: `client/presentation/combat_feel_director.gd` holds
  feedback to a minimum payoff and schedules chant completion.
- End-to-end judgment: `tools/run_client_live_proof.py` counts beats against the
  ruled value it names itself.

**What is missing.** The pulse is **legible as text and inaudible as rhythm.**
Nothing in the view expresses the beat as motion. Owner: the client
([client architecture](../client-architecture.md)); nothing about the cadence is
reopened.

**Proposal.** `proposed` — four presentation expressions, all derived from the
frame's `logical_time` / `ready_at`, none of them a second clock
([§2.1 Never](../boundary-map.md#21-the-authoritative-pulse-d5)):

1. a **beat meter** filling between the frame's current time and `ready_at`,
   interpolated locally and re-anchored on every frame;
2. **movement takes the beat** — a step animates across the interval rather than
   snapping, so a three-square path reads as three beats;
3. **preparation shows its band** — a warming spell shows beats remaining and
   lands on one;
4. **feedback resolves inside its beat** — the existing payoff window tied to the
   interval rather than a millisecond constant.

`open` — audio. A beat you can hear is the cheapest way to feel a pulse and the
easiest to make annoying. Recommend deferring past the proof.

**Proof.** Three captures at known offsets within one beat showing the meter
advance; the live proof's beat judgment; a client test that the meter's fill
comes from the frame and freezes when frames stop.

---

## §5 Item 4 — character death without loss of control or play

**What it proves.** The product's central claim. This is the item the whole proof
is for.

**What exists.**

- One life-state enum, one writer: `crates/tme-rules/src/model/death.rs:99-110`
  (`Alive`, `Ghost { corpse_id, defeated_at }`, `AwaitingResurrection`, `Dead`),
  written only by `crates/tme-rules/src/engine/death.rs`
  ([§1.13](../boundary-map.md#113-death-and-corpse-state)).
- Death already produces the right *shape*: a corpse with a real identity, the
  dead character's items relocated into it as real item locations
  (`engine/death.rs:78-131`), hands' contents dropped, gold piled, and a `Ghost`
  state naming its own corpse (`engine/death.rs:307-320`).
- Life state reaches the client as a projected fact:
  `crates/tme-server/src/protocol_v1.rs:266`, `client/adapter/wire_codec.gd:347`.

**What is missing — the game.**

- **A ghost cannot act at all.** `crates/tme-rules/src/engine/timing.rs:111-112`
  rejects every intent from a non-`Alive` actor: `cannot step after actor death`.
  Owner: rules, [§2.2](../boundary-map.md#22-death-as-continued-play).
- **The ghost is public** (defect §0.3.2).
- **No corpse carry.** A corpse is a location
  ([§1.7](../boundary-map.md#17-one-item-one-location-one-transaction)), not a
  thing a character can hold: no intent, no burden, no carrier.
- **No release**, no death cry, no bearing for out-of-view speech. Social scopes
  exist on the wire (`lib.rs:3182-3187`) but carry no direction.

**Proposal.** `proposed` — **ghost-first, control never lost.**

- On death with a corpse the character becomes a ghost **on the square it died
  on, on the living layer**, and keeps control — same pulse, same command path,
  same owners ([§2.2](../boundary-map.md#22-death-as-continued-play)).
- **What a ghost may do:** move, and speak. Nothing else.

  | Option | Consequence |
  | --- | --- |
  | move + speak (**recommended**) | smallest surface that makes death playable; no new legality rules for items, doors, or attacks |
  | move + speak + pass through blocked cells | forks terrain composition per life state; a real design, not a proof minimum |
  | full non-combat action set | every service, item, and interaction owner needs a life-state rule in the same slice |

- **A ghost is visible only to its own observer** — a life-state rule in the
  observed projection, never a widened radius at a call site. Item 8 reuses it.
- **The corpse is what everyone else sees**, and it can be **carried**: both
  hands, no attacking or casting, stamina per beat of movement. The cost is what
  makes recovery a favour rather than a chore.
- **Release** is always available to the ghost, and can never be gated on a
  carrier's consent.

**Reopened values**, each with a starting value and why:

| Value | `proposed` | Why this number |
| --- | --- | --- |
| release window while lying on the death square | **20 beats (60 s)** — owner-ruled 2026-08-21 (packet proposed 30) | one legible count at the ruled cadence; the owner judged 90 s too long for a ghost that can only move and speak |
| the window while carried | **stops outright**, not paused | a carrier can hold a body indefinitely; manual release is the escape, and that tension is the social texture |
| stamina per beat of carrying | **3× the cost of an unburdened step** | heavy enough to plan for, light enough to attempt alone |
| ghost speech range | the observation radius, **7** | reuses `engine/visibility.rs:8` rather than inventing a second range |
| bearing for out-of-radius speech | **eight compass points, no distance** | the imprecision is the gameplay; the rescuer has to search |

`open` — the death cry as an audible event, and whether a ghost's speech is
audible to the living at all versus only to other dead. Recommend: audible to the
living, because otherwise nobody comes.

**Proof.** A simulation trace in which a defeated player issues a move intent and
it is *accepted*; a rules test that the same intent from a second observer's
frame does not show the ghost; a live play session in which one account dies and
walks.

---

## §6 Item 5 — an active dead-world objective with geography and inhabitants

**What it proves.** The dead world is a place with business, not a waiting room
(charter §2.1).

**What exists.**

- A per-character quest ledger keyed by stable identity and typed stage, one
  writer: `World.quest_states`, `crates/tme-rules/src/engine/quests.rs`
  ([§1.11](../boundary-map.md#111-one-mutation-boundary-per-actor-state-family)),
  with two staged, terminal-marked quests already authored in the catalog.
- NPC interaction outcomes on the wire, `crates/tme-protocol/src/lib.rs:2983-2988`.

**What is missing.** Any actor on the dead layer; any obligation reachable only
while dead; the dead's observer projection (item 4's mechanism). Owner: content
and rules §2.2.

**Proposal.** `proposed` — **one resident below, one two-stage obligation, and it
is the eaten character's way home.**

- `the_unreleased` stands on `beneath` at the ruin's counterpart cell.
- It is completed by **reaching a place and speaking**, not by carrying an
  object — which keeps item locations out of the dead layer and leaves "can the
  dead hold things" a real design question rather than an accident of slice one.
- It is authored as **one instance of a family** with a fixed learnable identity,
  so the pool can grow without the first one becoming procedural filler.

**Reopened values.** Pool size (`proposed`: one, for the proof); whether the
obligation is drawn or assigned (`proposed`: assigned, because a draw needs a
pool); whether the dead may carry items (`open`, and deliberately so).

**Proof.** A trace — eaten, arrives below, speaks to the resident, stage
advances, the return becomes available — plus a test that a living character
cannot start it.

---

## §7 Item 6 — at least one social or authored route back to embodied life

**What it proves.** Return is achieved (charter §3).

**What exists — more than expected.**

- A whole priest-resurrection service path:
  `crates/tme-rules/src/engine/restoration.rs:140-211` validates the selection —
  an exact corpse, a *player* corpse, and **the corpse at the service** — and
  `:331-332` applies it and reschedules the actor.
- `engine/death.rs:472-551` validates a resurrection request (matching corpse,
  walkable destination, per-method resource bounds) and `:553+` applies it:
  contents relocated back, gold moved, corpse removed, life state `Alive`.
- A second route exists as a spell, `engine/spellcasting/effects.rs:792-802`.
- The catalog carries the service definition; the wire carries the intent.

**What is missing.**

- **Nobody can bring a corpse to the keeper** — no carry (item 4). Today the only
  reachable return is dying on the service's square.
- **The no-corpse branch is terminal** (defect §0.3.1).
- No service instance in the proof world's seed.

**Proposal.** `proposed` — **three routes, one ladder**, and the ladder is what
makes a death worth avoiding without making it a punishment:

| Route | How it happens | `proposed` cost |
| --- | --- | --- |
| **carried** | someone hauls your corpse to the keeper | your possessions return with you (already implemented); no attrition |
| **released** | you release, or the window runs out | you arrive at the keeper's square empty-handed; your belongings stay on the ground where you fell; **one skill track loses its practice points toward its current level** |
| **from below** | you were eaten; you complete the obligation | no attrition — the expedition *is* the cost |

Two things are deliberate. The heavy penalty is **recoverable by playing**, not
by waiting, so a bad death is a project rather than a timer. And belongings
become ordinary ground state at release — rescue is a winnable race, and the
thing that killed you may be wearing your gear when you come back.

**Reopened values.** Which track loses (`proposed`: the highest-level one — it
costs the most and reads the clearest); whether the keeper charges (`proposed`:
no, in the proof); resurrection hit points and stamina, which the engine already
fixes per method at `engine/death.rs:490-512` — those constants are **carried
from a port and are therefore reopened**, and S4 should re-derive rather than
inherit them.

**Proof.** Three traces, one per route, each ending in an `Alive` actor holding
the expected state; the rollback discipline test extended so a failed return
leaves no partial change.

---

## §8 Item 7 — a deliberate succession in which a new character inherits from the old

**What it proves.** The player is a lineage (charter §2.2).

**What exists — primitives, no seam.**

- **The tied weapon already has a type:** `ItemBindingState::Bound { character_id }`,
  `crates/tme-rules/src/model/items.rs:448-452`.
- **The transaction discipline exists and is proven:**
  `crates/tme-rules/src/engine/transactions.rs`, with the rollback case
  `service_transactions::late_reward_failure_rolls_back_costs_and_all_world_state`
  ([§1.8](../boundary-map.md#18-the-shared-transaction-planner)).
- **The state to share is typed:** `SkillEntry { track_id, level, critique_rank,
  practice_points, learning_rate }` (`crates/tme-rules/src/model/character.rs:79-86`)
  and banked experience (`character.rs:49`), each with one mutation owner
  ([§1.11](../boundary-map.md#111-one-mutation-boundary-per-actor-state-family)).
- The durable side already splits the way lineage needs: `tme.accounts` and
  `tme.characters` with `UNIQUE (account_id, slot)`,
  `crates/tme-server/migrations/202608190001_initial_persistence.sql:47-57`.

**What is missing.**

- **No lineage record.** Nothing account-scoped ties two lives together. Owner:
  the durable store, [§2.3](../boundary-map.md#23-lineage).
- **No succession transaction, no ancestor state.** Owner: rules,
  [§2.4](../boundary-map.md#24-succession).
- **Characters cannot be created at runtime.** `crates/tme-server/src/postgres.rs:2351`
  inserts `tme.characters` **once, at first world bootstrap**, from a config file
  (`crates/tme-server/src/production.rs:245-256`); there is no create or delete
  on the wire (`CharacterSummaryV1` is read-only,
  `crates/tme-protocol/src/lib.rs:658-663`). A successor cannot be seated today.
  Owner: the server.

**Proposal.** `proposed` — **what is inherited**, two things, because ruling 3
says two things:

1. **Every item bound to the ancestor**, moved by **relocation, never a copy**,
   its binding rewritten to the successor's `CharacterId`, so the instance stays
   singular ([§1.7](../boundary-map.md#17-one-item-one-location-one-transaction)).
2. **A bounded share of skill and experience.**

`proposed` — **the bound**, offered as three options:

| Option | Shape | Consequence |
| --- | --- | --- |
| flat fraction | successor gets *k*% of banked experience and of each track's practice points | simple; lets a high ancestor mint a near-peer successor |
| **capped fraction (recommended)** | *k*% of each, **and** no track may arrive above `floor(ancestor_level / 2)` | the relief is real and the ceiling keeps a successor a beginner in the ways that matter |
| whole tracks to a budget | the player chooses tracks to carry whole, up to a point budget | the most expressive; the most design and UI, and it invites min-maxing |

`proposed` starting values: **25%** of banked experience, **25%** of each track's
practice points, ceiling `floor(ancestor_level / 2)`. The argument for 25%:
succession must cut the cost of starting over without becoming the efficient way
to level, and a quarter of a long life is a real head start that still leaves the
successor's own life to be the substance of it.

`proposed` — **how the successor is seated.** Build the real path: the succession
transaction inserts the successor's `tme.characters` row **in the same database
transaction as the gameplay commit**, exactly as the deferred player-kill
consequence already does ([server notes](../server-notes.md)). Seeding a latent
spare actor in the world file is quicker and is a scaffold that cannot survive
the proof; not recommended.

`proposed` — **the rite is explicit and typed** — a distinct intent and a
distinct transaction class, not a flag on a death event, so no failure mode can
turn an accidental death into a permanent one
([§2.4 Never](../boundary-map.md#24-succession)).

**Reopened values.** Where the rite happens (`proposed`: at the keeper — the same
relationship, a third time); whether it requires the ancestor to be alive
(`proposed`: yes, you must *choose* it embodied); alignment or karma limits on
what may be inherited (`open`); any cooldown (`open`).

**Proof.** The atomicity claim against the existing rollback pattern: a
succession whose final reward fails leaves no ancestor, no successor, no
relocated item. Plus a gated database test that the successor's row and the
gameplay commit land together or not at all.

---

## §9 Item 8 — the successor later encountering the ancestor in the dead world

**What it proves.** Former characters remain people, not deleted save records
(charter §2.2).

**What exists.** Nothing. The nearest model is the quest ledger's discipline —
keyed by stable identity and typed stage, one reader and writer — which
[§2.6](../boundary-map.md#26-ancestry-and-world-memory) names as the shape world
memory must follow.

**What is missing.** The ancestor as an inhabitant; the memory ledger; the
per-observer rule that decides who may see whom. Owner: rules,
[§2.6](../boundary-map.md#26-ancestry-and-world-memory) for the ledger and
[§2.2](../boundary-map.md#22-death-as-continued-play) for the dead layer as a
place.

**Proposal.**

`proposed` — **what the ancestor is.** A persistent inhabitant of `beneath`,
standing at the counterpart of the cell where they gave up embodiment. Not an
actor with a schedule and not a quest giver with its own ledger: an entry in the
world-memory ledger, projected as an inhabitant to observers entitled to see it.

`proposed` — **who may see it.** Only its own lineage — the **same per-observer
rule item 4 needs** for the self-visible ghost. One mechanism, two uses, and an
argument for building it once in S3.

`proposed` — **what the meeting does.** It surfaces **one authored typed memory**
of the ancestor's life and tells the successor something otherwise unlearnable:
the ancestor names the obligation below. The encounter is a **knowledge route**,
and knowledge is a progression system (charter §5).

`decided` by invariant — **what it can never do.** Give items, experience, skill,
or resources. Alter any of the successor's state. Act on the world or enter as an
intent source. Be perceived outside the lineage. Be summarised into an affinity
or standing number that anything branches on —
[§2.6](../boundary-map.md#26-ancestry-and-world-memory) forbids that by name.

**Reopened values.** Whether the ancestor changes over time (`proposed`: no —
the dead do not age, and the contrast is the design); whether other lineages
perceive an anonymous presence (`proposed`: no); how many memories one ancestor
holds (`proposed`: one, authored as one of a family).

**Proof.** A test that two observers of the same square receive different frames,
and that the ledger entry is absent from an unentitled observer's projection
while present in the debug snapshot
([§1.14](../boundary-map.md#114-the-observer--debug-projection-split)).

---

## §10 Item 9 — the Workbench used to identify, stage, preview, and apply one map edit and one visual edit

The G0 ruling places this at Workbench **V1**, which shipped
([workbench-v1.md](../workbench-v1.md)).

### The map edit — ready today, on one condition

**What exists.** The complete V1 loop — point, stage, preview, Apply — with six
truth verbs, each carrying a proven rejection (`workbench-v1.md`, the verb
table). Apply re-verifies every bound digest, replays the staged set through the
compiler's own semantics, and is atomic by rename.

`proposed` — **the exact edit, on the proof's land**, once **S1** has made that
land an authoring-compiled land: `point` at the ruin mouth on the `settlement`
member; stage `move_landmark { landmark_id: <the ruin marker>, to: <cell> }`,
which moves where the dangerous route ends; stage
`set_terrain { cells: <the corridor>, class: <a blocking base class> }`, which
narrows it; `preview`; `apply`.

The same pair is provable on the existing fixture land **today**, with no new
code, as a rehearsal: `move_landmark` on `fixture_ruin_marker` and
`set_transition_endpoint` on `fixture_descent`
(`content/authoring-fixture/fixture-surface.tmj`).

**Two facts to schedule around.** A Workbench edit **does not reach play until S1
lands** — the served world is a hand-authored corpus file and
`promotion.json` records `runtime_loads_authoring_source: false`. And a truth
edit **invalidates the running world**:
`crates/tme-rules/src/engine/checkpoint.rs:125-126` rejects a checkpoint whose
content identity does not match the loaded definition, so applying the map edit
restarts the proof world from an empty database. That is correct behaviour
pre-external-boundary and fine for the proof — but plan the demonstration around
it rather than discovering it during one.

`open` — **whether the compiler gains a verb that can change `target_member` or
`paired_transition`.** `workbench-v1.md` honest gap 1 records that the
single-member candidate path's soundness argument depends on no such verb
existing, and a third member makes one tempting. Recommend: not in this proof.

### The visual edit — gated

`decided` (ruling 4). The visual half waits for a real first editable asset. The
synthetic swatch is not the target, and `workbench-v1.md` honest gap 3 already
says so in its own words: *"`fixture-swatch.png` is a synthetic editable master,
not an accepted visual master, and its provenance record says so. State 3 for a
real asset waits for an owner."*

**What unblocks it:** one **owner-accepted editable master** produced from an
accepted reference micro-scene at actual play size — the same gate the whole art
programme sits behind, the production rule in
[presentation direction](../presentation-direction.md#the-production-rule).

So item 9's visual half is **not blocked on tooling**; the tooling is built and
proven. It is blocked on an owner acceptance the charter deliberately placed
outside implementation. **R10** asks when that acceptance happens.

---

## §11 Item 10 — a clean build and verification path requiring no private predecessor data

**Already proven. This section records; it does not redesign.**

`tools/run_clean_clone_proof.py` copies the carried set into a scratch directory,
asserts none of the roots `.gitignore` declares came with it, and runs the
`portable` lane inside it. It is a standing CI job — `.github/workflows/verify.yml`,
the `cleanclone` job, *"a clean copy builds and tests with no private root"* —
and `tests/test_ci_workflow.py` resolves what each job names against the step
table and asserts their union is `full` exactly, with no step paid for twice, so
the proof cannot quietly stop running.

**The only obligation Phase 10 carries:** every file the proof adds must be
**carried**, every `research_boundary.review_refs` entry in new content must
resolve (`tools/check_review_refs.py`), and nothing may depend on an ignored root
([working-root policy](../working-root-policy.md)).

---

## §12 Slice plan

Ten items, ten implementation slices. Item 9 is deliberately split between the
map edit (S9) and visual edit (S10); item 10 needs no separate slice because its
clean-build obligation applies to every slice. Each slice is small, has one
owner and one proof, and names the rulings it waits on. **S1 is buildable the
moment the owner rules R1 and R2.**

| # | Slice | Owner | Proves | Waits on |
| --- | --- | --- | --- | --- |
| **S1** | The proof's land is authoring-compiled and the server loads it. One member, no new mechanics: author `settlement`, compile it, point the bootstrap at the emitted template, seed the cast's living half. | authoring compiler, content, server config | item 2's cast is reachable; item 9's map edit can reach play | R1, R2 |
| **S2** | The layer beneath: a third authored member, same envelope, with the correspondence test and the divergence floor. | authoring compiler, content | item 1 | R1, R3 |
| **S3** | **Death keeps control.** Lift the non-`Alive` intent rejection; ghost may move and speak; the observed projection gains its life-state rule (which item 8 reuses); release. | rules §2.2, and the client for presentation | item 4 | R4, R5 |
| **S4** | The return ladder: corpse carry with its burden, the keeper's service instance, the eaten cause, and the ladder's attrition. Closes defect §0.3.1. | rules §2.2, content | item 6, and the cost half of item 4 | R5, R6 |
| **S5** | The obligation below: `the_unreleased`, a two-stage quest, the eaten route's return. | rules (quests), content | item 5 | R7 |
| **S6** | **Lineage and succession**: the durable lineage record, the succession transaction, binding rewritten by relocation, the bounded share, and the runtime successor seat. | rules §2.3/§2.4, the durable store, the server | item 7 | R8, R9 |
| **S7** | The ancestor below: the world-memory ledger, the ancestor as a lineage-visible inhabitant, one authored memory. | rules §2.6 | item 8 | R9 |
| **S8** | The pulse made visible: beat meter, movement across the beat, preparation band, feedback inside its beat. | client | item 3 | — (nothing reopened) |
| **S9** | The map edit, on the proof's land, staged and applied through V1, with the world restart planned for. | the owner, at the Workbench | item 9a | S1 |
| **S10** | The visual edit. | the owner | item 9b | **gated** — R10 |

Item 10 needs no slice; it needs discipline in every slice above.

**Owner ruling, 2026-08-21 (scope and order).** The remainder of the genesis
plan is executed as follows, and the project pauses before content production
resumes:

1. S1 and S8 land first (both in flight; S1's geography is on a shape pass at
   the owner's review).
2. Then S3 → S2 → (S4, S5, S7 in parallel) → S6 → S9.
3. Engineering debt that must not cross into the G11 external release is slotted in
   as its owning code is touched: successor #2 (files over 1,000 lines), #4
   (rename "facet"), #6 (`authorize_grant` swallows errors), #11 (capture
   harness `--import`), #15 (gated PostgreSQL under host load), #16 (production
   content registry), #17 (deployment composes the drill world), #18
   (accessibility sweep).
4. **Parked until the owner restarts content production:** S10 (the visual
   edit), the reference micro-scene (R10), and successor #1 (the pixel-target
   re-cut). Nothing in the slices above may depend on them.
5. After S9: the owner rules G10, completes the trademark clearance (charter
   §15), and Phase 11 is one reviewed snapshot.

**Ordering.** S8 depends on nothing and runs in parallel with anything. S3
precedes S4, S5, and S7 — all three consume the per-observer projection rule it
builds. S6 is the largest and should not start while S3 is unsettled:
succession's atomicity argument reads the character state death's rewrite
touches.

**Present-order addendum, 2026-08-26.** The
[genesis ledger](genesis-ledger.md) is the live owner of resumption and order. It
temporarily brackets the S3-first fallback above with the bounded target and
Nomos presentation experiment in
[`2026-08-26-nomos-presentation-adoption-experiment.md`](2026-08-26-nomos-presentation-adoption-experiment.md).
The bracket does not complete, cancel, or silently reorder any Phase 10 slice.

---

## §13 Rulings requested — RULED 2026-08-21

**Owner ruling, 2026-08-21:** every recommendation below is accepted as written,
with one amendment — **R5 is 20 beats (60 s), not 30 beats (90 s).** R12 was
executed the same day (successor #12; the ghost-visibility defect is #13).
The numbered items are kept verbatim as the record of what was asked.

Short, because this is read on a phone. Each has a recommendation.

1. **R1 — Is the proof's land authoring-compiled, with the server loading the
   compiler's output?** *Recommend yes.* Without it the map edit never reaches
   play and the Workbench is proven against a file nobody walks on.
2. **R2 — Is the layer beneath a third member of the same land, same realm?**
   *Recommend yes*, over a second realm.
3. **R3 — Envelope 48 × 32, identical for both layers, at least 20% of cells
   differing?** *Recommend yes* — one coordinate space is what makes
   correspondence testable.
4. **R4 — May a ghost only move and speak?** *Recommend yes* for the proof;
   items, doors, and attacks stay refused.
5. **R5 — Release window of 30 beats (90 s) on the death square, stopping
   outright when carried, manual release always available?** *Recommend yes.*
   **Ruled: yes, at 20 beats (60 s).**
6. **R6 — Attrition ladder: nothing for a carried return, one skill track's
   practice points for a release, nothing for the route from below?** *Recommend
   yes* — and re-derive the resurrection resource constants a port left at
   `engine/death.rs:490-512` rather than inheriting them.
7. **R7 — Do the dead carry items?** *Recommend no* for the proof; the
   obligation is completed by reaching a place and speaking.
8. **R8 — Inheritance bound of 25% of banked experience and of each track's
   practice points, capped at `floor(ancestor_level / 2)`?** *Recommend yes*, of
   the three options in §8.
9. **R9 — Is the successor seated by a real runtime character-creation path?**
   *Recommend yes* — the alternative is a scaffold that cannot outlive the proof.
10. **R10 — Does the accepted reference micro-scene happen inside Phase 10 or
    before it?** *Recommend before*, as its own owner pass: item 9's visual half
    is blocked on it, and so is every later art decision.
11. **R11 — May a living character descend voluntarily?** *Recommend not in this
    proof.* It is a real feature and deserves its own design.
12. **R12 — File defect §0.3.1 (`AwaitingResurrection` is terminal) as an issue
    now, ahead of S4?** *Recommend yes*, so closing it stays deliberate.

---

## §14 Limits preserved after the ruling

- **No mechanic beyond section 13.** The 2026-08-21 ruling accepted the
  recommendations with the recorded R5 amendment. Implementation may prove or
  falsify them; it may not silently change them. Anything section 13 did not rule
  remains open with its named owner.
- **Any name.** Not the settlement's, not the dead world's, not the vocabulary
  for death, return, succession, ancestors, or departure (charter §15). The
  identifiers here are labelled placeholders.
- **The pulse cadence.** Ruled at 3.0 seconds (D5); it changes only by owner
  ruling with a side-by-side play-feel test.
- **Aging and departure.** [§2.5](../boundary-map.md#25-aging-and-departure)
  names the owner and the one-way invariant; the proof does not touch them, and
  this packet proposes nothing about them.
- **The visual target.** [Presentation direction](../presentation-direction.md)
  owns it, including the later target-authority packet; the owner owns
  acceptance.
- **Whether the proof is met.** That is gate G10, and it is the owner's.
- **Anything the owner has not ruled.** A gap here is named as a gap. It is not
  filled with the most plausible-sounding answer.
