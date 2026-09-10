---
last_updated: 2026-09-11
revision: 84
status: Selected dungeon view remains; shared Martial Artist combat motion accepted for integration, with walking variants requested.
public_safe: true
summary: Dungeon camera and scenery direction, shared martial motion acceptance and requested male/female walking variants.
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
[presentation history](plans/2026-09-08-presentation-3d-history.md), historical evidence to reassess under the reopening below.

## 3D reopening

**Owner ruling, September 10, 2026:** reopen 3D presentation. The previous
modeled temple is a useful quality benchmark; town exteriors still need work
and the dungeon has not had a finished 3D pass. This supersedes the September 8
prohibition on 3D product development. The following pixel-specific sections
record the previous direction and retained town/interior renderer, not a mandate
to produce more pixel artwork.

Begin with a player-centred 7-by-7 dungeon comparison, including walls and black
unobserved cells. Use real standing geometry and an adjustable elevated camera;
the owner likes the approximately 60-degree guide and explicitly values changing
camera elevation without redrawing assets. That guide is not a calibrated camera
acceptance. Retain centered feet for a lone occupant and consistent adult/door
scale. Camera changes never reveal unobserved cells or move authoritative cells.

**Wall-edge follow-up:** wall runs and their door frames/leaves share the tile
edge facing the player. The bottom (foreground) wall uses its tile's farther
edge instead. The whole bottom wall tile remains non-occupiable; shifting its
visible wall does not create another floor square. Join perpendicular wall runs
at their common edge intersection, and keep door hinges attached to that same
wall plane. In the private study, dark crossed tiles distinguish the retained
blocked area; this is a diagnostic treatment, not accepted final floor art.
World topology and passability remain authored/rules-owned.

**Height follow-up:** raise the bottom wall enough to conceal its blocked strip
at the chosen 60-degree view without covering the next walkable row. The north
wall adjoining the north doorway finishes flush with the existing doorway top.
Keep the door leaf at human scale; increased wall height elsewhere uses masonry
above the opening rather than stretching the door. This calibration is a study
at the chosen angle, not a promise that one fixed height hides a full row at
every camera elevation.

The visible wall/door join requires one continuous cap profile across the run,
including matching top, depth and front edges. Equal maximum mesh heights alone
do not prove alignment. Closed-door art must not expose a full-cell floor patch
above the lintel where it reads as a raised doorway top. This is a drawing
correction; doorway passability and server observation retain their owners.

**Fixed-camera traversal clarification:** the owner's adjacent-room concern
means walking south of the same wall while retaining the camera direction. It
does not require a reverse camera or an orbit. The owner confirmed that the
existing placement handles that case. Preserve this placement; the agent's
proposed wall-footprint overhaul came from a mistaken interpretation and is
withdrawn. The remaining comparison concerns the small foreground blocked strip
exposed by mild perspective. Camera and height trials remain private studies
pending visual acceptance, with authored blockage unchanged. The subsequent
taller-wall trial was rejected: its extra height would loom as the north wall
when entering the southern room. Keep the original wall heights; continue camera
comparison rather than raising masonry to hide the perspective strip.

**Selected camera approach:** the owner selected the 55-degree elevation with
mild straight-line perspective (20-degree vertical field of view) and the
original wall heights. Keep camera orientation fixed through movement. This
replaces the taller-wall trial and the 60-degree camera as the current dungeon
presentation baseline. Adjoining-room traversal and responsive framing remain
visual review concerns; selection of this approach does not accept unfinished
dungeon art. The live dispatch below separately authorizes integration.
Preserve the previously directed 40-percent
transparency for a wall or door obscuring an observed character; restore opaque
materials when the obstruction clears. This changes drawing, not world geometry,
passability or what the server permits the player to observe.


**Unseen-area and lighting ruling:** after comparing fog and portable-light
ideas, the owner selected complete black for unobserved areas, with occasional
wall-mounted torches providing limited pools of dungeon light. Retire the fog
veil and automatic player light from the study. Torch placement and falloff
shape the observed scene; they never expose hidden rooms or creatures, replace
authoritative visibility, or silently alter gameplay sight range. Treat local
baked/cached architectural lighting and bounded nearby light work as the initial
performance approach. Exact torch placements, final falloff and character
lighting remain art/integration work.

**Live dungeon dispatch:** the owner approved connecting all four existing
layouts to live 3D presentation next, with art refinement during actual play.
Town and temple keep their current presentation for this bounded integration.
The [live execution record](plans/2026-09-10-live-dungeons.md) owns implementation
and proof; this does not accept unfinished body, motion or scenery assets.

Prefer a better player body than the default Quaternius models. Existing custom
rigs are candidates, not accepted appearance. Mesh, rig and motion sources can
be evaluated separately. The owner specifically rejected treating the existing
walk, jog and run clips as good enough. Review foot contact, weight transfer,
turning, starts/stops and combat at the intended camera and play scale; merely
loading or renaming clips is not motion-quality proof.

The owner suggested generated motion video as a possible reference for Blender
keyframing. Evaluate short isolated actions with a locked camera, full body and
visible ground contact. This is a reference experiment, not automatic motion
capture, approval of generated anatomy or a new video-provider subscription.

**Shared martial motion follow-up:** the owner selected the shared Quaternius
male/female rig foundation after custom-body deformation trials, then accepted
the retargeted Meshy flying kick, guard, punches and blocks as good enough to
wire into play. Reuse shared motions across compatible bodies, with class-specific
combat clips and separate male/female walking variants where useful. The owner
requested Meshy walks next and authorized the associated credit spending.
This accepts the reviewed combat motion for integration; body appearance and
new walking quality still require review at the live camera. The
[motion execution record](plans/2026-09-10-martial-motion-integration.md) owns
delivery and evidence. The [gameplay baseline](gameplay-baseline.md#martial-artist-combat-sequence)
owns the intended closing attack; clip acceptance does not change its rules.

**Tool retirement:** remove the PixelLab MCP and the installed pixel-fixer
production tools. Stop new sprite-generation and pixel-treatment work. Preserve
existing artwork, source receipts and the installed preview while the replacement
is developed. General image generation and 3D tooling remain available.

The [reopening execution](plans/2026-09-10-3d-reopening.md) owns the bounded
comparison and handoff. Browser runtime and the atomic product migration remain
with [client architecture](client-architecture.md). Hunting-area timing remains
a [gameplay proposal](gameplay-baseline.md); this is a presentation ruling.

## Pixel-art decision

**Owner ruling, September 8, 2026:** make a hard transition to the detailed
pixel-art result. The final temple with the Martial Artist, revised Tomas,
filtered sampling and larger room is the selected visual benchmark. The owner
next directed exterior construction in this style. The
[transition record](plans/2026-09-08-pixel-art-transition.md) owns implementation
and the stopping-point handoff.

The September 8 ruling made pixel art the primary production direction and
retired 3D product polish. The [September 10 reopening](#3d-reopening) supersedes
that restriction. Historical tools still need review before product reuse.

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

## Entry and character creation

**Owner direction, September 9:** everything leading into play must feel like
the game, including sign-in, the roster, creation and connection recovery.
Use a full-screen game composition over the existing pixel world, restrained
framed controls, readable type and class emblems. Avoid website navigation,
account-dashboard cards and development explanations in the player flow.

Creation presents the five starting classes and the recovered base attributes,
class caps and unassigned points from the authoritative catalog. Nationality
choices are deferred. Do not invent stat effects, a sixth starting class or
new character art to fill this screen. The [class contract](class-training-contract.md#character-creation)
owns mechanical facts; the [browser contract](browser-client.md#private-authoritative-play)
owns entry behavior. The delivered layout remains subject to owner visual review.

**Owner follow-up, September 9:** do not offer a recommended point allocation
unless recovered evidence substantiates it. The current catalog suggestions are
authored examples, so the creation screen omits that option. Players distribute
their points manually; Reset points restores the selected class's minimums and
returns the full pool to spend again.

## Pixel atmosphere and shader effects

**Owner ruling, September 9:** retain the Graveyard Keeper approach to animated
pixel environments: height-aware fog, weather, and wind-driven foliage are part
of the playable pixel-art direction. This applies to the pixel renderer; the
3D reopening above supersedes the earlier retirement restriction. The primary technical reference is the
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

The September 9 dungeon clarification retains the town view as the first camera
reference. A dungeon-specific treatment is permitted if the town convention does
not read well in tight corridors; do not introduce that difference without a
concrete comparison. Use the current town and temple artwork with the existing
character as the dungeon guide inputs. The subsequent consistency clarification
kept upright subjects as the first choice, followed by a hold for live reference
dungeon observation. After that visit, the owner dispatched a square-ground
comparison with walls covering about one row. The subsequent clarification
explicitly permits a separate, steeper dungeon treatment, including a more
overhead character view and walls differing from town. The current comparison
retains upright artwork to isolate the spacing and framing change; town
presentation no longer constrains a later dungeon art revision. This permission
and experiment do not constitute final camera or artwork acceptance.

The owner subsequently directed a fixed, player-centred 7-by-7 dungeon playfield:
three cells each way, counting walls and black unknown cells within the 49-cell
footprint. Fit the complete field to the available screen with a corrected native
pixel grid and integer enlargement; higher resolution must enlarge the scene
rather than leave a small fixed-size board. The owner prefers the generated
60-degree study as the next art guide, including more overhead characters and
structures. Its angle label is an illustrative target, not a measured camera
calibration or an accepted sprite master. The current renderer still uses the
existing upright character art. The owner subsequently rejected that upright
figure in the dungeon; matching overhead sprites and architecture are required
before the candidate preview advances. New generated studies remain candidates.
The proposed wider distinction between hunting
and safe areas is recorded in the [gameplay baseline](gameplay-baseline.md#hunting-and-safe-area-proposal).

Ground recedes on parallel axes. Upright artwork deliberately presents more of
its front toward the viewer than a physically correct elevated camera would.
Do not infer a compulsory camera pitch from the ground-cell aspect ratio or
force upright subjects through a physical 45-degree projection. Ground contact
and shared cell identity stay fixed; rendering never changes occupancy or reach.

The full first dungeon floor uses a scrolling interior. Its staged frame declares
`fit_whole_level: false`; a room that declares whole-level composition still has
to fit its stated frame. The retired projected-shadow requirement must not force
an extra blocked cell behind each wall. Pixel foregrounds and shadows belong to
presentation; they cannot thicken the recovered collision layout.

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

The installed pixel presentation uses directional sprite artwork and matching animation. Preserve
identity, adult proportions and fixed gait pivots. Equipment appearance and
combat clips for its replacement follow the [3D reopening](#3d-reopening). Do not claim a held standing
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

The September 9 four-floor dungeon continuation permits provisional generated
floor, wall and door tiles. The [pixel production owner](pixel-art-production.md#provisional-dungeon-tiles--september-9)
records their preparation and acceptance boundary.
