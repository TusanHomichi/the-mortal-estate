---
last_updated: 2026-09-04
revision: 3
status: Compiler and land contracts; fixture accepted at G4X and identity-proof geography at S1. Audit clarifies diagnostic serving and remaining report obligations.
public_safe: true
summary: Land declarations, compiler commands, promotion, deterministic output, Workbench operations, and proven rejection classes.
routes:
  - crates/tme-authoring/**
  - content/lands/**
---

# The authoring compiler

`crates/tme-authoring` turns an authored Tiled document into proven runtime
content. It is the successor's re-authoring of a compiler method that worked in
a predecessor project; the method came across and the land did not.

## What it compiles

A **table of lands**, declared as data in `src/contract/`. Each land declares its
own identity, its own promotion receipt, its own outputs, and **its own member
set** — the count is content, not a type, so a land of one member and a land of
three run the same code and a new member is a declaration rather than a change
to the compiler.

| Land | Members | Runtime | Attestation |
| --- | --- | --- | --- |
| `authoring_fixture` ([directory](../content/authoring-fixture/README.md)) | `surface`, `interior` | diagnostic capture only; no production content authority | owner-accepted at G4X |
| `identity_proof` ([directory](../content/lands/identity-proof/README.md)) | `settlement` | served through its explicit world declaration | owner-accepted at S1 |

The fixture carries **zero content authority** — it exists so the compiler and
the Workbench have an honest logical target, and it deliberately names nothing
this project intends to ship. The identity proof's land is the one a runtime
loads; its receipt is the only one in the tree whose authority block sets
`runtime_loads_authoring_source`, and owner ruling R1 (2026-08-21) is what put it
there.

Every subcommand but the build addresses **one land, named explicitly**. There
is no default land: the compiler carries more than one, and a tool that guessed
would eventually edit a document nobody asked about. `--member` is optional
because the compiler derives it from the land's single candidate entry point,
which is a derivation from the contract rather than a guess.

## The six properties, and where each lives

| Property | Where it lives |
| --- | --- |
| **Typed input** | `src/tiled.rs` — every reader returns a typed value or a located error, and nothing downstream touches raw JSON |
| **Fail-closed validation** | `src/compile.rs` — one implementation of member semantics, no advisory tier, no partial acceptance |
| **Deterministic projection** | `src/project.rs` — ordered maps into the runtime's own type, through the single serializer in `src/emit.rs` |
| **Exact identity** | `src/contract/` — the land table, and per land its members' envelopes, vocabularies, layer sets, map properties, authored programs, receipt, and expected authority block, declared as data |
| **Promotion separation** | `src/promotion.rs` (the only path to a `Land`) and `src/candidate.rs` (the same semantics, no authority) |
| **Reproducible reports** | `src/emit.rs` — one serializer, pretty-printed with a trailing newline, over ordered inputs |

`src/export.rs` adds a third emission rather than a seventh property. It writes
each land's `generated/workbench_projection.json`: the same
compiled land, carrying the facts a logical view needs and the runtime
projection has no reason to hold — structure identities with their access cells
and façade doors, landmark and transition positions, per-cell passability, and
each cell's terrain attributed back to the authored layer it came from. It goes
through the same serializer, is covered by the same `--check`, and grants no
authority: nothing reads it back into a `Land`, and the runtime never sees it.

It exists so the Workbench can render the compiler's own truth instead of
computing geography a second time. See [Workbench V0](workbench-v0.md).

## The staged-operation vocabulary

`src/operations.rs` and `src/replay.rs` are the crate's second service to the
Workbench, and they live here for the same reason `export.rs` does: **a verb is a
statement about the authored object model**, and that model is declared exactly
once in `src/contract/` and asserted exactly once in `src/compile.rs`. A verb
set defined in the tool would be a second opinion about what an authored member
is made of; a re-implementation of the derived layers in another language would
be a second implementation of the compiler's semantics, which is the false-green
failure P9 exists to kill.

Six verbs, each with an acceptance case and a rejection case in
`tests/operation_replay.rs`. Three tile layers are **derived** — the structure
footprints, the landmark and transition markers, and the passability annotation —
and no verb writes them: the replay refreshes them and the compile independently
asserts them, so a wrong refresh is a rejected candidate rather than an accepted
lie. The verb table, the rulings behind it and the rejection each verb trips are
[Workbench V1](workbench-v1.md#the-verb-table).

Replay grants nothing. It reads the accepted master as bytes, writes a candidate
to a path its caller names, and produces no value `promotion::load` would accept.

## The command line

```bash
cargo run -p tme-authoring                          # compile EVERY land, write tracked output
cargo run -p tme-authoring -- --check                # prove the tracked projections are current
cargo run -p tme-authoring -- --report               # print the byte-reproducible reports
cargo run -p tme-authoring -- describe-operations --land <id> [--member <id>]
cargo run -p tme-authoring -- validate-candidate --land <id> [--member <id>] <document>
cargo run -p tme-authoring -- project-candidate --land <id> [--member <id>] <document>
cargo run -p tme-authoring -- replay --land <id> [--member <id>] \
    --operations <ops.json> --output-dir <dir> \
    --expect-base-sha256 <hex> [--validate] [--project]
```

The four subcommands are machine entry points: each writes one JSON document to
stdout and answers with its exit code — **0** yes, **1** no with the reason on
stdout, **2** the request could not be read. Distinguishing 1 from 2 is
load-bearing: "the candidate is rejected" and "the validator never ran" are
different facts.

`--expect-base-sha256` is required and never defaulted. Replaying against bytes
the caller did not expect is how a stale session quietly edits a document nobody
looked at.

Only the first form writes tracked bytes, and it is not reachable from the
Workbench: the bridge names none of its flags, and a test reads the bridge's
source to prove it.

## The double anchor

The promotion gate requires **two** independent agreements before authored
bytes become a compiled land, per land:

1. that land's `promotion.json` — a receipt carrying per-file SHA-256 digests,
   an attestation, and a bounded authority block;
2. its `master_digest` in `src/contract/` — the same digest, in reviewed source,
   beside the attestation and the authority the receipt must match **exactly**,
   in both directions. A receipt may neither grow an authority nor drop one.

Either alone is weak. A receipt can be rewritten by anything that can write
files; a constant alone cannot carry per-file digests or a bounded authority
statement. The mutant that proves the arrangement is
`a_master_edited_and_resigned_together_is_rejected`: it edits the master AND
re-signs the receipt to agree, which defeats a receipt-only gate and dies
against the constant.

**Named friction, honestly.** Because the reviewed digest lives in Rust source,
every accepted geography edit costs a source change and a rebuild. That is
correct while accepted edits are rare, and it will be felt once the Workbench
makes truth edits cheap. Revisit the SHAPE of the second anchor when there is
real usage evidence; do not revisit its existence.

## The candidate path

`validate_candidate(land, member, document) -> CandidateReport` runs that
member's semantics with no authority at all: it reads no receipt, consults no
reviewed digest, asserts no attestation, and writes nothing. It calls the same
`compile_member` the promoted path calls, so a candidate learns the real rule
at taste time rather than at promotion time.

`CandidateReport` exposes no conversion into `Member` and no constructor for
one. That is enforced by the type system rather than by convention, and proven
by a `compile_fail` doctest on the function itself.

## Mutant table (P9)

A check earns blocking status by killing a deliberate mutant. Every class below
has one, and every mutant starts from the tracked fixture and changes exactly
one thing.

### Surface semantics — `tests/surface_mutants.rs`, via the candidate path

| Class | Mutant |
| --- | --- |
| Envelope | width 24 → 25 |
| Envelope | `infinite` → true |
| Map properties | `content_authority` given a value |
| Map properties | an extra property appended |
| Tile vocabulary | a tileset class renamed |
| Layer set | `landmark_marks` removed |
| Layer set | an extra object layer appended |
| Layer set | a layer name duplicated |
| Tile vocabulary | a tile id outside the class set |
| Tile vocabulary | a footprint class painted into `base_terrain` |
| Tile vocabulary | a second embedded tileset |
| Terrain completeness | a `base_terrain` cell left at 0 |
| Declared vs painted | one passability annotation flipped |
| Declared vs painted | one footprint tile cleared under a structure object |
| Reachability | two cells watered over to seal a walkable pocket |
| Door geometry | access cell moved off the footprint edge (0 touches) |
| Door geometry | access cell moved inside the footprint (2 touches) |
| Structure access | access cell repainted as deep water |
| Structure access | the route spur cleared, detaching two structures |
| Structure scope | town ground repainted to grass under a clustered building |
| Footprints | two structures overlapped |
| Structure program | a structure renamed |
| Structure program | a structure reclassified `isolated` → `clustered` |
| Structure metadata | `occupied` → false |
| Structure metadata | `purpose` emptied |
| Structure metadata | object class changed |
| Landmark program | a landmark renamed |
| Landmark markers | the ruin marker tile cleared |
| Landmark placement | the arrival moved off the route network |
| Landmark placement | a landmark moved onto deep water |
| Transition program | `target_member` rewritten |
| Transition geometry | access cell moved away from its marker |
| Transition markers | the shaft marker tile cleared |
| Coordinate lattice | an object nudged 3px off the cell grid |

### The staged-operation vocabulary — `tests/operation_replay.rs`

Each verb twice: once producing a candidate the compiler accepts, once producing
one it refuses. A verb with no rejection is a verb nobody has shown the compiler
can refuse, and a test in that file asserts every published verb has both.

| Class | Mutant |
| --- | --- |
| `set_terrain` | deep water over a structure's access cell |
| `set_terrain` | two cells watered over to seal a walkable pocket |
| `set_terrain` | a route class written into the terrain layer |
| `set_route` | the route cleared from under the arrival landmark |
| `move_structure` | an isolated building moved onto town ground |
| `move_structure` | a structure the member does not carry |
| `set_structure_access` | the access cell moved inside the footprint (2 touches) |
| `set_structure_access` | the access cell moved away from the building (0 touches) |
| `move_landmark` | a landmark moved onto deep water |
| `move_landmark` | a landmark the contract does not carry |
| `set_transition_endpoint` | the access cell separated from its marker |
| `set_transition_endpoint` | an operation that moves neither endpoint |
| Vocabulary | an unknown verb, an unknown parameter, a cell outside the envelope |
| Vocabulary | a dressing or asset operation handed to the truth entry point |
| Vocabulary | an operation against a member with no candidate entry point |

### The derived layers — `src/replay.rs`

The refresh is arithmetic, and the argument for doing it in the replay is that
the compile checks it independently. That argument is worth what its mutants are
worth, so both are planted:

| Class | Mutant |
| --- | --- |
| Passability | a terrain edit applied without refreshing the annotation |
| Footprints | a structure moved without refreshing the footprint layer |

### Interior semantics and the connectivity graph — `src/mutants.rs`

The interior has no candidate entry point (the Workbench spec asks for a
surface one), and the graph's failure modes are unreachable by document
mutation precisely because the member compile enforces an exact transition
program first. Both are qualified against compiled values instead — the only
honest way to prove a check that a stricter check stands in front of.

| Class | Mutant |
| --- | --- |
| Envelope | interior width 10 → 11 |
| Map properties | interior declares `member_role: surface` |
| Layer set | a `routes` layer added to the interior |
| Tile vocabulary | a tile id outside the interior class set |
| Tile vocabulary | a passability class painted into `base_terrain` |
| Declared vs painted | one interior passability annotation flipped |
| Reachability | two cells walled over to seal an interior pocket |
| Transition program | the interior transition renamed |
| Transition markers | the stair repainted as plain floor |
| Graph — unknown member | `target_member` → a member the land does not carry |
| Graph — dangling pair | `paired_transition` → a transition the target does not carry |
| Graph — reciprocity | a pair that resolves but does not name each other |
| Graph — direction | a pair where both halves descend |
| Graph — endpoint | an endpoint moved onto a blocked cell |

### The promotion gate — `tests/promotion_gate.rs`

| Class | Mutant |
| --- | --- |
| Master digest | a byte appended to the master |
| Companion digest | a byte appended to the companion |
| **Double anchor** | master edited AND receipt re-signed to match |
| Authority — over-broad | `presentation_art` → true |
| Authority — over-broad | `content_canon` → true |
| Authority — incomplete | `coordinates` → false |
| Attestation | status raised to an owner approval nobody granted |
| Provenance | `review_refs` emptied |
| Receipt shape | the companion dropped |
| Receipt shape | an unrecognized receipt field added |
| Receipt presence | the receipt deleted |

## Named gaps

Recorded rather than papered over.

0. **A coordinate written into a mutant can silently disarm it.** Found while
   measuring the promotion ceremony for Workbench V1: an accepted edit that moved
   a landmark left `an_unpainted_landmark_marker_is_rejected` clearing an
   already-empty cell, so the mutant was ACCEPTED and a check that had been
   binding stopped binding anything. Both coordinate-pinned checks about that
   landmark now derive the cell from the landmark they are about. The general
   lesson is recorded rather than swept: **a mutant should name what it is about,
   not where that thing happened to be**, and an accepted geography edit sweeps
   the corpus with it. See
   [Workbench V1](workbench-v1.md#5-the-second-promotion-anchor-spec-134).

1. **Owner attestation — RESOLVED, twice.** A receipt is authored with a LANE
   attestation and a pending status, because fabricating an owner approval would
   make the strongest check in the crate a lie. The owner accepted the fixture at
   gate G4X (2026-08-19) and the identity proof's land at slice S1 (2026-08-21,
   after sending the first authored version back for a shape pass); each time the
   receipt's status and attestor were re-signed together with their reviewed
   constants — the ceremony the double anchor exists to require, now executed on
   two lands.
2. **No review-artifact digests.** The predecessor's receipt pinned owner
   review-image digests. The successor has no accepted visual review artifact
   class, so the receipt pins document digests only. The receipt gains a review
   block when that class exists.
3. **`visual_manifest_digest` is a borrowed field.** The runtime world-template
   contract requires a 64-hex `visual_manifest_digest` and this project has no
   visual manifest. The projection pins the field to the authored master's
   digest so it carries a real, verifiable identity rather than a placeholder.
   Revisit the field's NAME when a presentation boundary is decided.
4. **The terrain registry is the test corpus catalog.** Every land's terrain
   classes are resolved against
   `content/test-corpus/catalogs/prototype_catalog_v6.json` under
   `profile/first_land_structure`, because that is the only terrain registry the
   successor carries. The binding is deliberate — an unmapped class is a compile
   failure rather than a load-time surprise — and each land declares its own
   registry pair, so a production registry re-points one land at a time.
   **This is felt now that a land is served**: the identity proof's settlement
   paints `testland_*` classes because that is the vocabulary the one registry
   carries. Those are registry ids, not a decision about the land's palette.
5. **No elevation, kit, or observation artifacts.** The authoring standard's A2
   (elevations), A5 (verdict ledger), A6 (materials) and A8 (kit contracts) have
   no instance here. The fixture is geography only; those artifacts arrive with
   the boundaries that consume them.
6. **No agent-played first-playthrough report.** The standard requires one in a
   geography round's evidence packet. The existing live and capture harnesses
   prove bounded interactions; they do not produce that report. The gap is carried explicitly rather than the requirement being
   dropped.

## Running it

The command line is above. The corpus is:

```bash
cargo test -p tme-authoring             # the mutant corpus, including the doctest
```

The promotion-gate mutants run against **every** land in the table, and the
member-semantics mutants against the fixture, which is the land that exists to be
mutated. `tests/identity_proof_land.rs` proves the served land's own contract:
its envelope, its cast's geography, the emitted template's shape, and that it is
the only land carrying the runtime-loading authority.

The `compile_fail` doctest on `validate_candidate` is part of it: it is the
type-level proof that a candidate report cannot become a `Member`, and flipping
the fence produces `error[E0277]: the trait bound Member: From<CandidateReport>
is not satisfied`.
