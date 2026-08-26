# The identity proof's land — authored, compiled, and served

This directory is the first authored land in this project that a **runtime
loads**. Everything the server serves for the identity proof is derived from
`settlement.tmj` by `cargo run -p tme-authoring`, and nothing else in this
directory is hand-written except the promotion receipt and the served-world
declaration.

## What is here

| Path | What it is |
| --- | --- |
| `settlement.tmj` | The authored `settlement` member, 48 x 32, the attested master |
| `promotion.json` | The attestation receipt: per-file digests and a bounded authority block |
| `world.json` | Which catalog, profile, compiled template, and seed make this world |
| `simulation_seed.json` | The cast that stands in it |
| `generated/world_template.json` | The runtime projection the server loads — regenerate, never hand-edit |
| `generated/compile_report.json` | The compile report — regenerate, never hand-edit |
| `generated/workbench_projection.json` | The logical view the Workbench renders — regenerate, never hand-edit |

## The names are placeholders, and they claim nothing

Owner ruling D2 reopened every name, and charter section 15 leaves the
vocabulary for the settlement, the dead world, death, return, succession,
ancestors, and departure open. `identity_proof`, `settlement`,
`settlement_ruin_mouth`, `proof_keeper_hall`, `threshold_keeper` and
`ruin_mouth_lair` are **labels on geometry and roles**, exactly as the design
packet labelled them. None of them is a proposal for a name, and a later slice
renaming all of them costs a re-attestation and nothing else.

The terrain classes are the runtime terrain registry's, which today is the test
corpus catalog's `profile/first_land_structure` — the only registry this project
carries. That binding is deliberate (an unmapped class fails the compiler rather
than the running world) and it is a recorded gap, not a decision about this
land's palette: see [the authoring compiler](../../../docs/authoring-compiler.md)'s
named gaps.

## The land, in one paragraph

A shore meadow on the west coast, a settlement above it, and wooded country
east. The landing sits at the water's edge and the road turns inland almost at
once: north to the settlement's street, east along it past the keeper's hall,
then south and east again out of the settled ground. Past the last houses it
crosses a channel on a short bridge, and the country closes in — the road runs
INSIDE the wood from there, bending south around a rock outcrop it cannot
cross, east along the shore of a forest pool, then north and east again to a
clearing in the trees where a ruin stands. Marsh fills the south-west, a stand
of trees the north, and rock closes the border. No stretch of the road runs
straight for more than seven cells, and every walkable cell is reachable from
the landing.

The dangerous route is that road east: four turns past the bridge, and a wood
between the keeper and the thing at the ruin mouth. The danger is authored as distance
and cover, not as a level boundary.

## What the owner accepted, and what that does not cover

The owner accepted this geography on **2026-08-21**, after sending the first
authored version back for a shape pass. The receipt records that acceptance
(`owner_accepted_at_s1`) and so does the reviewed contract in
`crates/tme-authoring/src/contract/identity_proof.rs`; the two moved together,
which is the ceremony the double anchor exists to require.

What the acceptance covers is what the authority block says and nothing beside
it: coordinates, terrain and passability, structures and landmarks, member
transition endpoints, and this land being loaded by a runtime. It is **not** an
acceptance of art, of tuning, or of anything as canon — those three stay false
in the receipt, and every name here is still a placeholder role label.

## What is not here yet

- **The layer beneath.** The land declares one member. The dead layer is a
  second authored member with the same envelope, and it arrives with the slice
  that authors it — the member count is content, so that slice adds a
  declaration rather than changing a type.
- **A descent.** With one member there is no member transition, so the
  `transitions` object layer is present and empty.

## Editing rules

- The master's digest is pinned in **two** places: `promotion.json` here and the
  land contract in `crates/tme-authoring/src/contract/identity_proof.rs`.
  Changing the member means changing both, deliberately, in one commit.
- `generated/` is written by `cargo run -p tme-authoring`. `--check` proves the
  tracked bytes are exactly what a fresh run would write, so a hand edit and a
  stale projection fail the same way.
- Editing this land through the Workbench is the ordinary path:
  `docs/workbench-v1.md`. An applied truth edit changes the compiled template,
  and a running world whose content identity no longer matches is refused at its
  next checkpoint — which is correct, and means the demonstration restarts the
  world from an empty database.
