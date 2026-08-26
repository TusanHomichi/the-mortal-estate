---
last_updated: 2026-08-19
revision: 1
status: Ported at the G1 to G2 boundary; pending owner acceptance at G2.
public_safe: true
summary: The successor's contract-first authoring standard for geography, authoring artifacts, conformance, validation, and owner taste gates.
routes:
  - crates/tme-authoring/**
---

# Contract-First Authoring for LLM Game Development

## Status

This is The Mortal Estate's authoring-contracts standard.

It is ported from a predecessor project's original of the same method, which
was authored at owner direction, adversarially reviewed by three external model
families on 2026-08-01, and owner-accepted the same day as a **living
standard**. The original declared its own core noun-substitutable by design;
this port performs that substitution. Predecessor-specific instantiation —
case evidence, tool and file wiring, verification scopes, issue and slice
history, and the house appendix — was dropped rather than translated. What
remains is the method.

Ported 2026-08-19, at the boundary between the evidence-duplication gate (G1)
and the staging-repository identity gate (G2). **Pending owner acceptance at
G2.** Until accepted it is the working standard for authoring work in this
repository and carries no authority over owner decisions.

This is deliberately the first substantive document in this tree. The
public-boundary machinery, the authoring compiler, the Workbench, and the
verification runner are all built under it rather than retrofitted to it. In
particular, **P9 is the bar every public-boundary check must clear**: a check
earns blocking status by killing a deliberate mutant, and runs advisory until
it does.

Owner mandate carried forward from the original: a PERMANENT method for
anything new, for the life of the project; covering ALL of the project's
geography — lands, settlements, dungeons, interiors; shareable outside the
project if it earns it.

## 0. Reading guide

Sections 1-9 are the method. They are written to survive noun substitution by
another project, and this document is itself the proof of that property: it was
produced by substituting nouns in a predecessor's copy. Project-specific
numbers, paths, history, and wiring do not appear in them. Where this document
names something this project has not built yet, it states the obligation in the
future tense and names who owes it. If a passage only makes sense with context
outside this file, that is a defect in this document.

Nothing here is instantiated yet. This project has no validators, no
verification runner, no blueprints, and no evidence convention at the time of
the port. Every "the validator blocks on..." below is a specification for work
not yet done, not a description of a running check.

## 1. Problem

LLM-authored visual and spatial work tends to converge when it aims at
machine-checkable artifacts and to drift when it aims at prose. We assert this
as a working hypothesis from one predecessor project's experience, not a proven
law; the method below is how we bet on it while measuring it.

Two failure classes dominate:

- **Plan-build divergence**: what was intended and what was built differ, and
  nothing fails.
- **False green**: a check exists, passes, and does not actually bind the thing
  it claims to bind. A false-green check is worse than no check. The standard's
  checks are themselves subject to the standard.

## 2. Principles

P1. **Intent that goes to authoring gets a contract artifact.** A contract is
    structured data (SVG, JSON, engine resource, table — or a hybrid document
    whose canonical data blocks a validator extracts). Prose motivates and
    contextualizes; it never owns a coordinate, dimension, or count. Where
    prose and data coexist, the data block is canonical and prose restating it
    is a lint error.

P2. **Contracts are authored originals and are tracked.** Reference imagery has
    three tiers, mirroring the project's public and private boundary policy:
    (a) private or third-party-derived references — quarantined, never tracked,
    never runtime inputs; (b) own-account generated references — quarantined by
    default, with provenance; (c) generated assets deliberately promoted to
    tracked use — permitted only through review, per-asset provenance,
    licensing basis, and the owner gate. A blueprint derived from a reference
    is original interpretation, never a trace or embed.

P3. **Severity is per assertion, not per artifact.** Every contract has a
    validator; within it, each assertion is classified BLOCKING or ADVISORY on
    two axes: consequence (what breaks if wrong) and oracle quality (how
    reliably the check detects the wrong). Blocking: schema validity,
    referential integrity, deterministic-projection drift, provenance and
    licensing presence, asset-pipeline integrity (missing textures, unbound
    materials, invalid resources), collision and navigation contradictions,
    required-evidence presence. Advisory: palette and taste ranges, perceptual
    diffs, placement of presentation-only dressing, any heuristic not yet
    qualified under P9.

P4. **Conformance is three proofs, not one.**
    (1) Source → canonical projection: the importer's output is validated
        against independently authored fixtures, including asymmetric ones that
        catch transposition.
    (2) Projection → tracked content: SET EQUALITY — every expected element
        present, correctly classed, correctly positioned, and no unexpected
        element in governed layers.
    (3) Tracked content → the PRODUCTION build path: the conformance test
        drives the same code path the shipped product uses, asserting semantic
        role (a door node that faces into a wall is not a door), not just
        id-at-position. A harness-only pass is a false green.

P5. **Machines gate structure, conformance, consistency. The owner gates
    taste.** Aesthetic metrics never block; they flag and trend. The owner's
    review time is spent exclusively on taste because everything structural
    arrived green. Advisory results and owner verdicts are RECORDED DURABLY
    (the verdict ledger, A5) so taste judgments accumulate into a labeled
    corpus instead of evaporating.

P6. **One source of truth per fact, with a named boundary owner per projected
    field.** A blueprint that projects into multiple domains (world topology,
    simulation anchors, presentation massing) declares which downstream
    boundary owns each field. No blueprint becomes a cross-boundary mega-owner;
    the projection map is part of the contract.

P7. **Representation-matched observation.** Every comparison binds a
    TARGET-FRAME DESCRIPTOR: camera geometry, viewport, content identity or
    digest, lighting and time state, UI state, animation state, and the named
    properties plus masks the comparison binds. Anything not in the descriptor
    is explicitly non-evidence. Binding targets are tracked or digest-pinned; a
    comparison against an unpinned image is advisory at most.

P8. **Compiled-content separation.** Runtime loads compiled, tracked content;
    it never loads blueprints or validators. Importers compile at authoring and
    verification time; derived content is ordinary content under ordinary
    content rules. Each artifact names where its compiled output lands and who
    consumes it.

P9. **Checks earn blocking status by killing mutants.** A structural check may
    block only after a mutation corpus proves it: for each defect class the
    check claims to catch, a deliberate mutant (deleted mesh behind a surviving
    id, swapped class, transposed coordinates, duplicated door, narrowed road,
    bypassed production path) must fail the check, and known-valid scenes must
    pass. Unqualified checks run ADVISORY until qualified. The mutant corpus is
    maintained like any other test fixture.

P10. **Proportionality.** The method scales to the act. Applicability profiles
    (structural world authoring; asset-kit authoring; material and lighting;
    presentation dressing; direct runtime-content authoring) declare which
    artifacts apply. Reuse an existing validated contract before inventing one;
    a new contract is justified only by a new durable fact owner, a
    transformation boundary, a cross-boundary interface, or a high-consequence
    invariant. A parameter tweak inside an existing validated schema needs
    nothing new.

## 3. The geography model

**One mechanism, many values.** In the owner's formulation: "The values change
but the mechanism by which we turn ideas into geographic content? Consistent."
The pipeline is invariant for every piece of geography the project will ever
author:

    idea/reference → blueprint (contract) → fail-closed import →
    compiled content → built world → conformance + lint → owner taste

What changes between a town, a dungeon level, a forest band, and a land is ONLY
the values inside the blueprints — dimensions, terrain classes, elements,
massing, palettes. No geography class gets its own bespoke process; a new class
gets, at most, new element types in the shared grammar and new P9-qualified
invariants. This is the property that makes the method permanent: learning it
once suffices for the rest of the project, and tooling built for one land is
tooling built for all of them.

The method must carry an entire game world: lands, settlements, dungeon levels,
interiors — and, in this project, the living and dead states of the same
geography, which the product requires to stay recognizably related. One
blueprint cannot; a hierarchy with an explicit connectivity contract can.

**Profile duality**: the two grammar profiles are inversions of one default.

- **Surface: default-WALKABLE.** Unless a feature is specifically meant to be
  impenetrable, terrain on surface lands is walkable — including shallow water.
  Blockage on a surface blueprint is a deliberate authored exception, and every
  blocked cell must belong to a DECLARED impenetrable feature (deep water,
  cliff, forest_mass band, structure footprint, land boundary) — a blocked cell
  with no feature membership is an authoring defect the importer rejects.
  Walkable terrain variety (shallow water, marsh, scree) is declared by terrain
  class; movement cost and burden stay owned by the rules boundary (P6), never
  by the blueprint. Companion invariant: no sealed walkable pockets — every
  walkable region connects to the blueprint's declared connectors.
- **Dungeon: default-SOLID.** Walkability is the authored exception — rooms and
  corridors are carved, and unexcavated rock needs no declaration. The dungeon
  profile adds its own primitives (rooms, corridors, lairs, authoring-time-
  secret doors) and its own P9-qualified invariants (compression budgets,
  sightline caps, no orphan room, lair bounds contain their anchors, mode
  flags).

One mechanism, two defaults, each making the author state exactly the thing
that is deliberate in that setting: what you may NOT cross above ground, what
you MAY reach below it.

What does NOT split with the profiles: the encoding standard, the importer
framework, the signature discipline, the conformance machinery, and above all
the ONE connectivity graph spanning surface and dungeon blueprints. A stair's
reciprocity cannot be validated across a format border.

**The terrain-class vocabulary is itself a contract.** Blueprint terrain
classes are not free-form labels: a tracked terrain inventory maps every class
to its content-registry terrain identity and names the gameplay semantics owned
elsewhere — movement cost and burden, one-tile-at-a-time crossing constraints,
and hazard-over-time behavior (per-pulse harm escalation for lingering on a
hazardous tile without the negating capability). The inventory validator blocks
on: a blueprint class with no registry mapping, an orphan mapping, or a hazard
or movement semantic asserted in a blueprint (P6: blueprints declare WHAT
terrain is; the rules boundary owns what it does).

### 3.1 Blueprint taxonomy

- **Land manifest** (one per land): the composition root. Names every member
  blueprint (regions, settlements, dungeon levels, interiors), their coordinate
  frames and origins, global id namespaces, seam ownership between adjacent
  surface blueprints, and the land's connectivity graph (3.2). The manifest is
  itself a contract with its own validator.
- **Region blueprint** (surface districts, wilderness zones, gathering
  landscapes): terrain-class grid layer plus structure and landmark metadata.
  **Gathering nodes are first-class elements of the surface grammar**: typed
  resource elements — tree stands and lumber, rock outcrops, ore veins, herb
  and reed patches, water sources — authored with resource-class attributes
  wherever overworld geography is planned, EVEN WHILE the consuming systems are
  inactive. Dormancy is explicit, not absent: a node is authored, projected,
  and conformance-asserted (present, positioned, correctly classed) with an
  `inert` activation state; activating it later is a content decision that
  flips state, never a geography retrofit.

  **Gathering terrain is walkable terrain.** In the owner's words: "people need
  to be able to walk through forests. Under tree cover on some level. Can't cut
  down a tree you can't touch." The surface grammar therefore distinguishes two
  forest classes that look similar and are not: `forest_mass` — blocked
  landscape used as edge treatment and sight boundary — and `forest_floor` —
  walkable gathering terrain whose tree elements are nodes the player stands
  beside. Canopy over walkable floor is declared in the blueprint (a
  canopy-cover zone class) so the presenter owns the resulting occlusion under
  whatever visible-bodies rule the presentation boundary adopts: foliage over
  the play space is the same occlusion problem as walls and buildings, and the
  blueprint's job is to declare it, not to solve it.
- **Settlement blueprint**: the town case — terrain grid, structure footprints,
  doors, lots, streets, plazas.
- **Dungeon-level blueprint**: rooms, corridors, doors (including
  authoring-time-secret doors — blueprints are not player-visible, P8),
  vertical connectors (stairs, shafts, ropes) as first-class elements, darkness
  and lighting zones, lair and spawn anchors, mode flags (a level or sub-zone
  declares its presentation mode where the game distinguishes exploration from
  combat pacing).
- **Interior blueprint**: small dedicated scenes (a bank, a shop); same
  grammar, one room scale.

All share one encoding standard (A1): a grid layer for per-cell terrain classes
plus vector metadata for structures and elements — two encodings, one importer,
chosen because pure-vector encoding fails at land scale (a finding all three
reviewing model families converged on) and pure-grid encoding cannot carry
element semantics.

### 3.2 The connectivity graph contract

Geography is a graph before it is a picture. The land manifest carries a
connectivity contract: every transition (door between blueprints, stair between
levels, portal, road exit crossing a seam) declared ONCE, with both endpoints,
direction semantics, and reciprocity requirements. Its validator blocks on:
dangling endpoints (a stair down with no stair up), non-reciprocal pairs, seam
mismatches (a road exiting district A where district B has no matching cell),
coordinate-frame violations, and orphan blueprints (no path to the graph root).
Within-blueprint reachability is the blueprint validator's job; between-
blueprint reachability is the graph's.

Where this project's living and dead states of one place are authored as
distinct blueprints, their correspondence is a graph relation like any other —
declared once, with both endpoints, and validated. A correspondence that exists
only in an author's head is exactly the plan-build divergence this method is
built to kill.

### 3.3 Geography-specific invariants (A4 members, P9-qualified)

- vertical reciprocity: every declared connector pair lands on walkable cells
  in both blueprints at the declared coordinates;
- no orphan room in a dungeon level (within-level reachability from its
  connectors);
- corridor and street width budgets: min and max widths are blueprint fields
  and the painted grid is measured against them (declared-vs-painted equality —
  the false-green lesson applied);
- anchor preservation: simulation-owned anchors (spawn sites, ecology members,
  arrival points) referenced by a blueprint are asserted present and exact in
  the projection, with their owning boundary named (P6);
- scale sanity: blueprint dimensions checked against the land manifest's
  declared budget envelope, so a land grows by decision, not drift;
- gathering-node reachability: every authored gathering node, active or inert,
  is on or cardinally adjacent to at least one walkable cell of its declared
  interaction class — "can't cut down a tree you can't touch" as a blocking
  check, so unreachable resources are authoring defects, never live
  discoveries.

## 4. Artifact catalog

Every artifact: what it binds, validator (with per-assertion severity),
compiled output and consumer, the failure class it kills, qualification status
(P9), first consumer.

### A1. Geography blueprints (taxonomy of 3.1)

- Format: hybrid encoding — terrain grid layer (compact per-cell rows) plus
  vector element metadata (axis-aligned footprints, doors, connectors, anchors
  as typed elements with attributes). SVG is the preferred vector carrier for
  human and agent editability; the IMPORTER is the contract, not the file
  suffix. Composition per the land manifest; no monolithic land-scale files.
- Validator (blocking): schema; on-grid integer geometry; footprint overlap;
  doors on perimeters; declared-vs-painted equality for every declared quantity
  (road widths, plaza extents); within-blueprint reachability to declared
  connectors; unknown fields and layers rejected; COMPLETE semantic signature —
  every semantic field hashes into the contract signature, annotations
  explicitly excluded by listing, one negative test per advertised guarantee.
- Compiled output: canonical JSON per blueprint plus the land manifest's graph
  projection. Consumers: world-template generation (topology), presentation
  structure sources, conformance manifests (A3).
- Kills: plan-build divergence, hand-edit rot, partial-signature false greens,
  grid drift.

### A2. Elevation blueprints (façades and massing)

- Format: per-archetype and per-building elevations; dimensions as data (story
  heights, door offsets, window rhythm, prop anchors), consumed by the building
  assembler as compiled layout data (P8).
- Validator (blocking): dimensional consistency closed-form: rhythm fits façade
  length under an explicit formula, door on ground band, heights within the
  massing class declared in the settlement blueprint. "Fits" is defined in the
  schema, not invented by the importer author.
- Kills: dice-roll façades, identity loss, silent constant drift.

### A3. Conformance manifests and three-proof conformance (P4)

- Exactness classes: STRUCTURAL (footprints, doors, connectors, anchors) —
  exact integers, blocking; DRESSING (movable presentation props) — presence
  within declared bounds, blocking at the bounds, not the centimeter. Class
  membership is declared in the blueprint.
- Includes semantic-role assertions (P4 proof 3) and set equality over governed
  layers. Reciprocity invariants — a visual door and its navigation or
  transition cell may not drift apart — are conformance members here. Where a
  design deliberately offsets a façade from its transition cell, the invariant
  asserts the pair, so neither half can move alone.
- Kills: dropped and moved elements, harness-only passes, drifted door pairs.

### A4. Geometry lint — typed structural predicates

- Typed, not generic: per-class support rules (grounded, attached, suspended,
  actor, effect); interval-union coverage for façade runs with declared
  openings (doors, cutaway states) subtracted; roof coverage by projected
  support region, not bounding box; intrusion checks against collision and
  walkability semantics, not raw render geometry; ground contact measured
  against local ground, tolerances from the kit matrix (A8), absence of a
  declared tolerance is an error.
- Occlusion and visibility rules are ADVISORY probes (named-anchor rays in
  controlled states) unless and until P9-qualified — visibility is not a
  scene-graph fact.
- Every predicate ships with its mutant set (P9). Unqualified predicates run
  advisory.
- Kills (once qualified): façade gaps, floaters, intrusions, hollow reveals —
  with proof it kills them.

### A5. Observation protocol and verdict ledger

- Target-frame descriptors (P7) with digest-pinned targets; required
  side-by-side comparisons per named anchor property; a TRACKED verdict ledger:
  round → property → owner verdict → capture refs. The ledger is the
  accumulating labeled corpus that advisory metrics are later qualified against
  (P9 for perceptual checks, if ever).
- The ledger is a small versioned text contract and is tracked in-repo.
  Referenced captures may remain untracked evidence, but each is pinned by
  SHA-256 digest in the ledger, so verdicts stay verifiable if evidence packets
  move or die. The owner's judgment history is permanent project data.
- The enforceable check is completeness and reproducibility of the record; the
  judgment is the owner's. This artifact makes judgment reliable and durable,
  it does not automate it.

### A6. Material and palette contracts — split per P3

- A6a Pipeline integrity (BLOCKING): every material slot bound, every texture
  path resolves, no fallback or error materials in governed scenes, shader
  parameters within declared valid ranges.
- A6b Palette conformance (ADVISORY): named-role color ranges sampled from
  fixture captures, flagged and trended in the round report. In-range is not
  "good"; the owner's verdict remains the taste oracle.

### A7. Fixture scenes

- Minimal isolated fixtures per subject (one building, one street, one dungeon
  room, one connector pair), stable ids and cameras; the substrate for A4, A5,
  and A6 observations and the mutant corpus.

### A8. Kit and family contracts

- Measured per-piece bounds under a declared measured-size convention,
  per-family declared tolerances (bleed, overhang, ground epsilon) that A4
  consumes, legal-adjacency data consumed by assemblers (the assembler refuses;
  lint exists for combinations assemblers cannot see), and — when a consumer
  exists — per-family performance budgets.

### A9. Land manifests and connectivity graphs (3.2)

- First-class artifact; validator blocks on the graph invariants of 3.2.
- Kills: dangling stairs, orphan levels, seam mismatches, silent land sprawl.

## 5. The authoring packet

What a lane receives, by reference: governing blueprints plus signatures; the
applicability profile (P10) naming which artifacts bind this round; target
descriptors and anchors; fixture list and capture matrix; the blocking and
advisory check sets; evidence layout. Packet assembly and evidence scaffolding
are generated by tooling, not hand-built — a method people route around has
failed regardless of its soundness.

A geography round's evidence packet additionally carries an **agent-played
first-playthrough report**: before the owner gate, an agent plays the built
content through the project's real runtime harness — traversing every authored
route, exercising every declared transition, and provoking every placed
encounter — and reports reachability, pacing, and difficulty findings with
captures. The report is supervisor evidence, not taste certification; its
purpose is to spend the owner's gate walk on taste rather than defect
discovery. The successor's runtime harness and capture tooling must be able to
produce this packet; until they can, geography rounds carry the gap explicitly
rather than skipping the requirement silently.

**Visual sign-off requires eyes.** When a round's work has a visual component,
the producing agent must view the actual rendered output — capture files,
contact sheets, the artifacts a human would judge, not logs, diffs, or code —
and iterate against the round's stated visual bars until its own inspection
passes, BEFORE submitting to any gate. The evidence packet carries a required,
named record of what was seen per iteration (pixel-level observations:
composition, seams, readability at gameplay scale), and any reviewer forwarding
visual work to the owner must view it first. Visual work signed off unseen is
an incomplete round regardless of its proofs: automated checks bound
correctness, but only inspection bounds nonsense, and the owner is the last
critic, never the first.

Where a round has binding feel targets, iteration compares against them for
feel-rhyme only — never pixel, layout, or payload copying. This is the same
line the project's public and private boundary policy draws, applied inside the
authoring loop where it is easiest to cross by accident.

## 6. Pipeline integration

- Validators wire into the project's standing verification runner per scope;
  blocking failures fail the run. **Forward obligation:** the successor's
  verification runner must define an advisory sink — advisory results emitted
  as a machine-readable report artifact into the round's evidence packet, with
  defined exit semantics in which advisory findings never alter exit codes. The
  sink is specified here because a runner knows only pass and fail otherwise.
- Versioning: SCHEMA VERSION (format) is separate from CONTENT SIGNATURE
  (instance). Consumers pin content signatures; conformance fails on mismatch.
  Schema changes migrate every active blueprint atomically in one slice via
  migration tooling (regenerate, re-sign, re-bind consumers, verify) — no dual
  formats, no compatibility readers, while contracts remain internal. If a
  blueprint format ever crosses an explicit external boundary (release,
  modding), it graduates to the project's external-boundary policy: explicit
  versions, migrations, support windows. That policy activates by owner
  decision, never by drift.
- Blueprint-to-projection lineage is recorded (which signature produced which
  compiled content), so audits can walk from any built scene back to the exact
  contract that authorized it.

## 7. Boundaries and non-goals

- No pixel-golden aesthetic gates. No fail-closed taste metrics — the
  predecessor tried and the approach was refuted empirically. No runtime
  blueprint loading. No validators without a named consumer in the same or next
  slice. No unqualified check ever blocks (P9). The owner's taste gate is
  permanent and load-bearing.

## 8. The extension recipe — anything new, any domain

0. **Inventory first.** Map the intent onto the project's existing contract
   inventory. Extend an existing validated contract before inventing one (P10).
   If the domain-native artifact is already structured, tracked, and
   validatable, IT IS the contract — add derivation and digest checks, do not
   build a parallel table that restates it.
1. **Name the binding representation** for what remains: spatial → geography
   blueprint; tabular or tuning → validated data; sequential → timeline data;
   relational → graph contract.
2. **Fail-closed validator** with per-assertion severity (P3) and a rejection
   test per failure class.
3. **Conformance**, branched by architecture: BUILT domains (a build step
   transforms contract into product) get the three proofs (P4). DIRECT-SOURCE
   domains (runtime consumes the validated artifact itself) get schema plus
   referential integrity plus bounded behavioral scenarios with EXPLICIT
   coverage statements — no manifest tautology, and no global claims ("every
   gate satisfiable") where the property is undecidable; state what the
   scenarios cover and stop.
4. **Fixtures and observation protocol** under fixed named conditions.
5. **Advisory taste layer** with durable verdicts (A5 pattern).
6. **Owner review placement**: for taste-led domains, one EARLY intent review
   before expensive implementation and one final review on green structure —
   discovering a tone rejection after full implementation is a process defect,
   not a taste defect. The early review is not a new gate: it lives inside spec
   acceptance, as a required artifact class. For taste-led rounds, the governing
   blueprint render and the picked target frames are part of what the owner
   approves before implementation launches.

Stopping rule: if applying a step produces an artifact that restates an
engine-native or already-validated source without adding a consumer, stop — the
step is done by inventory, not by duplication.

## 9. Shareability

Sections 1-9 are the publishable core. This document is itself the noun
substitution performed once; it carries no predecessor evidence, no house
wiring, and no case history. Claims asserted from project history are labeled
as such, and the convergence hypothesis of Section 1 stays a hypothesis until
evidence beyond one project exists.

The noun-substitution test remains part of this document's own review
checklist. Two additional rules apply to this copy:

- Any future instantiation detail — this project's verification scopes,
  evidence conventions, camera constants, contract inventory, and fact-owner
  registry — belongs in a separate house appendix, not in Sections 1-9. The
  core stays substitutable.
- A term scan for retired predecessor and source nouns is part of accepting any
  change to this file. A hit is a defect, not a style note.
