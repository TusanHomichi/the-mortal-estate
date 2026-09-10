# First expedition

This land encodes the owner-accepted town surface, seven service interiors and
four complete dungeon-floor frames. The [surface brief](../../../docs/plans/2026-09-05-first-land-surface.md#geography-acceptance-and-implementation-2026-09-06)
owns acceptance; the [execution record](../../../docs/plans/2026-09-06-first-expedition.md)
owns gameplay integration, native expedition proof and remaining limitations.

The arrival member is 27 by 36 with 680 walkable cells and dock arrival `(8,34)`.
The four floors are 36, 38, 36 and 37 columns wide, each 43 rows high, with
635, 686, 765 and 585 potentially traversable cells. Their 199 door endpoints
include four paired grand-door endpoints. The complete land has 46 directed
graph connections and 241 runtime topology edges.
Temple `(0,4)` descends to dungeon `(24,7)`; dungeon `(25,7)` returns to temple
`(0,5)`. Subsequent sections retain earlier encoding receipts as history.

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
growth, encounter values and service inventory. The cast includes twelve town
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

## September 8 town amendment

The owner selected the seven-building roster and directed its implementation.
The horseshoe places the temple at its crown, retains the dock and water cells,
and removes three standalone retail members. The [town record](../../../docs/plans/2026-09-07-horseshoe-town.md#september-8-implementation)
owns inventory disposition and cutover proof. Retained interior geometry and the
temple/dungeon connection remain unchanged.

The current layout receipt SHA-256 is
`aa9c82d286a0bb6c6da2b4236e9851f437e2a3743e550e43adbc939f2d2c8133`.
Its independently derived geographic SHA-256 is
`4a49d0cb2e2f495e64bdb60ce100927f0c4dfce9b64eb3cc2e05722c3d57b39d`.
These replace the September 7 anchors; the earlier review remains archived.
Artwork and scenic water are candidates, not accepted masters.

## September 9 pixel-town encoding

The owner explicitly approved fitting the rebuilt town's authored geometry to
the connected pixel scene, as recorded in the
[surface amendment](../../../docs/plans/2026-09-05-first-land-surface.md#pixel-town-geography-amendment).
The arrival remains 27 by 36, with seven structures and fourteen ordinary
passages. All eight companion Tiled files are byte-identical to the previous
encoding; their services and the temple/dungeon pair remain unchanged. Only the
arrival map and controlled-player dock seed move. No wider island map is edited.

The external geometric review records the source scene digest, uniform projection,
seven footprints/access/return pairs, coast, pond masks, street lines and prop
anchors. An independent canonical geographic record, derived from those explicit
facts and retained interiors, agrees with the Rust compiler. Blocked terrain
replaced by a bridge is correctly omitted from the resulting terrain stack.
The promotion receipt and reviewed Rust contract pin together:

- Master `0b6cd1149004780dbab45ec52ef9853751ed02e1962efba25317fdc6de09f6ee`.
- Geometric review `c8ddf6d5724e9f53361185c469cadb47f827c539bbb5c32f671844edabd263d0`.
- Canonical geography `869e33951932dde1d470111c746802ee02cc141213019fe0c049c3da07f039be`.

This receipt grants no artwork-master, tuning or canon authority. The
[exterior execution](../../../docs/plans/2026-09-08-pixel-exterior.md#gameplay-integration-and-pixel-effects)
owns runtime presentation proof. Hosted state has not been migrated or activated.

## September 9 initial first-floor encoding — superseded history

The owner directed copying the selected historical first floor. The
[execution record](../../../docs/plans/2026-09-09-dungeon-one.md) owns that dispatch,
source identity, implementation and proof. This supersedes the 7×7 entrance crop.
All 1,505 cells retain their source-region positions, including solid margins,
rooms, corridors, ordinary and concealed doors, water, earth, rubble, pit and
chasm. Original artwork and prose remain outside the checkout.

The floor has 38 initially closed ordinary doors, seven initially open doors and
ten concealed doors. Its temple-connected component has 598 potentially
traversable cells. Ten explicitly declared cavern components account for the
remaining 36 under current corner and vertical-traversal rules; no invented
corridors connect them. Concealed-door player discovery remains unfinished.
The second surface stair, three lower-floor stairs and eastern passage are
retained reserved geography; they have no invented destinations.

The review SHA-256 is
`94e5c07474b5eee6a281a174150ebdb5d3e9699e992e865ad5e302d4f6906500`;
the independently derived canonical geography SHA-256 is
`ada3b68bee963c439a3d07803d88a04e86c7116d6f6f9b8ea283d31bcdc969a3`.
The eight non-dungeon members, including the town master, retain their prior byte
identities. The receipt and Rust contract pin the new review and geography.
The pixel packet is rebound to this geography with byte-identical image assets;
the dungeon uses observer-driven map rendering, with no new artwork acceptance.

## September 9 four-floor encoding

The expanded [execution record](../../../docs/plans/2026-09-09-dungeon-one.md)
owns corrected source boundaries, the omitted locker cutout, connected stairs,
guild corridor and remaining mechanical gaps. The first-floor-only receipt above
is historical; the selected review SHA-256 is
`a476f7f0ac5a440c4af65307d46a8e9d882a0e7910a9337a4aebcfbf00dd2c19`,
and its independently derived canonical geography SHA-256 is
`56e414e7fdeee374eab714fbae38770b5a8f06a592998f6a57a2690593857d55`.
The eight other members retain their previous byte identities. Pixel packet
version 5 binds this review and four provisional generated dungeon tiles.
