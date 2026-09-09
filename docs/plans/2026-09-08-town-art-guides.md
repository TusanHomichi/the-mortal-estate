---
last_updated: 2026-09-08
revision: 2
status: Selected guides inform the seven-building native exterior candidate; artwork acceptance remains open.
public_safe: true
summary: Shared town and building guidance, purpose-readable architecture, external provenance and implementation constraints.
---

# Town art guides

Planning record for the owner-requested image-generated architectural and
environmental guides. The [surface brief](2026-09-05-first-land-surface.md#current-town-building-roster)
owns the selected roster. The [presentation direction](../presentation-direction.md#building-purpose-from-the-street)
owns purpose readability: architecture and visible use identify the building
with its signs removed. The owner explicitly reinforced that requirement for
the temple and every other functional building on September 8.

## Generated packets

The built-in image generation tool produced the external
`town-guides-20260908-r1` packet: a town overview, four-building civic sheet and
six-building trades sheet. The selected September 5 architectural guide supplied
material direction, and the September 7 blockout supplied layout reference.
The owner subsequently selected a reduced roster. The omitted shop concepts
remain historical alternatives, not current building targets.

The external `town-guides-20260908-r2` packet updates the town overview for the
current roster and leaves the centre open. A targeted correction replaced an
overly domestic-looking bank with a masonry counting house and attached sheriff
office. The current town guide SHA-256 is recorded in the packet's
`provenance.json`, alongside both exact prompts, source references and image
digests. Its README links the current image and the retained correction draft.
All raster images and prompts remain outside the checkout.

The civic sheet remains useful for sanctuary masonry and entrance treatment,
the bank's substantial base, the lodge's communal porch, and the training hall's
open practice bay. The current trades references show a forge working arch,
an outfitter's leatherwork porch and an open provisions frontage. Inventory
consolidation follows the surface brief; images introduce no item definitions.

## Implementation constraints

These are guidance candidates, not owner-accepted artwork masters or game
screenshots. The selected roster does not approve the image's exact coordinates,
architecture, sacred symbols or invented details. Generated ground boundaries,
camera angles and proportions are illustrative. Implementation must retain the
authored map, actual entrances and collision, fixed zero-yaw 45-degree camera,
native gameplay scale and visible tactical grid. The first overview underplayed
the grid; that omission does not amend the existing presentation ruling.

Build and inspect silhouettes and useful entrance forms first, followed by
materials, working features and restrained signs of occupation. Simplify dense
roof courses and prop detail into readable groups. Judge the identity with signs
hidden at gameplay size. The temple is the proposed first implementation target,
followed by the lodge and bank; the owner has not accepted final models here.

The seven-building authored migration and proof remain tracked in the
[horseshoe record](2026-09-07-horseshoe-town.md). Existing ten-building native
receipts cannot prove the reduced roster or generated architecture. The guide-generation pass made no runtime change; subsequent implementation
and deployment evidence belongs to the horseshoe record.

The owner subsequently requested further town-wide iteration against these
guides. The [exterior and ground iteration](2026-09-08-town-visual-iteration.md)
owns that modeling, native review and preview follow-up.
