# Authoring fixture — synthetic, non-canonical

Everything in this directory is a **synthetic authored fixture**. It exists to
give the authoring compiler and the Workbench a real logical target to work
against, and for nothing else.

It is **not** world canon, **not** lore, **not** a design proposal, and **not**
release content. It does not name or imply the first settlement, the dead
world, or any place, person, or institution this project intends to ship. Those
are open questions and this fixture leaves them open. The identifiers are
deliberately synthetic: a realm called `testland`, buildings called
`fixture_structure_north` and `fixture_structure_outland`, a landmark called
`fixture_ruin_marker`. They are labels on geometry.

**Zero content authority.** Nothing here authorizes a coordinate, a name, a
terrain palette, or a piece of geography anywhere else in the project. If a
future land happens to share a shape with this one, that is a coincidence and
not a precedent.

## What is here

| Path | What it is |
| --- | --- |
| `fixture-surface.tmj` | The authored surface member, 24x16, the attested master |
| `fixture-interior.tmj` | The authored interior member, 10x8, one companion |
| `promotion.json` | The attestation receipt: per-file digests and a bounded authority block |
| `generated/world_template.json` | The runtime projection — regenerate, never hand-edit |
| `generated/compile_report.json` | The compile report — regenerate, never hand-edit |
| `generated/workbench_projection.json` | The logical view the Workbench renders — regenerate, never hand-edit |
| `fixture-swatch.png` | A synthetic editable master asset, 32x24, four neutral values, depicting nothing |
| `asset-provenance.json` | What that asset is, its digest and palette, and the authority it does **not** carry |

The two members are Tiled JSON maps. They carry **no tileset image**: this
project retired its predecessor's visual corpus and has accepted no successor
visual vocabulary, so a tile class here is a NAME and nothing more. Opening
either file in Tiled shows correct geometry and blank tiles, which is the
honest picture of where the project actually is.

## The land, in one paragraph

A small island. Water rings it. A route runs east to west across the middle,
bridging a pond, and a second route runs south from a dock landing up to that
crossing, with a spur through a patch of open ground where three buildings
stand — two clustered on the open ground, one off on its own. A ruin marker
sits in the south field. In the east, a shaft descends to a single walled
interior room, and the stair at the bottom comes back up to the same cell. Every
walkable cell on the surface is reachable from the dock, every walkable cell in
the interior is reachable from the stair, and the two are reachable from each
other.

## The swatch

`fixture-swatch.png` exists so that the Workbench's image operations have a real
editable master to protect. It is **not art, not a style, and not a proposal**:
three horizontal bands and a checker, in four neutral greys, written once by this
project's own standard-library PNG encoder. It depicts nothing.

It is **lane-authored**, not owner-accepted, and `asset-provenance.json` says so
in its status and its authority block. An owner-accepted visual master is a thing
this project does not have yet, and naming one here would fabricate the approval
the promotion gate exists to require.

It carries **one** anchor — its digest in the provenance record — rather than the
master's two. That is proportionality, not a weaker rule: no runtime loads it, no
projection derives from it, and nothing grants authority on its basis. The
master's second anchor exists because a compiled land reaches the runtime.

## Editing rules

- The master's digest is pinned in **two** places: this directory's
  `promotion.json` and a constant in `crates/tme-authoring/src/promotion.rs`.
  Changing a member means changing both, deliberately, in one commit.
- `generated/` is written by `cargo run -p tme-authoring`. `--check` proves the
  tracked bytes are exactly what a fresh run would write, so a hand edit and a
  stale projection fail the same way.
- The terrain classes resolve against the terrain registry in
  `content/test-corpus/catalogs/prototype_catalog_v6.json`. A class with no
  registry mapping fails the compiler.

Read [`docs/authoring-compiler.md`](../../docs/authoring-compiler.md) for the
compiler's fail-closed classes and the mutant that qualifies each one.
