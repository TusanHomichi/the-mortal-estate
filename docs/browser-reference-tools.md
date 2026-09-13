---
last_updated: 2026-09-13
revision: 1
status: Reference documentation for retired local 3D tools; no product integration or current visual acceptance claim.
public_safe: true
summary: Historical packet, scene, material and capture tooling separated from the active browser owner.
routes:
  - web/src/space/**
  - web/src/feelScene.ts
  - web/proof/walk-proof.mjs
  - web/proof/capture-packet.mjs
---

# Browser reference tools

These are the retained local tools from the earlier 3D studies. Their packet,
placement, camera and asset contracts do not control the playable world. The
[browser client](browser-client.md) owns the active Three.js renderer and current
proof commands. Earlier observations below are dated study evidence.

### Earlier local scene

One click drafts a shortest legal route; a second click on its endpoint or a
double-click commits. `walkIntent.ts` owns the disposable intent state and
`movement.ts` its local timing and route allowance. Each accepted move gets a
full three-second interval. Competing clicks, Escape, and right-click cannot
replace or cancel it; Escape and right-click can clear an uncommitted draft.
The logical square changes at completion. This implements the local stand-in
for [D5](boundary-map.md#21-authoritative-individual-deadlines-d5).

`route.ts` searches legal neighbours breadth-first with stable target-facing
tie breaking. The three-step allowance counts actual traversed steps, including
detours. `layoutPassability.ts::canStep` owns walls, props, and diagonal corners.
A target needing four steps remains refused even when it lies closer in a
straight line. These are preview constraints, not accepted D2 gameplay values.

`walkPresenter.ts` draws the figure along the committed route. Its facing follows
each segment, including a skipped landing frame; idle facing follows the
pointer's ground cell. The rig's forward axis is +Z about world up, and portals
preserve heading. Outdoor camera focus stays on the logical caretaker square;
interior focus stays on the room. Landings update outdoor focus.

The ready arrow gains a large amber hourglass during cooldown or a coral cross
for a refused target, with matching ground outlines. Cooldown keeps the cursor
locked even away from the grid. A hidden live region announces state changes.
There is no visible countdown, movement status panel, or experiment label.
Only the movement owner releases the lock; animation cannot grant readiness.

Route categories remain walk/run/sprint. They select packet clip names, which
may all name the same clip. The current candidate uses `Walk_Loop` for all three;
changing categories with the same clip preserves stride phase. Clips play at
native rate. Distance-driven stride pacing, planted-foot correction, and early
visual arrival are **not implemented**. Early arrival is allowed by the
[presentation ruling](presentation-direction.md#movement-and-readiness),
provided it never runs late and does not release the cooldown.

## Packet and rendering contracts

The 3D packet, lighting and scene sections below describe retained reference
tools. They are not current pixel-art production requirements. The active
pipeline is [pixel-art presentation](browser-client.md#full-3d-presentation).

`manifest.ts` validate current schema 7. Schemas 1–6 and absent
required fields are refused; there is no compatibility parser. Assets remain
outside the checkout. A packet is a candidate, not an accepted master.

### Cards and lighting

Prop placements carry finite `elevation` from zero through six world units and
`card_height`, the height of the projected image rather than the subject's own
height. The retired `nominal_height` key is refused. View-facing and wall-plane
cards anchor at elevation plus half their image height. Floor-facing cards are
allowed only for assets declaring `flat: true`; no flatness is inferred from art.

Optional normal sheets belong only to prop rows. They are digest-bound,
decoded as data, and must match the color sheet's pixel dimensions. Normals use
the card's frame (right, up, toward viewer); mirroring a placement mirrors its
tangent. All sheets decode with `premultiplyAlpha: "none"` so transparent normal
pixels survive filtering. The outstanding normal-sheet surround issue is #29.

`cardLighting.ts` uses wrapped diffuse lighting with width 0.5 in both standard
and wind materials. Shader patches fail if Three.js changes their insertion
anchors. Ground and wind shaders output sRGB; indoor and outdoor palettes are
separate. Wall-attached fixtures use batched geometry, occupy their tile, and
own practical lights; hearth fire remains a card.

Outdoor tree and grass cards share a world-position-phased wind field. Decoded
art weights canopy motion separately from trunks. Deterministic grass clumps
use one non-blocking, non-shadowing instanced draw and are omitted indoors.
First-land tree variety is an [authoring requirement](presentation-direction.md#structure-and-cards),
not a guarantee made by the current arbitrary packet loader.

### Coastal ground cover

`terrainSurface.ts` owns shared coastal relief and ground contact.
`groundCover.ts` perturbs the visual path margin without changing that height,
the authored material identity or any walking verdict. Both the coastal material
blend and grass placement consume this wear field. Worn path centres remain
clear; shorter edge blades may cross into the margins of lane cells. Other
authored materials, including stone and decks, keep their separate surfaces.
Restrained dry-turf patches and small worn path stones belong to the coastal
ground shader. Scene geometry, coastal ground and water share the palette's key
direction; the exterior key illuminates street-facing relief from the front-left
in the current visual candidate. The [town iteration](plans/2026-09-08-town-visual-iteration.md)
records its native comparison and remaining art work.

`grassCover.ts` builds curved geometry blades, with shorter town grass and taller
meadow cover. Grass and flowers are grouped into small spatial instance batches
so colour and shadow frusta can cull distant patches. Their bounds include the
maximum wind and recent-step displacement; grouping changes no placement.
Sparse cream flowers share the rooted placement field. Grass and
flowers use the same wind and recent-step deformation in colour and shadow
passes. This is presentation motion; it creates no harvesting, trampling or
movement rule. The current candidate uses quieter generated turf and earth
swatches; its sources and the original opaque tree geometry remain external.
Tree structures remain static GLBs and have no wind deformation.

### Figures and structures

`figureRig.ts` decodes the rig, clip library, outfit parts, buffers, and textures
from verified bytes. The glTF URL modifier resolves only listed basenames to
verified blob URLs; unlisted requests fail. Resolution and load errors remain
fatal even if the glTF loader catches a texture error and returns a material
without the requested map. Refusal disposes already parsed figure sources.
Missing idle or movement clips fail.
Instances clone skeletons and materials, apply the packet palette and rim,
play clips through a mixer, and cast/receive shadows. The candidate asset packet
owns figure scale; the renderer imposes no fixed actor height. The current
private study's character packet and native evidence belong to the
[character study](plans/2026-09-06-character-study.md#client-integration-and-sourcing-pass).
Accepted appearance and further animation classes remain open.

Every space carries `structures`. Each placement names a digest-bound static
GLB, anchor, quarter-turn yaw, and bounded inclusive footprint. GLBs must embed
buffers/images and contain no skins or animations. Decoded geometry/materials
are decoded once per verified digest, shared by space instances, and disposed
once when the packet scene stops. Structure roots use the same terrain contact
field as figures. Ground verdicts own compiled occupancy; the footprint bounds
authored placement rather than inferring collision from visible eaves. An outdoor building model does
not imply an interior or portal.

Spaces connect through explicit door portals. Landing on a portal swaps space
and tile at the same completion, rebuilding local passability, hover, occlusion,
and focus. Closed exterior footprints block covered tiles except portals; roofs
remain visible dressing. Interiors omit roofs/weather, shorten camera-near walls
to their sills, and use their own props/lights. Foreground surfaces follow the
[fading contract](#foreground-surface-fading) in either space.

## Operation and proof

From the repository root:

```bash
npm --prefix web ci
TME_FEEL_ASSETS=/absolute/path/outside-checkout npm --prefix web run dev
```

Use Vite's printed loopback URL. `web/vite.config.ts` serves the external packet
only during development. `web/dist/` does not package or serve private assets.
An absent or refused packet produces an absence banner.

Comparative captures that substitute earlier source modules must preserve the
exact runtime dependency URLs, including Vite dependency-version queries. A
second Three.js module instance invalidates the matched baseline even when the
scene renders. The [shape-pass execution record](plans/2026-09-08-town-visual-iteration.md#upper-building-shapes-and-grouped-banks)
owns the discovery and qualification of earlier comparisons.

`presets.ts` owns query controls, for example `?preset=night,wind&zoom=-1`.
Whole-number zoom steps from −3 to 3 change world height by a quarter per step;
the ruled frame remains the default. The camera fixes vertical extent to nine
cells and derives horizontal extent from aspect ratio. Equal world extent across
display shapes and proportional production chrome remain unfinished. The
explicit private study described below integrates the authored expedition with
server authority. There is no separate engine entry point.

The [web lane](verification.md#browser-evidence) proves locked dependency install,
typecheck, synthetic unit tests, and build without candidate assets or a GPU.
Optional real-tab evidence requires a packet, installed Playwright engines from the shared roster, and ffmpeg for the walk sequence:

```bash
TME_FEEL_ASSETS=/absolute/path/outside-checkout node web/proof/walk-proof.mjs
TME_FEEL_ASSETS=/absolute/path/outside-checkout node web/proof/capture-packet.mjs \
  --out /absolute/capture/output --query preset=night --width 1280 --height 800
```

The walk proof uses 1280 × 800, writing to `TME_CAPTURE_OUTPUT` or a named temporary
directory. It checks timing, competing-input refusal, cursor feedback, detours,
travel, facing, portals, and camera focus. Its bounded in-page trace records motion
even when slow screenshots skip frames. Movement sequence output is
`walk-movement-sequence.webp`. Each engine gets its own process/server and output
subdirectory. The capture command requires `--out` and accepts repeated queries;
output names include query and engine. `serve.mjs` owns the loopback server group.

### Candidate ground verdicts

The reserved cell material `void` authors a floor opening. It requires
`walkable: false`, carries no terrain asset, and retains its explicit grid
address. Both ground renderers and the tactical grid omit its surface; normal
landing validation refuses it. This extends the schema 7 material vocabulary
without permitting omitted cells or deriving collision from artwork. Static
geometry may show descending stairs below an opening; a destination requires
its own authored space and portal. `floorOpenings.test.ts`, manifest refusal
tests and the native town walkthrough cover this seam.

Candidate cells carry a required boolean `walkable`; absent or nonboolean
verdicts are refused. Static GLB bounds never imply collision: a dock may be
walkable, a house occupied. Candidate authors supply those ground verdicts;
compiler-backed candidates copy `Member::is_passable` and verify the complete
packet mask against it. Local wall, roof, and prop occupancy still constrains
synthetic feel spaces. It must not introduce differences in a compiled preview.
The authoritative client continues to receive its geography from the server.
Outdoor weather classification uses the declared weather setting, including
exteriors whose roofs are embedded in static meshes.

`ground.ts` selects the coastal presentation for outdoor spaces carrying
`water`. `terrainSurface.ts` owns its deterministic fine-grid height field;
`coastalGround.ts` triangulates that field, blends grass/path/bank materials,
and draws a separate sea 0.24 world units below the town datum. Figure contact
interpolates the same triangles; vegetation and static structures share that
sampler. Foundations stay level and dock cells supply a separate level deck datum;
material context preserves its sharp edge above the underlying seabed.
Stone, planks and other authored materials retain their own swatches.
Gentle meadow relief is presentation only. It adds no cells, traversal costs,
height legality or server authority. Noncoastal spaces retain flat ground.

`inlandShore.ts` derives closed water contours from the existing water/deck cells,
rounds their corners and applies gentle deterministic variation. Components that
reach missing cells retain the existing open-coast treatment. Shared terrain
sampling owns the resulting bank, seabed and ground contact; natural ground also
covers dry scenic corners within water cells, with submerged fragments clipped
by the water datum. This changes no cell material, standing verdict or route.
`inlandShore.test.ts` covers wet/dry centres, rounded corners, open/deck-connected
water, islands, diagonal contacts and overlapping pond bounds. The
[pond iteration](plans/2026-09-08-town-visual-iteration.md#rounded-pond-shoreline)
owns the native comparison and deployment evidence.

Coastal ground also adds a deterministic, sparse batch of embedded faceted stones
through `groundStones.ts`. Water-neighbour cells receive a denser sampling pass;
a narrow dry-side margin groups larger stones with open gaps between patches.
Broad vertex colour tones retain their facets in building shadows.
It keeps portal clearance and quiet lane centres;
instances have no occupancy or traversal authority. Ground texture combines broad
wear with smaller scuffs, while submerged pebble outlines use angular profiles.
The inland contour keeps gentler large bends with smaller edge irregularity.
Candidate buildings, dock timbers and tree bark use original embedded tangent
normal maps through the existing GLB material loader. Those maps change shading,
not geometry, practical-light metadata or foreground-fade ownership. The
[surface texture iteration](plans/2026-09-08-town-visual-iteration.md#material-relief-and-broken-ground)
owns asset provenance, comparison and resource evidence. The subsequent
[shape and bank pass](plans/2026-09-08-town-visual-iteration.md#upper-building-shapes-and-grouped-banks)
varies upper-building pitch and individual slate positions in the external
candidate GLBs. Vertex normals follow the deformation; attached smoke markers
follow their chimneys. Textures, mesh topology, placements and gameplay masks
retain their existing owners.

Coastal vegetation follows [Coastal ground cover](#coastal-ground-cover).
Other exterior spaces keep the card-clump treatment; `grass_clump` remains part
of that vocabulary.

### Foreground surface fading

The [owner ruling](presentation-direction.md#projection-and-surface-ruling)
sets the visual target. `surfaceOcclusion.ts` replaces both wall-run selection
and the stencil figure silhouette. After the walk presenter moves the figure
and camera each frame, it tests nine body rays against static foreground mesh
geometry. Cached bounds reject unrelated meshes before triangle tests; selection
runs every 75 ms, with a 150 ms release hold and a 300 ms eased transition.
These are local animation values, unrelated to gameplay deadlines.

Walls, individual roof batches, solid hearth parts, prop cards and placed model
meshes participate. Terrain, grass, contact shadows, particles and the actor do
not. Roof batches are per roof so one blocker cannot fade another building.
Transparent card padding rejects ray hits using decoded alpha and transformed
UVs. Painted wind cards use their resting mesh for selection; their rendering
passes preserve the same live deformation. The body samples and release hold
reduce small hole flicker; this is sampled coverage, not a pixel-perfect mask.

Each selected mesh temporarily owns material clones. Shared model materials
remain unchanged, so repeated trees stay independent. A colourless depth pass
runs after ordinary scene colour; the faded colour pass accepts equal depth.
Only the nearest selected foreground layer blends per pixel, including within
a leafy mesh. Geometry, alpha cutouts and shadow coverage remain present.
Custom lighting callbacks and wind uniform identities survive cloning; emissive
window updates follow their original owner. On release the original material
and render order are restored; disposal releases only the controller's clones.
The development hook exposes `surfaceFades()`, and the stage carries
`walkFadedSurfaces`; the old wall-run hook and counter are removed.

Observed arrival proof and the separate calibration-packet regression are
recorded in the [surface receipt](plans/2026-09-05-first-land-surface.md#arrival-verification-receipt).

### Coastal water and exterior camera comparison

`waterSurface.ts` builds one connected surface, dense around the authored land
and coarse offshore. It samples the existing bank beneath decks and retains a
shore-distance attribute; it changes no movement cells. `coastalWater.ts` applies
three analytical swells on the GPU, finer normal ripples, view-dependent sky
colour, crest lighting and bank-intersection foam. It uses one transparent water draw over one continuous opaque seabed, with
no scene-copy or reflection-camera pass. `seabedSurface.ts` refines submerged
bank triangles to the terrain lattice and stitches shared edge midpoints into
adjoining coarse triangles. This prevents the earlier coarse bottom cutting
through the ground while preserving the water mesh and offshore density.
`seabedSurface.test.ts` proves bank contact, unchanged input and a closed interior
edge topology across refinement. Natural ground fragments below the water datum
are omitted in favour of this continuous bottom. Ground shadow overlays stop at the water datum and blend
before the sea, and tactical grid ink fades at the waterline instead of tracing
submerged tile skirts. Dry cell addresses, grid emphasis and legality are unchanged.
Both geometry and fine ripple strength
follow optical depth and exposure. `coastalProfile.ts` validates bounded scenic
depth/exposure zones at the asset boundary; `terrainSurface.ts` samples the same
submerged height field used by water and shore geometry. These optional profiles
are restricted to outdoor water spaces. Their depths grant no gameplay authority. Key direction comes from the same
palette helper as the scene light. Eye-path absorption and a low normal-incidence
reflection term preserve the shallow bottom, with stronger absorption in deep
water. The seabed carries sand, stones and a restrained animated daylight light
pattern; it shares the surface elapsed uniform and fades that pattern with depth.
Exposed swells reach a combined 0.22 world-unit amplitude before depth/shelter
attenuation. Broad sky glints and a directional sun highlight reveal their slopes.
These remain procedural shading choices without additional render targets.
Daylight uses a stronger directional key and lower ambient fill; the coastal
ground's shadow overlay deepens contact while the tactical grid retains its
separate presentation. Interior source lighting retains its own profile.
This is an original analytical-wave approximation,
not an FFT ocean or a buoyancy simulation.

The [coastal-water follow-up](plans/2026-09-07-coastal-water.md) records the
owner's shallow-water correction, inspected gaps and implementation proof needed.

The default camera now follows the accepted
[projection ruling](presentation-direction.md#projection-and-surface-ruling).
`camera.ts` preserves it through focus changes; pointing uses its actual matrix.
`view=dimetric` and `view=front` retain explicit geometry-exterior comparisons;
unknown names and alternate views with wall/roof runs or painted cards are
refused. Interior cutdown selects the south boundary. Foreground fading uses
the actual camera matrix. View-facing cards now face zero yaw. Old painted calibration
assets remain mechanical proof inputs, with their baked perspective visibly
obsolete; they are not accepted masters under this camera.

### House atmosphere

`structureAtmosphere.ts` owns the explicit GLB extras contract. Material
`tme_practical` carries `periods` and bounded emission `intensity`; node
`tme_light` adds bounded light `distance`; node `tme_smoke` carries bounded
`rise` and `radius`. Periods are unique members of day/dusk/night. Decoding
validates them even when inactive. The authored GLB owns anchors and schedules;
mesh names and building coordinates never infer them. Missing tags mean ordinary
static geometry. Runtime emission clones belong to the space; cached materials
remain immutable. Smoke uses twelve soft procedural puffs per active chimney,
with shared wind direction, fading ascent and no collision or gameplay meaning.
Space disposal releases its clones, smoke buffers/materials and local lights.

`structureLighting.ts` resolves an optional node `tme_interior_lighting` from
only the current space's structure roots. Exactly one profile may own a room;
unknown fields, malformed values and competing owners are refused. Its
`indirect_intensity` is finite in 0–0.5 and `shadow_count` is an integer in 0–2.
`source_decay` is finite in 1–2 and `source_color` is an exact six-digit hex
colour. These author the room's fixture falloff and colour; the lower decay
bound permits a stylized, broader pool without adding an unanchored key.
`shadow_radius` is finite in 0–4 and `shadow_strength` in 0–1. They control the
selected fixture shadows' filter radius and darkness. All six fields are
required; the earlier four-field profile is refused. The current temple asset
and profile fixtures migrate together. Shadow tuning adds no lights or maps.
The profile removes the unanchored directional key and substitutes its bounded
indirect fill. Outdoor use is refused. Untagged rooms retain the general palette;
this is explicit authored lighting, not a mesh-name or location heuristic.
Practical anchors, schedules and output retain the existing `tme_light` owner.
The strongest active sources receive the profile's shadow budget, with stable
ordering for ties and 256-pixel cube faces. Inactive sources cannot consume it.
The temple candidate uses this profile; its fixture positions and tuning remain
in the external asset. [Presentation direction](presentation-direction.md#room-lighting-from-visible-sources)
owns the visual target; the [town execution record](plans/2026-09-06-town-buildout.md#temple-stonework-and-textile-iteration)
owns native results. Tests cover malformed and duplicate profiles, outdoor
refusal, active-source selection, anchor preservation, shadow tuning and light disposal.

The `day`, `dusk` and `night` presets exercise local lighting states; day wins
if several phases are supplied, then dusk, with night as default. They do not
implement an authoritative world clock or household simulation. The current
arrival study uses selective window schedules and only a subset of working
chimneys. Native evidence is recorded in the
[surface brief](plans/2026-09-05-first-land-surface.md#arrival-homes-and-selected-camera-owner-2026-09-05).


### Visible tactical grid

`space/tacticalGrid.ts` builds one depth-tested overlay over authored non-water
cells. Its vertices use the shared surface sampler, including level decks;
blocked standing cells are drawn too. Water and scenic space beyond the packet
receive no standing grid. Roofs, props and grass naturally occlude the marks.
The shader uses derivative-based line widths, quiet two-tone edges and short
corner emphasis. Ground texture shaders no longer darken every cell seam.

The walking presenter forwards its existing hover cell through a callback;
the grid strengthens locally and returns to its persistent baseline on leave.
It performs no route search and assigns no legal/illegal colours. Cursor and
selected-cell feedback retain their existing ownership. Grid geometry/material
lifetime belongs to the space's existing disposal path. The
[presentation ruling](presentation-direction.md#tile-assembly-ruling) owns the
experiment; native results are recorded in the surface brief.


## First-expedition presentation study

The earlier 3D playable study is retired. Its renderer, actor binding, receipt
and tests were removed in the [pixel-art cutover](plans/2026-09-08-pixel-art-transition.md).
The authored world, server authority and interaction controls remain active in
the shared world renderer. Historical implementation details remain in earlier execution
records; do not restore the removed query selector or 3D asset packet.
