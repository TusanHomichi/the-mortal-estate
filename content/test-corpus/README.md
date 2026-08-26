# Test Corpus — non-canonical

Everything in this directory is **test data**. It is the conformance corpus the
rules, protocol, and simulation crates are proven against.

It is **not** world canon, **not** lore, and **not** release content. Nothing
here names or defines the first settlement, the dead world, or anything this
project intends to ship. The identifiers are deliberately synthetic — a realm
called `testland` is a fixture, not a place.

Read [`docs/test-corpus-provenance.md`](../../docs/test-corpus-provenance.md)
for where this corpus came from, the ruling it migrated under, and what was
changed on the way in. Every fixture's `research_boundary.review_refs` resolves
there and to its own `*.provenance.md` sibling.

## Shape

| Path | Count | What it is |
| --- | ---: | --- |
| `*.json` | 52 | scenario roots |
| `*.provenance.md` | 52 | the per-fixture provenance record |
| `simulation_seeds/` | 52 | the seed each scenario instantiates |
| `world_templates/` | 52 | the world each scenario runs in |
| `catalogs/` | 1 | the shared definition catalog |

## The catalog is shared, and one thing in it is not corpus data

`catalogs/prototype_catalog_v6.json` is the only definition registry this
project carries, so it is also the registry an **authored land** resolves its
terrain, actors, and services in. Since slice S1 it therefore carries two
entries that belong to the identity proof's land rather than to any scenario
here: `actor-definition/identity_proof/threshold_keeper` and
`service/identity_proof/threshold_keeper`, both selected by
`profile/first_land_structure`.

They are named as what they are and they change nothing about this corpus: no
scenario instances them, and the identifiers are the design packet's placeholder
role labels, not names. They live here because there is exactly one registry and
a second copy of one would be a second source of truth for the same fact. A
production content registry is the recorded gap that ends this arrangement — see
[the authoring compiler](../../docs/authoring-compiler.md) and
[the land itself](../lands/identity-proof/README.md).

## Editing rules

- A scenario, its seed, and its template change **together**. They are validated
  as one unit and the harness will reject a partial edit.
- Changing any of them changes the golden transcripts in
  `crates/tme-sim/tests/golden/`. Regenerate them with the harness rather than
  hand-editing; a hand-edited golden proves nothing.
- Three crates reach this directory at **compile time** through `include_str!`.
  Renaming or removing a file here breaks the build at the compiler, not at an
  assertion.
