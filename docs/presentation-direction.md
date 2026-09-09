---
last_updated: 2026-09-09
revision: 64
status: Pixel town integration, grey-stone temple and Graveyard Keeper-style effects directed; character detail must survive scaling.
public_safe: true
summary: Pixel art, preserved character detail, temple material and hatch direction, atmosphere and lighting reference.
routes:
  - web/**
  - content/test-corpus/**
---

# Presentation direction

This Canonical document owns what the game should look like.
[Client architecture](client-architecture.md) owns the rendering boundary;
[browser client](browser-client.md) owns implemented behavior;
[pixel-art production](pixel-art-production.md) owns the reproducible art method.
Earlier camera, mesh, lighting and presenter decisions are preserved in
[presentation history](plans/2026-09-08-presentation-3d-history.md), not parallel active options.

## Pixel-art decision

**Owner ruling, September 8, 2026:** make a hard transition to the detailed
pixel-art result. The final temple with the Martial Artist, revised Tomas,
filtered sampling and larger room is the selected visual benchmark. The owner
next directed exterior construction in this style. The
[transition record](plans/2026-09-08-pixel-art-transition.md) owns implementation
and the stopping-point handoff.

Pixel art is the primary production direction. Do not resume 3D scene, mesh,
rig, water or vegetation polish as a competing product track. Existing
3D work may supply project-owned geography, proportions or guidance references;
its rendering limitations do not dictate the pixel-art style. Diagnostic tools
and historical evidence do not confer product authority.

This is owner acceptance of the visual direction and benchmark, not automatic
promotion of every generated file to an editable production master. Animation,
exterior construction, dead-world treatment and scalable scene assembly still
need their own delivered evidence.

**Owner direction, September 9:** extend the town using the detailed frontage
with standard pixel correction as the visual reference. Preserve its readable
architecture and quiet ground across adjoining scenes. This selects the look
for continued work; the assembled district still needs its own visual review.
The [exterior record](plans/2026-09-08-pixel-exterior.md#connected-standard-correction-study)
owns the resulting connected candidate and proof.

## Pixel atmosphere and shader effects

**Owner ruling, September 9:** retain the Graveyard Keeper approach to animated
pixel environments: height-aware fog, weather, and wind-driven foliage are part
of the playable pixel-art direction. This applies to the pixel renderer; the
retired 3D presentation remains retired. The primary technical reference is the
[developer’s effects breakdown](https://www.gamedeveloper.com/programming/graveyard-keeper-how-the-graphics-effects-are-made).

Keep environment effects on the scenery's native pixel lattice and preserve
integer scenery enlargement. Preserve the selected finer character raster and
filtered reduction so faces remain readable; a common coarser grid must not
silently discard their detail. Keep tree roots and architecture fixed; move selected foliage with
spatially varied wind. Fog should settle around ground and lower structures,
leaving high roofs and canopies clearer. Weather must respect indoor boundaries
and preserve readable actors, entrances and routes. Use restrained defaults.
The owner also explicitly selected its lighting treatment: ambient day/night
illumination, colour lookup tables, normal-mapped local lights, depth-sensitive
light distribution and sprite shadows. Prepare the corresponding lighting data
alongside colour art; these are part of the intended pipeline, not a competing
3D track. A painted scene
alone does not supply reliable normals or removable moving silhouettes.

Keep trees separately identified with fixed ground anchors and foliage masks.
Future chopping would need removable tree/stump art, clean ground beneath the
tree, and authoritative state. The owner explicitly places chopping far outside
this slice; current baked backgrounds do not prove removable trees.

Presentation effects own no collision, visibility, action timing or simulated
weather consequences. The [browser owner](browser-client.md#pixel-art-presentation)
records delivered effects and their proof; this direction does not claim all
of the reference game's effects are already implemented.

## Continuous visual review

Inspect the real playfield whenever working in or passing through an area.
Judge character faces, silhouette, furniture scale, reachable entrances, grid
alignment, occlusion and motion together. Fix bounded defects in the slice;
otherwise record a screenshot, location, observation, owning boundary and proof
needed. Placeholder art does not exempt a place from review.

## The target

A warm, richly coloured, inhabited tactical world made with finely resolved
pixel art. Preserve charm through materials, posture, use and purposeful
imperfection. Readability and coherence take priority over literal low-resolution
nostalgia, a provider's stock pixel style or physically exact camera geometry.

### The grammar

- One authoritative cell layout for every observer, player, resident and creature.
- Ground texture without a competing painted tactical grid; draw shared cell
  boundaries, routes and interaction information in the presentation layer.
- Camera-presenting upright figures, monsters, facades, doors, trees and props.
  Floors and selected top planes convey depth; feet, roots and bases stay anchored.
- Adult proportions and understated faces, with expressive clothing and posture.
  Avoid oversized heads, anime eyes and the cutesy proportions used to hide
  earlier mesh shortcomings.
- Rich, restrained material colour and wear placed where use explains it.
  Quiet ground regions and a clear value hierarchy preserve actors and routes.
  The owner explicitly rejected busy exterior generation: use broad calm ground
  and foliage masses, concentrating detail at meaningful focal points rather
  than scattering flowers, gravel and equally sharp texture across the scene.
- Local-material-coloured edges, selective deep accents and contact shadows.
  Avoid a continuous black contour, bright replacement rim or pasted-on halo.
- Enough actual display space for faces. Preserve source detail through sensible
  reduction and responsive layout; enlarged low-resolution pixels do not create it.
- Depth-ordered layers that can place people behind furniture, facades and trees.
- Restrained environmental motion, unsynchronized character presence and legible
  gameplay information within the same visual language.

### Warm architectural direction

Use substantial timber, stone footings, visible entrances, useful windows,
softened forms, repaired surfaces, stored supplies and practical lights.
Buildings communicate their purpose and agree with human scale. Exterior
construction starts from the current authored town, not a new decorative map.

### Stylization and restrained charm

Young Tomas is kind and tired, in plain dark working robes with the open hand.
Maude remains an older, fuller-bodied person; the Martial Artist supplies the
current adult player proportion reference. Natural differences in build are
welcome. No Christian crosses or church vestments; lore owns the religion.
Keep the working class name Martial Artist until the
[class owner](class-training-contract.md) records a rename.

### Projection and surface ruling

Use one presentation projection over the authoritative cells. **Owner
clarification, September 8:** show mostly the fronts of upright subjects, with
enough top surface to establish depth. Doors, windows, faces and bodies must
remain prominent; the exterior must not become a field of roofs and the tops
of heads. Match the selected interior's deliberate upright presentation.

Ground recedes on parallel axes. Upright artwork deliberately presents more of
its front toward the viewer than a physically correct elevated camera would.
Do not infer a compulsory camera pitch from the ground-cell aspect ratio or
force upright subjects through a physical 45-degree projection. Ground contact
and shared cell identity stay fixed; rendering never changes occupancy or reach.

### Tile assembly ruling

Terrain, overlays, art placement and pointing share cell identity. A room plate
is not collision data. Exterior chunks and foreground layers must be authored
against that shared layout and preserve traversable widths and entrances.

### Relative scale ruling

Compare people, monsters, doors, furniture, trees and buildings on their ground
contacts. Source canvas padding, opaque body height and foot pivot are distinct.
Increase the whole room's display size together when more detail is needed;
do not enlarge characters independently of their surroundings to rescue a face.
Nominal physical size and visible bounds do not replace authoritative occupancy.

**Owner clarification, September 8:** ordinary doors across the town share the
same human scale. Smaller doors indicate buildings that need to be enlarged as
complete structures, including their windows, roofs, steps and attached props.
Preserve compact architectural proportions; do not make an oversized doorway
in an unchanged miniature shop. Compare all buildings against one adult and
ordinary-door reference while retaining open streets and approaches.


### The dead world

Its appearance should express memory and unfinished continuity rather than
defaulting to blue-grey fog and skull decoration. The specific treatment remains
open; use the living-world scene structure to make correspondence judgeable.

## The production rule

The selected temple is the starting benchmark. The next exterior must prove a
second matching environment at actual play size: actors, terrain, structures,
interaction anchors, occlusion and UI together. Combat overlap, restrained
effects, living/dead correspondence and production cost remain subsequent proof.
Do not confuse a generated concept, a valid alpha channel, a passing test or an
enlarged review image with owner acceptance of the resulting playfield.

## The candidate lifecycle

Keep disposable experiments, review candidates, owner-accepted editable masters,
deterministic exports and promoted assets distinct. The owner's choice of pixel
art does not waive provenance, licensing, alpha checks or native visual review.
[Pixel-art production](pixel-art-production.md) records the techniques and
rejected approaches that produced the selected benchmark.

### Generated-source isolation and alpha validation

Generate one isolated reusable subject at a time with complete silhouette and
transparent padding. Do not generate production atlases as multi-subject sheets.
Verify actual RGBA alpha, then inspect over light/dark fields at source and native
sizes. Painted checkerboards remain disposable references. Extraction creates a
new candidate and preserves the untouched source. The
[production guide](pixel-art-production.md#alpha-outlines-and-light) owns details.

### Candidate assets

Project-owned generated images and receipts remain external, bound by manifest
and per-file hashes. Missing art or stale bindings refuse loading, never choose
a retired renderer. [Public boundary policy](public-boundary-policy.md) owns
source eligibility and promotion; [working roots](working-root-policy.md) owns
external/disposable state. The selected visual benchmark is not an exception to
those boundaries.

### Live characters

Characters use directional sprite artwork and matching animation. Preserve
identity, adult proportions and fixed gait pivots. Equipment appearance and
combat clips must be designed for this presentation; earlier requirements for
skinned meshes and modular 3D rigs are superseded. Do not claim a held standing
pose as finished walking animation.

#### Settling into an occupied square

Retain the intent that people inhabit a square through varied attention, weight
shifts, stretches, gear tending and suitable sitting/standing transitions.
Keep gestures within readable occupancy and interrupt them naturally for play.
Exact dwell times and sprite clips remain work to deliver.

#### NPC attention and room activities

Tomas may attend to the cot; Maude may work with supplies. Fit hands and feet to
prop-contact poses. Authoritative movement owns travel to another square, and
visual activity cannot add a delay to an otherwise legal service. The
[resident contract](town-resident-contract.md) retains its authority.

### Chrome and actions

**Owner ruling, September 8:** remove the provisional gameplay HUD. Use the
established direct movement interaction while the real interface is designed.
Single-click route drafts and endpoint/double-click confirmation remain; residents
retain their right-click interactions. The temporary movement buttons, readiness
sidebar, occupant list, generic equipment/action panels, coordinate readout and
instructional decoration are not the product interface.

### Accepted chrome layout

A real interface is a subsequent design task. Earlier painted chrome studies and
the temporary diagnostic panels are not accepted product chrome. Keep the actual
playfield readable and retain only necessary access/session controls for this
interim view. The eventual UI still owes the client accessibility contract.

### Proportional scaling

Fit the available window while keeping world proportions stable. A larger view
should provide more actual character detail; a decorative border must not force
an unintended extra image reduction. Browser client owns the measured dimensions
and responsive behavior. Judge compact windows separately from desktop captures.

## What is implemented today

The default playable renderer is pixel art with a connected town, a restyled
temple and pixel atmosphere/lighting. Other interiors use an explicitly labelled
map until their art is constructed. [Browser client](browser-client.md) owns implementation and
[the transition](plans/2026-09-08-pixel-art-transition.md) records proof and gaps.

## Who decides

The owner selects visual direction and accepts masters. Implementers choose
bounded techniques, preserve the selected identity and report evidence honestly.
A tool upgrade does not authorize style drift or a second production direction.

## Open

Further island/room construction, matching Martial Artist/Tomas gait, combat and
equipment art, refined lighting maps, low-end/crowd performance, dead-world
presentation and complete master promotion remain explicit work.

## Historical navigation

The following entries route earlier evidence; they are not current instructions.

### Asset sourcing order

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#asset-sourcing-order).

### Placeholder town interiors

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#placeholder-town-interiors).

### Building purpose from the street

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#building-purpose-from-the-street).

## The current target-authority packet

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#the-current-target-authority-packet).

## Paused-experiment presenter evidence order

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#paused-experiment-presenter-evidence-order).

## Retired pipelines carry no authority

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#retired-pipelines-carry-no-authority).

## The in-engine feel scene

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#the-in-engine-feel-scene).

### Movement and readiness

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#movement-and-readiness).

### Construction and viewport

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#construction-and-viewport).

### Interior camera

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#interior-camera).

### Structure and cards

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#structure-and-cards).

### Temple material and framing experiment

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#temple-material-and-framing-experiment).

### Room lighting from visible sources

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#room-lighting-from-visible-sources).

### Coastal water by depth and exposure

See [the historical ruling](plans/2026-09-08-presentation-3d-history.md#coastal-water-by-depth-and-exposure).
