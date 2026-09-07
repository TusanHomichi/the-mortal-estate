# First expedition

This land encodes the owner-accepted town surface, ten service interiors and
bounded first-floor entrance. The [surface brief](../../../docs/plans/2026-09-05-first-land-surface.md#geography-acceptance-and-implementation-2026-09-06)
owns acceptance; the [execution record](../../../docs/plans/2026-09-06-first-expedition.md)
owns gameplay integration, native expedition proof and remaining limitations.

The arrival member is 27 by 36 with 472 walkable cells and dock arrival `(9,31)`.
The dungeon entrance is seven by seven with 17 walkable cells. Closed doors and
the remaining cropped cells bound deferred content. Twenty ordinary directed
passages join town to its interiors. Temple `(0,4)` descends to entrance `(5,4)`;
entrance `(6,4)` returns to temple `(0,5)`. The selected historical correspondence
remains explicitly provisional as recorded in the surface brief.

## September 6 acceptance and encoding

The original September 6 reviewed packet manifest SHA-256 was
`0b9148ab18a4fd5066cb2845f3fcb6cdab4fbe1afa7cdfb2c586f532cba94e6f`.
The original candidate and raw research remain outside the checkout. The owner
approved its geography and instructed implementation to proceed. This receipt
accepts geography only; artwork, tuning and canon are not accepted by it. The
[interior ruling](../../../docs/presentation-direction.md#placeholder-town-interiors)
designates current room artwork as placeholders pending guidance and iteration.

The authored files are an encoding of that review, not byte-identical copies.
Encoding preserves cell masks, terrain roles, structure footprints and access,
the dock arrival, and every portal departure and destination. Candidate-only
metadata is replaced by authored metadata, terrain classes gain neutral runtime
identities, interior JSON gains complete Tiled layers, and transitions become
explicit compiler objects. The dungeon's candidate inspection marker is removed
from the arrival program: the world's arrival remains the dock. Structure
purpose strings retain their candidate labels and grant no service capability.

An independent comparison derived the canonical geographic record directly
from the accepted packet and source layouts before invoking the compiler. Its
SHA-256 is
`2a06190f4b4a1fcf0c1792f8f722b6cd8b08522d6be8dba6f7415f72ad038d2b`.
The compiler computes that same record from compiled cells, structures,
landmarks, arrival and connected endpoints. The selected revision's review identity and geographic
digest are pinned in the land contract and receipt, in addition to the normal
master-byte anchor and per-member digests. Changing and re-signing a companion's
landing cannot preserve the accepted geographic identity.

## Inputs and outputs

The twelve `.tmj` members, `promotion.json`, `catalog.json`, `simulation_seed.json` and `world.json` are
authored inputs. `generated/` is deterministic compiler output; regenerate with
`cargo run -p tme-authoring`, and check with the same command plus `-- --check`.
Builds and runtime require no private packet, archive, encoding script or wiki.

The catalog declares `clean_authored_content`, which receives the same required
boundary scan as original fixtures. Its initial combat, item and skill definitions
are inherited integration values and remain provisional under the
[gameplay ruling](../../../docs/gameplay-baseline.md#first-expedition-provisional-values).
The native expedition proof loads this declaration through the real server;
its evidence is recorded in the execution record. Successful compilation and
integration do not establish historical numerical fidelity. Presentation mass excludes furniture: blocked
furniture is occupancy, not a wall with a projected structural shadow.


The [gameplay provenance](gameplay.provenance.md) distinguishes recovered
creation bounds and class relationships from provisional loadouts, resources,
growth, encounter values and service inventory. The cast includes fifteen town
residents and one bounded encounter. `world.json` is the served-world declaration;
no server has a built-in default for it.

## September 7 temple amendment

The owner selected the revised temple guide and directed implementation of the
larger or better-arranged room. The temple now has a 7×8 envelope and 41 passable
cells; its entry is `(3,6)` and town-facing departure `(3,7)`. Dungeon endpoints
retain the coordinates above. This is a bounded amendment to the earlier review,
not a claim that the September 6 packet contained the new room.

The current layout receipt SHA-256 is
`d132d434d60e20d15b2962834bfde80e6bd04d8cf8d01da615674c297dee8d73`.
Its independently derived geography SHA-256 is
`2ce205b35536006932437c855827575d26f61a26813ade0bc885160db95534d4`.
Those current identities replace the earlier anchors in the land contract and
promotion receipt. The original review remains archived outside the checkout.
[The town record](../../../docs/plans/2026-09-06-town-buildout.md#september-7-temple-iteration)
owns this implementation. The [resident contract](../../../docs/town-resident-contract.md)
owns Maude and Tomas's service bindings and circuit; artwork remains candidate.
