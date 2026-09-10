---
last_updated: 2026-09-10
revision: 4
status: Four-floor renderer verified in three browsers and installed on the private preview; saves preserved.
public_safe: true
summary: Area-selected Three.js dungeon renderer, retained town presentation, atomic pixel-dungeon retirement and proof scope.
---

# Live four-floor dungeon presentation

The owner accepted connecting all four existing dungeon layouts to live 3D
rendering before further artwork refinement. The
[presentation ruling](../presentation-direction.md#3d-reopening) owns appearance;
[client architecture](../client-architecture.md#the-renderer-seam) owns authority.
This Planning record owns the implementation slice and its proof.

## Scope

One product shell selects Three.js for `d1_entry`, `d2`, `d3`, and `d4`, retaining
the existing pixel town, temple and service interiors. This is an explicit area
selection, with no missing-art fallback or URL renderer switch. Connection,
commands, reconciliation and gameplay remain shared. No map, collision, timing,
class or save-format changes are part of this slice.

`worldRenderer.ts` owns canvas selection under the shared input surface.
`dungeon/renderer.ts` implements the existing frame/pointer/walk seam. Each newer
frame supplies the draw selection: observed cells within three squares of the
player, current actors and ground contents, and authoritative door states.
Absent or unobserved cells have no meshes. Level changes and disconnect clear
all preceding geometry and targets. Authored terrain fixes wall/door orientation;
its exported passability cannot authorize actions or expose hidden geometry.

`dungeon/topology.ts` supplies named world-cell wall sections. The fixed camera
keeps the selected elevation and perspective. Drawing-side wall placement and
height follow the prior study; physical collision remains wholly server-owned.
This is still provisional masonry, with limited fixed wall torches and cached
architectural shadows. There is no player light. Observed character occlusion
uses the existing transparency ruling. A lone body uses the exact tile centre;
shared-square offsets are deterministic presentation data.

The earlier custom body and idle clip are retained candidates in digest-bound,
self-contained GLBs. Creature names distinguish provisional shared bodies; this
slice does not claim completed creature art, locomotion or combat animation.
Movement changes authoritative positions directly; smooth travel and foot
contact remain the next character-presentation slice. Stair meshes, lighting
falloff and dark character fronts still require visual refinement.

## Atomic cutover

The `world` product build replaces the `pixel-art` build mode. Inspection remains
explicit and cannot be selected through navigation. Product fences reject the
retired pixel dungeon modules and diagnostic/study renderers. Pixel dungeon
source and proof are retired together; their unpublished candidate history is
retained externally. Town packet schema 7 refuses the old dungeon tile payload
and prior versions. Its raster receipt and the dungeon GLB receipt each bind
their own files through the same explicit external asset mount. Deployment copies
only receipt-bound assets and checks their hashes again after copying.

## Proof plan and findings

The new native proof uses disposable accounts and PostgreSQL with the actual
server, browser shell and TLS. Separate seeded routes check the local-door stop
and open-door sprint, temple round trip, and down/up stairs at each floor pair.
These checks prove all four floor presentations and selected inter-floor routes;
they do not claim every staircase has a native-browser traversal receipt.
Existing authoring/rules proof covers the complete authored stair inventory.

The same browser evidence checks observed-cell selection, fixed seven-cell
framing, character centering, black background, viewport changes, reconnect and
clearing on logout. Pixel town/temple controls and startup asset refusal retain
native proof. Full-floor walking and every alternate connector remain broader
route coverage, not a condition inferred from static exported passability.

A first door run reached the requested endpoint and correctly stopped at the
closed door, then completed a three-step sprint through the open door. Its
centering assertion initially compared floating-point world units exactly;
the corrected proof uses a numerical tolerance without changing occupancy.
The selected verification runner completed all steps in 237.186 seconds,
including Rust, Python, boundaries, TypeScript, 554 browser unit tests and the
product build. The immutable staged release passed all twelve primary native
scenarios (four routes in each browser), plus three actor-menu/startup scenarios.
Actual actor-menu traversal preserves the exact offered intent; missing and
wrong-digest body responses refuse startup. Chromium, Firefox and WebKit each
provided hardware-renderer evidence. All staged content digests match the
previous preview; this slice changes presentation, not authored maps or saves.
After the supervised-review fixes, the selected web/boundary runner passed with
556 browser unit tests. The final immutable release passed all fifteen native
scenarios again, including graphics-context loss clearing and its reload message.
Activation and served-file checks passed: the live HTML, bundle and both asset
receipts match the verified release, and the service reports gameplay ready.
Saved character state, inventories, banks, lockers and content were preserved;
the real Git index and HEAD were unchanged. The external release receipt owns
the installed identity and activation evidence. The retired `pixel-art` build
mode was also observed refusing execution before creating output.

## Follow-up ownership

Presentation owns brighter character fronts, smoother stair shadows, genuine
creature forms, translated locomotion and combat motion. Client presentation owns
reviewing raised-art hit regions alongside the current ground-cell pointing.
Mid-session graphics-context loss clears dungeon geometry and gives an immediate
accessible reload message; startup asset failures disable admission. Full-floor
route coverage should expand from these selected stair circuits, without using
exported passability as proof of runtime legality.

The supervised review found two implementation defects: dead actors contributed
to shared-square offsets despite not being drawn, and departing cloned rigs did
not release their skeleton textures. Both are corrected with targeted regression
proof. Shared asset geometry/material remain owned by the source asset. The
review's proposed observed-only wall orientation was rejected: authored structural
orientation is an allowed presentation input and stabilizes drawing across sight
windows; only observed wall rows become meshes. No hidden tile, door state or
occupant is drawn. Multiple enabled stair offers remain disambiguated through the
actor menu; no current authored dungeon access cell has competing stair offers.

## Fresh-session continuation

The owner authorized committing the accumulated entry, creation, geography,
door and renderer work together, followed by PR review, merge and branch cleanup.
The PR owns Git delivery status; the external deployment receipt owns the exact
installed build. Begin the next presentation slice with translated movement and
ground contact at the selected dungeon camera, then review character-front
lighting and stair shadows during actual travel. Retain the existing four-floor
layout, visibility authority and camera ruling. Creature forms and combat clips
remain separate unfinished work; technical playback is not visual acceptance.
