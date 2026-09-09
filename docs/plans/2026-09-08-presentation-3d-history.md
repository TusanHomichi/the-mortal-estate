---
last_updated: 2026-09-08
revision: 1
status: History through the September 8 pixel-art decision; superseded as present visual authority.
public_safe: true
summary: Visual target, shared pixel grid, frontal upright readability, reusable class-art direction and fixed presentation rulings.
---

# Earlier presentation rulings

Historical snapshot. The owner selected pixel art as the primary direction on
September 8. Statements below describe earlier rulings and implementations;
[presentation direction](../presentation-direction.md) is the current authority.
Do not use this snapshot to resume 3D production or reopen the selected pixel look.

This document owns **what the world should look like** and the rules that govern
getting there. The client's architecture is
[client architecture](../client-architecture.md); what is currently implemented is
[browser client](../browser-client.md).

Read the target and the applicable ruling below. Superseded camera,
cutaway, and interface proposals remain historical evidence, not parallel
implementation choices. The [paused experiment](#paused-experiment-presenter-evidence-order)
has a separate stop line from the active browser feel work.

## Continuous visual review

Every visible game area remains subject to iteration: exterior travel, ground,
shorelines, water, vegetation, architecture, interiors, characters and interface.
The owner directed agents to inspect the actual view whenever working in or
passing through an area, rather than concentrating visual review on the latest
room. Fix a concrete defect in the active slice when bounded; otherwise retain
a screenshot, location, specific observation, owning boundary and acceptance
check in the relevant execution record. Placeholder status does not exempt an
area from scrutiny. This is standing review direction, not blanket artwork-master
acceptance or an instruction to indefinitely expand each deployment.

## The target

> **An animated, weathered tactical map rendered in chunky illustrated pixels.**

The target is a **playable world view**: not a cartographic poster, and not a
miniature 3D diorama photographed from above.

### The grammar

- a fixed orthographic, straight-on gameplay projection at an intimate local
  play scale, with the angle owned below and ordinary world-up geometry;
- tile-coherent terrain over authoritative logical cells, with restrained visible
  joints rather than a requirement for invisible pixel-perfect seams;
- broad intentional pixel clusters and large calm material fields rather than
  noisy fake-pixel detail;
- uneven, material-specific texture density: wear follows traffic, roots follow
  trees, stones explain structure or terrain, and detail yields around actors,
  routes, and interaction anchors;
- limited material and regional palettes with a strong value hierarchy;
- actors, enemies, doors, stairs, bridges, and interactables enlarged for
  gameplay readability;
- structures built from a few clear planes and strong silhouettes;
- selective outlines and contact shadows rather than a uniform black contour
  around everything;
- restrained environmental motion — water, smoke, embers, foliage, weather, idle
  weight shifts, and concise combat effects;
- information-dense UI that shares the world's art language without overwhelming
  the playfield.

### Warm architectural direction

**Owner direction, 2026-09-05.** After reviewing a generated guidance image
based on the current arrival scene, the owner selected a warmer, more stylized,
lived-in treatment and dispatched the arrival house and its immediate street
as the first implementation. Substantial timber joinery, chunky stone footings,
deep window and door reveals, softened slate edges and purposeful domestic
details establish the architecture. Wear gathers at thresholds and exposed
edges; vegetation gathers at foundations and the margins of maintained paths.
Practical window and lantern light supports the inhabited feel.

Shape and proportions must read at the selected gameplay camera. Roof coverage
should leave enough facade visible to identify the entrance and domestic
details. Broad, calm material fields and grouped vegetation retain the standing
texture discipline. The visible logical grid remains welcome through organic
ground treatment. This selects a direction and a bounded implementation study;
the generated image is guidance, not a production master or an in-engine result.
Native review still judges the implementation. The
[surface brief](../plans/2026-09-05-first-land-surface.md#stylized-arrival-house-study)
owns the local study's scope and evidence.

### Stylization and restrained charm

**Owner direction, 2026-09-06.** Push the whole visual language further toward
stylized, lived-in forms, with a small amount of cuteness. Strictly realistic
surface rendering is no longer the aesthetic target. This applies to
characters, architecture, furnishings, ground and trees together.

Use softly bevelled, substantial forms, gentle asymmetry, warm colour and clear
silhouettes. Show habitation through repairs, maintenance, folded cloth, stored
supplies and wear that follows use. Keep broad quiet material fields; additional
surface noise does not create life. Restrained charm can come from rounded pots,
chunky boots, imperfect joinery and expressive posture. Avoid infantile
proportions, exaggerated cartoon eyes, toy-like uniformity and decorative clutter
that obscures play. The fixed camera, visible tactical grid, physical ground
contact and serious narrative stakes retain their owners.

**Character study dispatched.** The owner requested better character models and
authorized trials using image-to-model tools, original Blender work, or both.
Start with one adventurer and judge silhouette, face, outfit and motion at the
ruled camera and native play size. A generated design is guidance; a generated
mesh still needs editable geometry, a usable rig and material/motion inspection.
The [live-character ruling](#live-characters) retains modular skinned equipment
and clip-based animation. Specific proportions and models remain candidates;
this direction accepts no new artwork master or provider subscription.

**Character guide selection.** After comparing two generated adventurers, the
owner preferred the first figure. Preserve its adult proportions, restrained
facial stylization, practical layered outfit and warmth from materials and use.
The second figure's stronger cartoon treatment overshot the intended charm.
Do not interpret the broader style ruling as requiring enlarged heads or short
limbs. The selected guidance identity and modeling trials belong to the
[character study](../plans/2026-09-06-character-study.md); model acceptance remains
separate from this guide preference.

### Asset sourcing order

**Owner direction, 2026-09-07.** Use the retained Quaternius collection first
when its assets match the selected visual direction. Check established CC0
libraries for remaining needs, and use Meshy generation or suitable community
models where existing assets do not supply the required shape or style.
Original Blender work, generated guidance and material treatment can adapt
these sources into a coherent result. Availability alone is no reason to adopt
an asset that misses the target.

Prefer useful silhouettes, proportions and construction over source rendering
style. Review candidates with the game's materials, lights, fixed camera and
native play size. Props, buildings, terrain and trees share the character's
warm, inhabited visual language. Keep editable sources and record changes so
an adapted asset remains reproducible.

The [public-boundary policy](../public-boundary-policy.md#kinds-of-material) owns admissibility
and per-asset provenance. Retain the licence that accompanied each actual
download, its source URL and content hashes externally; a site's present
branding does not establish the licence of every pack or community item.
The external source inventory is a research aid, never a build dependency or
an accepted-artwork catalogue. Candidate integration and native review remain
separate from artwork-master acceptance.

### Placeholder town interiors

**Owner ruling, 2026-09-06.** The current town interiors are placeholders.
Their models, furnishings, materials and lighting establish no accepted room
design. Generate interior guidance images, review their direction with the
owner, then iterate the modeled rooms toward that guidance at the fixed gameplay
camera. Keep the warm, inhabited architectural direction above.

The accepted geographic layouts and current service integration can continue
supporting gameplay and proof while appearance is replaced. Guide images grant
no geography change or artwork-master acceptance. The
[expedition record](../plans/2026-09-06-first-expedition.md) owns that implementation;
this section owns the interiors' visual status and subsequent design process.

**Temple direction selected, 2026-09-06.** The owner selected the first temple
guide's warm inhabited treatment and directed its modeled implementation.
Substantial joinery, dressed stone around the guarded descent, proper enclosed
lanterns, a linen-dressed altar, care supplies and a stocked balm counter are the
selected vocabulary. Simplify the guide's dense floor wear at gameplay scale.
The [town execution record](../plans/2026-09-06-town-buildout.md#interior-guidance-pass)
binds the guide identity and subsequent model proof. The selection accepts this
visual direction; the new model still needs native review and does not inherit
artwork-master acceptance. Other interiors retain placeholder status.

**Temple refinement, 2026-09-07.** Reimagine the interior with stronger
stylization and an older, worn appearance. Soft substantial forms, irregular
joinery, repairs and wear from repeated use should convey a maintained working
sanctuary in the rebuilt town. Give the room more space and a better arrangement:
the older female balm seller belongs farther back beside stocked shelves, with
her figure visible and approachable. Tomas is young, friendly and tired; he
should wander the hall instead of remaining in a fixed altar pose. Keep the
receiving area and routes to both residents and the dungeon descent clear.

The [town execution record](../plans/2026-09-06-town-buildout.md#temple-refinement-proposal)
owns the selected guidance, lore inspection and implementation evidence. The owner
then directed iteration toward this guide, including the requested larger or
better-arranged room. The resulting 7×8 layout is a bounded amendment of the
previously approved playable geography; it preserves the dungeon endpoints.
The [resident contract](../town-resident-contract.md) owns authoritative wandering
and service binding. The [browser owner](../browser-client.md#temple-resident-interaction-direction)
records direct interaction and native framing. Resident models, furnishings and
materials remain artwork candidates. This instruction accepts no artwork master.

### Building purpose from the street

**Owner direction, 2026-09-06.** Town buildings with a public or gameplay purpose
must communicate that purpose through their exteriors. Readability is judged
from the street at the fixed gameplay camera and normal play size. Roofs and
foreground surfaces must leave the identifying features and entrance legible.

Use the building's form, entrance treatment and visible activity to establish
its role. Signs and symbols can reinforce that reading. A service building
should not depend on a text label, a roof-hidden prop or a change of paint on a
generic house to become identifiable. Maintain the shared warm architectural
language while varying silhouettes, materials and signs of use to suit purpose.

**Proposed design vocabulary:** a workshop may show a working bay, tools and
materials; lodging may show a welcoming porch, a hanging sign and shared seating;
a temple may show a distinct sacred entrance, tended lamps and protective
stonework. These are visual candidates, not an accepted service roster or final
building designs. Review the exterior in daylight as well as after lamps are lit.

The [surface brief](../plans/2026-09-05-first-land-surface.md#town-temple)
owns the owner's specific temple and dungeon-entrance relationship. This section
owns how a building's decided role becomes legible; visual acceptance remains
with the owner.

### Projection and surface ruling

**Owner ruling, 2026-09-05:** the world uses a fixed orthographic camera
with **zero yaw and 45-degree elevation**, looking from positive world Z.
Standing geometry remains upright, without shear. The owner selected the
native straight-on arrival comparison after reviewing both front elevations.
Square logical cells project as axis-aligned rectangles: horizontal world X
stays horizontal on screen. The existing nine-row framing is retained; its
projection derivation is owned by [proportional scaling](#proportional-scaling).

This supersedes the August 31 dimetric selection (45-degree yaw, 30-degree
elevation), including its September 3 reaffirmation. The direction requires
entrances, roof clearance and attached details to read from the chosen view.
It accepts the camera, not finished geography, building masters, or a presenter.

Standing source geometry uses ordinary world up. Actors, walls, doors, posts,
props, trees, and monsters receive no camera-facing shear or shared northwest
lean in their three-dimensional construction. Facing rotates the source around
world up. The later [construction ruling](#construction-and-viewport) determines
which subjects remain runtime geometry and which are rendered into cards; it
does not introduce compensation into the source model.

An actor's feet/contact sit at the centre of its authoritative square ground
cell. A nearby wall edge or doorway threshold does not replace or move that
cell-centre anchor. Its rendered body may cover portions of several projected
cells, but visual coverage does not redefine occupancy.

The camera stays close enough that one actor, the immediate traversable ground,
and a nearby interaction anchor are readable at native play size. Geography
leaves the frame naturally, and routes may disappear behind structures or
terrain rather than being surveyed from above. Steam Deck native **1280 × 800**
is the minimum play-size review surface; an enlarged lower-resolution capture is
useful evidence but does not substitute for that judgment.

This ruling supersedes the 2026-08-30 screen-square high-oblique selection,
including its camera-facing compensation and northwest-rise experiments. It
changes the projection and projected-up facts, not the accepted calm-field
texture discipline, material hierarchy, palette direction, silhouette
priorities, visible-joint tolerance, relational-scale discipline, or production
rule.

The generated candidates also exposed a specific style failure: filling every
surface with curling, equally weighted pattern. That is not TME's pixel grammar.
Large quiet shapes establish material and navigation first. Smaller clusters add
wear, growth, breakage, or relief only where the surface calls for them. Repeated
swirls, decorative meanders, and uniform high-frequency texture are generation
defects to correct, not characteristics to preserve.

This camera ruling alone selected no engine, platform, or production tool.
The later [browser-first contract](../client-architecture.md#the-web-client) owns
the current client choice; production presenter and tool acceptance still
require the evidence gate below.

**Historical comparison, 2026-09-03:** a straight-overhead treatment with
a shared diagonal lean was rejected after native courtyard/interior review.
The September 5 selection retains ordinary world-up volume and supersedes
that comparison's dimetric control. The earlier lab remains historical evidence.

### Tile assembly ruling

**Owner ruling, 2026-08-30, retained under the 2026-08-31 projection change:** a
restrained material joint may remain visible between terrain cells. Adjacent
tiles must agree on apparent scale, material family, value hierarchy, lighting
convention, and their quiet edge band; cracks, stones, grass blades, and other
interior clusters do not have to continue pixel-for-pixel across the boundary.

The terrain cell is one square logical world cell, projected by the accepted
camera according to the projection ruling above. The former **64 × 64 screen-pixel cell**, its
one-pixel perimeter, and its 62 × 62 interior belonged to the superseded
screen-square projection and carry no authority into the current world. No
raster source resolution, projected cell bounds, native tile-edge length, or
joint width is accepted yet; those are representative-scene calibration at the
1280 × 800 minimum play surface. A comparison zoom chosen only to normalize two
experimental cameras is evidence, not a production scale value.

Taller props, facades, trees, roofs, and compound modules may span integer cell
footprints and extend above their ground anchors; they do not change the logical
terrain-cell contract. The joint's material colour may vary with the admitted
material family, but its accepted native treatment may not drift asset by asset
once calibration closes.

Building floors and wall runs use that same cell ruler. A wall run owns the tiles
it stands on: the strip is drawn on each tile's camera-facing edge, the tile is
never occupied, and a door tile is the crossing (owner ruling, 2026-09-02).
Roofs are exterior dressing on closed footprints and match the walls' motif.
Entering a door loads the building's dedicated interior space. A roof owns no
footprint or occupancy fact. Foreground visibility follows the surface-fading
ruling below; it does not expose or imply a building interior.

An actor — player or monster — remains anchored by its feet/contact point at the
centre of exactly one authoritative square ground cell. Its body is not clipped
to the projected diamond: ordinary world-up height and volumetric width may cover
neighboring screen cells where the silhouette needs them. Walls, doors, posts,
props, trees, and other elevated modules retain their own ground contacts and
world-space construction. Visual extent never changes occupancy, walkability,
targeting identity, or collision; those remain facts supplied to the renderer
seam rather than inferred from projected bounds.

**Foreground visibility (owner, 2026-09-06).** A wall, roof, prop or canopy
blocking the presented player fades smoothly toward 40% opacity and restores
when the player clears it. The surface remains recognizable and the actual
textured, animated character is visible through it. Select the offending surface
of the placed object; unrelated instances stay opaque. Layered leaves must not
accumulate opacity until they hide the character again. This supersedes the
candidate figure silhouette and wall-only fade. It changes no occupancy,
collision, targeting or authoritative visibility. The
[browser client](../browser-client.md#foreground-surface-fading) owns implementation
and proof; final visual acceptance remains the owner's.

**Visible grid experiment (owner, 2026-09-05).** The owner explicitly
welcomes a visible gameplay grid and dispatched a new treatment alongside
material refinement. Natural shoreline, relief and vegetation can retain their
shapes while the presentation makes logical cells readable. A quiet persistent
grid with clearer corner ticks and cursor-local emphasis is the first candidate.
Grid edges identify cells; movement availability and refusal remain owned by
the existing input feedback. They neither infer passability nor replace its
source. Terrain/structures should naturally occlude ground marks; the grid does
not paint over roofs. The exact colour, weight and emphasis are visual candidates.

Material joints may still belong to the art, but they need not carry the entire
gameplay grid. This amends the earlier art-joint-only treatment without changing
the logical cell ruler or granting new targeting rules. Source provenance and
native visual acceptance still apply to every material and grid candidate.

### Relative scale ruling

**Owner ruling, 2026-08-30, retained under the 2026-08-31 projection change:**
the logical terrain cell is the world's common visual ruler, not a box that makes
every asset the same size. Production assets are normalized to one shared
projection scale and displayed at their authored size. An arbitrary per-scene
multiplier may diagnose a candidate, but it is not a production scale decision
and cannot make unlike source assets coherent.

An adult human is the comparison baseline for visual-development review, not a
new gameplay unit. Doors, windows, storeys, furniture, graves, vegetation, and
other familiar cues must remain mutually believable around that baseline. A
building must read as a tiled, occupiable volume capable of containing its
intended occupants: its floor footprint, wall and corner runs, openings, and
always-on exterior roof must agree around the person. The dedicated interior's
footprint, wall height, and clear door opening establish its scale directly; a
detached whole-building card or exterior roof silhouette does not. A monster intended to
read as large must stand unambiguously taller and carry more silhouette mass than
the adult baseline when both share the same ground-contact line and display
scale. The same rule applies to every authored size relationship: the picture
must express the size the subject is supposed to have.

Presentation metadata therefore distinguishes facts that must not collapse into
one another: the instance's ground-anchor **cell**, the asset-local **contact
point** placed on that cell, the presentation-only ground/occlusion **footprint**,
the subject's **nominal physical height**, and its native projected screen
**vector and bounds**. The last two are not interchangeable: foreshortening can
make a physically taller subject occupy fewer vertical screen pixels. These are
art and ordering facts. State and animation variants of one subject preserve
their declared scale, contact, footprint, and projected world-up direction
unless the authored variant is explicitly a differently sized subject. Authoritative
occupancy, collision, walkability, interaction reach, and targeting remain
separate gameplay facts and are never derived from visual height or sprite
bounds.

Before a scale family can be accepted, a neutral native-1280 × 800 scale audit
must align representative subjects on their ground contacts with atmosphere and
perspective tricks disabled, then the same subjects must remain coherent in the
representative gameplay micro-scene. Exact actor heights, building dimensions,
monster categories, and footprint values remain authored/test-calibrated rather
than being invented by this relational ruling.

### The dead world

The dead world **should not default to generic blue-grey fog and skull
decoration.** It may be uncannily preserved, historically layered, quieter, or
more legible than the living world. Its visual identity should express memory and
unfinished continuity.

That is a direction, not a palette. The specifics are open.

## The production rule

**No visual production system earns scale until one representative gameplay
micro-scene is accepted at actual play size.**

That scene must prove, together and at once:

1. player and enemy readability;
2. terrain continuity;
3. structure scale;
4. interaction anchors;
5. UI framing;
6. living/dead correspondence; and
7. the ability to create a **second matching asset efficiently**.

The seventh is the one that gets skipped and the one that decides whether a
pipeline is real. A method that produces one beautiful scene and cannot produce
the next one at a sane cost has proven a picture, not a production system.

Acceptance is the owner's, at actual play size, on the real playfield. A capture
scaled up for a review is not acceptance.

**Renderer choice, model provider, and generation tool are implementation
details.** The visual grammar and the accepted masters are the authority. A tool
change does not reopen the grammar; the grammar changing is an owner decision.

## The current target-authority packet

**Owner ruling, 2026-08-26:** resume one exact target packet before Phase 10 S3
and before any presenter is allowed to define what the target was. This is the
visual authority for the bounded experiment in
[`plans/2026-08-26-nomos-presentation-adoption-experiment.md`](../plans/2026-08-26-nomos-presentation-adoption-experiment.md).
It does not select Nomos, a renderer, a platform, or a production pipeline.

The packet contains exactly:

1. a hero environment frame;
2. a normal gameplay frame at the actual camera distance;
3. actor scale and silhouette references;
4. a combat frame with overlapping actors;
5. one restrained effect;
6. low-light and dead-world treatment;
7. a UI-overlay frame;
8. a material and palette sheet;
9. an animation or motion timing strip; and
10. explicit invariants and explicit non-invariants.

Together, those views must make all seven parts of the production rule above
judgeable. The target is incomplete if it avoids the real playfield, the actual
camera distance, living/dead correspondence, UI framing, or the cost of making
the next matching thing.

The owner records exactly one target verdict: `accepted` or `rejected`.
Rejection ends the experiment before presenter selection. Acceptance approves
the target only.

**Owner verdict, 2026-08-27: `accepted`.** The owner reviewed all six visual
artifacts at their native 1672 × 941 size together with the explicit invariants
and non-invariants. The ten required parts are represented by the hero frame,
normal gameplay frame, actor-and-material sheet, combat-and-UI frame,
living/dead-and-effect comparison, motion-timing sheet, and the written
invariant ruling. Their exact accepted target manifest has SHA-256
`d6676c95918280587cf737b07162579eb69d0e3eeef43c40b229bc12efe53487`.
The manifest and generated sources remain content-addressed local evidence and
are not tracked build inputs or promoted runtime assets. This verdict clears P0
only: it accepts no representative presenter result, Nomos boundary, renderer,
engine, or platform.

The current [projection ruling](#projection-and-surface-ruling) supersedes
the packet's camera geometry and intervening projection experiments. Any screen-square high-oblique, true
isometric, or other camera visible in those digest-bound artifacts is explicit
non-authority. Their remaining palette, material, surface-density, silhouette,
motion, UI, and living/dead evidence stays useful where it does not depend on a
retired projection. The packet's prior P0 clearance is historical for that
bounded experiment; a production implementation of the accepted target
must still earn the representative micro-scene verdict under the current
production rule.

## Paused-experiment presenter evidence order

This order belongs only to the paused bounded experiment recorded in the
[experiment plan](../plans/2026-08-26-nomos-presentation-adoption-experiment.md).
It is not an active production order and dispatches no presenter work. Resuming
it requires the fresh current-target evidence and separate owner direction named
by that plan; a future presentation slice that does not resume that experiment
is not browser-first by inheritance.

The experiment's earlier browser/Godot comparison order is historical. The
September 5 owner retirement removes Godot as an implementation or proof target.
Any resumed experiment must use the current browser architecture and still meet
its independent acceptance gates.

The owner judges every candidate at actual play size against the accepted
target. A proceed verdict also requires a second matching asset or scene made by
someone other than the presenter implementation author, without compiler or
presenter source edits. The evidence records elapsed authoring time, iterations,
diagnostics, interventions, and changed files.

## The candidate lifecycle

**Separate pixel-temple study (owner, 2026-09-08).** The bounded 2D experiment
uses a textured room plate without regular painted floor tiles; the tactical
grid is a separate presentation layer aligned with movement and pointing.
Each room has one authoritative grid for every player, NPC and creature.
Visibility may hide cells and a camera may project them differently, but neither
changes cell identity, boundaries or occupancy. No observer-specific art grid is
permitted.
Characters should have more adult proportions and understated faces. The cutesy
3D treatment partly addressed model limitations and does not bind this pixel
study. Keep expressive posture, worn clothing and rich colour. The owner also
requested finer character resolution to match the room's material detail;
coarse sprite blocks and Tomas's remaining cartoon face were rejected.
The temple's religious imagery uses the open hand. The owner explicitly rejected
Christian crosses and church-like vestments: use the faith's own receiving-hand
emblem and modest working robes. Tomas remains young, earnest and tired, with
adult proportions. The artwork must not invent a divine name, pantheon or origin
to explain its symbols. This selects no
production renderer or artwork master. The [pixel-temple record](../plans/2026-09-08-pixel-temple.md)
owns implementation and native review evidence. After inspecting an elevated
traveler candidate, the owner preferred the earlier frontal presentation:
readable faces, clothing and creature silhouettes take priority over matching
the room's camera angle. Retain the earlier lean adult art and palette; improve
walking without elevated-camera foreshortening or stockier proportions. This
preference applies consistently to every upright asset: characters, creatures,
trees, buildings, doors, statues and upright props. Preserve readable height and
front-facing silhouettes; trees show their trunks and upright crowns, buildings
show their facades, windows and entrances. Ground surfaces and selected top
planes convey depth. Orthographic projection and asset-facing angle are distinct;
upright assets may cheat their facing consistently for readability. Feet, roots
and bases keep their shared grid anchors. This does not change room geometry.
The owner next directed reusable class artwork before further generic-traveler
polish, beginning with the unarmed class. Establish its visual identity before
expanding animation; the existing traveler is only a possible thief candidate.
Class display-name discussions follow [classes and training](../class-training-contract.md).
No class concept becomes an accepted master through generation alone.


Visual work distinguishes five states, and the cost belongs at the end:

| State | What it is |
| --- | --- |
| disposable experiment | thrown away by default; no ceremony at all |
| review candidate | offered for judgment |
| owner-accepted editable master | the authority for what a thing looks like |
| deterministic runtime export | what the game actually loads |
| promoted, verified asset | provenance sealed, licensing recorded, certified |

**Expensive certification, provenance sealing, and full verification belong at
promotion.** They must not block ordinary taste iteration — moving a fence,
lowering a roof, trying a palette. A pipeline that makes every experiment
expensive produces fewer experiments, and taste is a volume problem.

Provenance and licensing for every promoted asset are mandatory and are owned by
[public boundary policy](../public-boundary-policy.md).

### Generated-source isolation and alpha validation

Generated source assets enter the candidate lifecycle **one isolated subject per
generation request**. A request for a character, plant, prop, or other reusable
piece asks for one complete silhouette with transparent padding and genuine PNG
alpha. It explicitly excludes a backdrop, floor, border, checker pattern, glow,
halo, environmental shadow, and coloured matte. Multi-subject sheets, grids,
quadrants, and atlases are assembled deterministically from separately validated
pieces; they are not requested as the generated source. This keeps generation
focused on the subject and prevents a sheet-layout convention from being drawn
as part of the image.

Prompt language is a request, not evidence. Before any generated result can
become a review candidate, a mechanical inspection must prove that the untouched
file is an RGBA image whose alpha includes both fully transparent background and
non-transparent subject pixels. The candidate is then composited over at least
one light and one dark contrasting field at source scale and at native gameplay
scale to expose matte fringe, halos, clipped fine detail, or an opaque background
disguised as transparency. A filename, preview checker, or claim by the generator
does not satisfy this check.

If native alpha is absent, the result stays disposable. A focused background
extraction retry or deterministic matte-cleanup pass creates a **new** candidate;
it never rewrites the untouched generated source or inherits acceptance from it.
The prompt, meaningful project-owned inputs, untouched output, transformations,
and tool identity remain external evidence until promotion. Nothing enters the
tree merely because it passed the alpha check: visual acceptance still belongs
to the owner, and promotion still requires the provenance and licensing record
owned by [public boundary policy](../public-boundary-policy.md#generated-content).

This rule validates a cheap way to produce and judge reusable transparent source
pieces. It does **not** accept a particular model, provider, subscription, asset,
atlas packer, renderer, or end-to-end production pipeline. Those still earn scale
only through the representative micro-scene and second-matching-asset proof.

## Retired pipelines carry no authority

Earlier presentation experiments — a native 3D presentation scaffold, an
image-to-3D asset pipeline, a painterly repaint chain over generated atlases, and
a multi-stage generation conductor — are **retained privately as lessons and
evidence only.** They receive no automatic authority here, and none of their
outputs is in this tree.

Re-entry for any of their outputs is through the production rule above — an
accepted representative micro-scene at actual play size — and then the promotion
path. **Never through a migration step that copies a file across**, and never
because a previous project got a good result from it once.

This applies to the technique as much as to the artefact. "We already solved this"
is a claim that has to survive the same micro-scene gate as any new proposal.

## What is implemented today

[Browser client](../browser-client.md) owns the current Three.js candidate scene.
It is not an accepted production presenter or asset set. Godot was retired by
owner direction on September 5. Server wire observation remains covered by
Python proof; authoritative browser integration and fresh identity-addressed
Workbench capture remain unfinished.

## Who decides

The owner is the final authority for taste, visual acceptance, and the accepted
masters. Agents may present evidence, candidates, and recommendations; **they do
not manufacture acceptance through automated consensus**, and an autonomous studio
approving its own art is an explicit project non-goal.

## The in-engine feel scene

**Owner direction, 2026-09-01:** after the lab's constructed-scene experiments
(one deterministic compositor assembling separately generated, alpha-validated
components on the ruled grid; then a member-level wall kit whose every shared
edge is drawn by geometry rather than by the generator), the owner directed the
next evidence step into the engine: a real 3D scene under the ruled camera, so
lighting, foliage motion, weather, and time of day can be felt rather than
inferred from stills.

The scene began in `client/presentation/feel/`. The September 2 browser-first
ruling moved active feel work to `web/`; the September 5 ruling retired Godot ([client architecture](../client-architecture.md#the-web-client)). This is a
bounded experiment beside the world-view seam: it consumes no authoritative
frame, emits no authoritative targets, and sends no server command. Promotion
behind the production seam still requires the production rule above.

### Movement and readiness

The September 5 owner direction supersedes shared-beat landing. Each accepted
browser move receives a fresh three-second cooldown and locks out replacement
movement until that cooldown expires. The owner subsequently clarified that the
figure may reach its destination before the cooldown ends, but must not arrive
late. Early visual arrival cannot release the movement lock: the UI must make
the remaining cooldown obvious. Travel animation and action readiness are
separate presentation concerns; the figure follows its committed route. [Browser client](../browser-client.md#movement-and-availability)
owns current behavior; the [timing ruling](../boundary-map.md#21-authoritative-individual-deadlines-d5)
owns server authority and individual deadlines. Route allowance remains
experiment tuning under D2. Foreground visibility follows the surface-fading
ruling above.

The owner's subsequent September 5 UI direction requires action availability to
be immediately understandable. Ready, cooling down, and unreachable must have
legible, distinct cursor marks; colour and shape both carry their meaning. The
owner then clarified the surface: the cursor alone supplies visual movement
availability. No ready notices, cooldown countdowns, or status panel appear over
the world; the lower area belongs to the future chrome. The status-strip trial
is superseded. An invalid target must not conceal an active cooldown.

The same follow-up asks routes to search around obstacles when a legal route is
available. This does not itself change the movement allowance; current local
routing and its limits are owned by client notes.

### Construction and viewport

Its construction follows the lab's conclusions rather than re-deriving them:
the camera follows the [projection ruling](#projection-and-surface-ruling), with the
frame governed by [proportional scaling](#proportional-scaling); ground cells carry material swatches in continuous world-plane coordinates;
the local coastal study adds presentation relief under the browser contract; walls are real geometry built from a member profile in world
units (plinth, sill, plaster, posts at every cell boundary, braces, lintel and
door, cap front and cap top) so courses never seam; standing subjects other
than characters are world-up billboards anchored at their cell centres at
their nominal heights (characters are live rigs since 2026-09-03, below);
and the scene's only warmth comes from actual light sources — the lantern and
candles — as the owner ruled in the lab.

**Viewport ruling (owner, 2026-09-02).** The frame was ruled at 6.31 world
units tall at the 1280 × 800 minimum play surface — one comparison step out
from the lab's 5.05-unit frame, which is superseded — so the whole closed
footprint of a building, roof included, sits in frame and the player stands
about a fifth of the frame height. That height is itself superseded as a
ruled value by the cell-count ruling of 2026-09-03, below, which derives the
frame from the number of cells it shows; the intent this ruling records — the
footprint in frame, the player about a fifth — stands. The ruling was made against painted full-frame references
of the courtyard and the ground room (lab, review-candidate tier) in which the
player stands about a sixth of the frame height; a further step out is
expected once cards are re-baked at a higher density, and the client's `zoom`
comparison step stays for that judgment. Rulings made beside it: **prop scale
is one metre to one world unit** for three-dimensional source models, whether
rendered to cards or placed as live characters, superseding the lab's 0.79 mapping, because metre-true furniture
read as dollhouse scale beside the references; **the wall profile stays** and
the roof gains its missing weight — steeper pitch, deeper eave, a darker and
larger-scaled shingle swatch; and the two painted references are accepted as
placement and scale targets at the review-candidate tier only, never as
masters. Masters come from this project's own build once it matches them.

### Interior camera

**Owner ruling, 2026-09-02.** Outdoors the camera is the
player's: centred on the caretaker and re-centred on each landing. Inside a
building the camera belongs to the space: it is centred on the room and
stops following the character, so a room that fits the frame is seen whole,
as the painted references frame it. It follows the player again on the way
out. The rule keys on the space, not the player's position: a dedicated
interior is the space that carries no weather.

### Structure and cards

Anything that belongs to a building — a hearth, a doorway, a window, a shelf —
is built from material swatches over math like the walls, because it is seen at
an angle and must have a side; free-standing things — a tree, a table, a
stone — are painted cards at the camera angle, placed clear of walls, and a
flat wall-plane card is for genuinely flat things only. A person was a card
under this ruling and is not since 2026-09-03 (below). Characters were ruled on
2026-09-02 as modelled and rigged in 3D but shipped as cards rendered at the
ruled camera angle; that ruling is superseded below.

### Live characters

**Owner ruling, 2026-09-03.** Characters are **live rigged
meshes in the client**, not cards: the rig stands in the scene under the
ruled camera and the scene's own lights and shadows, its equipment is skinned
parts on the shared skeleton, and its motion is animation clips played on the
rig. The painted grammar the cards carry is reproduced in the character's
material: the lab showed the treated-card palette applied as a nearest-colour
lookup in the fragment shader, with a rim darkening, puts a dressed live
figure beside the treated furniture at play size without announcing a
different medium (lab `camera-v36`, rounds three and four, 2026-09-03). The
same ruling extends to **any subject class that cannot read as a card** — the
owner's words: anything that has to be 3D to not look stupid — judged per
class at play size through the production rule. The hearth is the precedent:
as a flat card it read wrong, it became fixture geometry, and nothing since
has failed the same test, so the live class list today is characters alone.
Everything else stays a card or structural geometry as ruled above. What this decides: the medium for
characters, that equipment is modular parts, and that figures animate by
clip. What stays open: the exact in-shader treatment (the lab's palette
lookup and rim are the candidate; the cards' block grain is optional), clips
beyond the subsequently decided idle and three gaits, which further classes
cross, and the character source pipeline itself.
[Browser client](../browser-client.md#movement-and-availability) records
the implemented clips and the current movement experiment.

**Tree placement variety (owner direction, 2026-09-05).** When authoring the first
land, do not place identical tree cards beside one another. Mix distinct
silhouettes and sizes, distribute repeated assets across the landscape, and
review the composition from the actual game camera. Mirroring or resizing the
same card alone does not establish enough variety. This is an authoring
requirement; it does not accept the current candidate tree set as a master.

Vegetation cards take motion from one world-space wind field, phase from their
position rather than a shared loop, and hold the root while art-derived canopy
weight lets leaves move apart from trunk; grass follows the same rooted rule
(owner ruling, 2026-09-02).

#### Settling into an occupied square

**Owner direction, 2026-09-07.** Players and NPCs should visibly inhabit the
square they occupy. Idle behavior responds to whether they are alone or sharing
it, how long they have remained there, and the immediate interaction context.
A newly arrived character can settle before adopting a longer relaxed idle;
characters should not all repeat one synchronized standing loop.

Variety should communicate presence and personality through suitable weight
shifts, stretches, attention changes, gear tending, or more settled poses where
the surroundings support them. These are illustrative animation possibilities,
not a required clip list. The owner explicitly included sitting down after a
long quiet idle, with a natural sit-down and stand-up transition when movement
or engagement resumes. This is intended behavior where the local space and
context allow it, including an appropriate ground-seated pose; it does not
require every square to contain a chair. Movement and interactions should
interrupt or change the idle naturally. In-cell gestures and offsets must
preserve readable square membership and leave room for other visible occupants.

This is standing presentation direction for both players and NPCs, not an
implemented system or a new exclusive-occupancy rule. Authoritative positions,
occupancy legality, action readiness and service requirements retain their
existing owners. Exact dwell thresholds, transition rules and clip repertoires
remain implementation and visual-review work. The current basic idle and
shared-square offsets do not fulfill this direction by themselves.

#### NPC attention and room activities

**Owner direction, 2026-09-07.** NPC idle presence includes looking around,
turning toward people or activity, and carrying out small tasks in their room.
They should not spend every idle facing straight ahead. Tomas may check the cot
and straighten its blanket; Maude may stir a preparation, wipe her hands and
return to her service position. These are preferred first examples, not a claim
that the corresponding animation clips or activity scheduler exist.

Start with a few deliberate, character-specific routines. Each should have a
credible approach, contact pose, short activity and return to attention. Author
reachable prop-contact anchors and fitted animations so hands meet the bowl or
blanket, feet stay planted during the task, and cloth or held tools respond where
needed. Head and torso attention can vary without requiring a full chore.
Player interaction should interrupt or finish the brief gesture naturally;
visual busywork must not introduce a new wait to receive an otherwise legal
service. The [resident contract](../town-resident-contract.md#movement-and-attention)
continues to own movement and attention, and its service rules still apply.
Travel between cells must use authoritative movement; the client cannot invent
a chore route or relocate a provider through visual offsets. This is recorded
direction for a subsequent implementation, not behavior delivered by room art.

### Chrome and actions

The September 3 rulings below give the current target. The accepted layout
supersedes the earlier action-tile row, unboxed world log, freely scattered bag,
and column sequence; proportional scaling supersedes the earlier claim that a
larger screen shows more world. Those are closed revisions, not alternatives.

**Shared direction (owner, 2026-09-03).** The screen is one built object: no
floating windows or panels and no glossy overlay. The chrome belongs to the
estate's materials and practical light, with serif ink for type; the accepted
layout was judged in a blackened-iron treatment, whose assets are still
candidates. The game is mouse driven, with no mode button. The route preview
and its pace word live on the world. The three gauges are **health, stamina,
and mana**, left to right; a resource the character lacks is shown empty. Bag
item pictures are a separate asset class from scene cards. The hands are
recessed slots; gauges have numeric readouts, and a door mark is the leave
affordance. The six-tab region covers worn equipment, rings, the sack,
inspection, nearby things, and the spell-book/preparation view.

The initial structure came from the owner's reviewed reference screens,
recorded in this project's words; the later lab plate supplied the accepted
layout below.

**Action rulings (owner, 2026-09-03).** There is no row of action buttons.
Attacking is a double-click on the creature with whatever is in hand. The
spell book is one of the column's tabs; the player drags up to **three**
spells from it into three readied slots at the foot of the column, and those
three are the only shortcuts the chrome carries. Double-clicking a readied
spell warms it and turns the pointer into a crosshair; clicking the creature
casts the warmed spell at it — the warmed-spell and readiness-is-the-cursor
rules already recorded, now with their surface. Effects active on the player
are shown as small spell icons floating in the play view's top-left, icons
and not frames, and nothing else floats over the world.

### Accepted chrome layout

**Owner ruling, 2026-09-03.** The layout is ruled from a
painted plate in the lab, in the blackened-iron hand the owner chose. Its exact
1920 × 1080 design-space play aperture is 1462 × 753 at `(33,21)`, recovered
from the sliced plate manifest when the owner recalled the design on September 7.
Those are design coordinates; proportional scaling below owns display behavior. The
play view takes about three quarters of the width and seven tenths of the
height at 1920 × 1080; the column on the right is one fifth of the width and
holds, top to bottom, the bag, the six tabs, the two hands, the three vials,
and the three readied-spell slots, compactly; the strip along the bottom is
two panels — the left one every social channel as tabs (area, group, guild,
and the rest), the right one combat and server messages only, each with its
own input trough. The bag's slots are an invisible, compact grid that items
snap to, sized so that a full bag holds many things, never a loose scatter.
A group readout exists only while the player is in a group and appears then
as its own forged plate at the right end of the strip; solo, it is absent.
The window this leaves is wider than the ruled 1280 and some fifty pixels
shorter than its 800, which the owner accepts. What remains open is not the
layout but its assets: the slices, a nine-patch for every panel that
stretches, this project's own engraved glyph set, and the accessibility
floor's collapse.

### Proportional scaling

**Owner ruling, 2026-09-03.** Every player sees the
same extent of world in the play view whatever their screen resolution, and
the interface scales with it: the screen — chrome and window together — is
designed once at 1920 × 1080 and scaled uniformly to the display; at 4K the
tiles are simply larger. This supersedes the fixed pixel density: the
179-pixel cell diamond of the viewport ruling becomes a value derived at the
design size, not a ruled one, and a larger screen never shows more world.
The reason is tactical fairness — sight range must not be bought with a
monitor — and the reference game's fixed seven-by-seven view. The frame
height is derived from the visible cell count, ruled below. Four rules keep it from looking like a
stretched bitmap: the world is rendered at the display's real pixels with a
fixed world extent, never as a scaled image; the chrome is authored at twice
the design size and only ever scaled down; text, numbers, gauge levels, and
health lines are drawn live by the client, never baked into art; and a
display that is not 16:9 gives its spare space to the parts that want it —
spare height to the bag and the chat through nine-patch edges, spare width
to the side panels ruled below — while the window never moves. The owner's
direction is that spare width expands the interface rather than filling it
with stone; the ruling below says with what.

**Visible cell count and ultrawide ruling (owner, 2026-09-03; projection
amended 2026-09-05).** The play view is **nine ground-cell rows tall**.
At the accepted elevation a row projects to sin 45° ≈ 0.707 world units
vertically, giving **≈ 6.36 world units** over the full play view height.
A cell's pixel height is the view height divided by nine; its width is that
height divided by sin 45°. This preserves the preceding comparison's frame
height while replacing its diamond geometry. Actor dimensions remain authored
world units, with their screen extent derived from the camera. The earlier
6.31-unit frame and seven-by-seven alternative remain superseded.
On a display wider than 16:9 the play view
keeps that extent and stays where the layout puts it; the spare width becomes
**a forged side panel on each flank**, and the group readout and the
active-effect icons move out of the play view into them. The chat does not
widen; the panels take the width. At 16:9 and 16:10 nothing changes: the
group readout stays a plate at the strip's right end while grouped, and the
effect icons stay top-left of the play view. The bag and the chat take spare
height only, and the play view never widens — sight range is not bought with
a monitor. The side panels'
pieces are not yet drawn; they are assets, and open with the rest.

### Candidate assets

The scene's art is candidate material from the lab, and none of it is tracked.
Assets enter through a digest-bound manifest from `TME_FEEL_ASSETS`; missing
inputs and digest mismatches are refused. A missing packet is shown as absence
inside the picture, never replaced with a private default. Tracked synthetic
fixtures prove the loader and geometry without granting content authority.

[Browser client](../browser-client.md#operation-and-proof) owns the browser
commands, packet-serving boundary, presets, comparison zoom, and proof inputs.
Retired experiment environment controls do not configure the browser.

Nothing this scene renders is an accepted master. Acceptance of any of it is
the owner's, at play size, through the production rule.

## Open

Deliberately unresolved, and not to be resolved by implementation:

- the owner verdict on the representative micro-scene;
- the exact native tile-edge length, joint width, and the actors' authored
  world-unit dimensions within the current projection ruling
  and framing (the orthographic frame's size is not open: it derives from the
  ruled nine cells; the camera's distance along its view direction sets
  nothing but clipping);
- the ultrawide side panels' pieces — the treatment is ruled, the assets
  are not drawn;
- the exact palette discipline and animation budget within the ruled pixel and
  surface grammar;
- the dead world's visual identity beyond the direction above;
- the UI's typography and contrast palette, and the chrome's assets and
  glyph set (the layout is ruled; the painted plate it came from is a lab
  candidate, not a master)
  ([client architecture](../client-architecture.md#input-and-the-accessibility-floor)
  owns the accessibility floor those must clear);
- which production tools and presenter earn a place, which is decided by the
  production rule and the bounded evidence plan, not in advance.

### Temple material and framing experiment

The early September 7 candidate used a brighter warm indoor fill and front-side
key. The later [source-lighting ruling](#room-lighting-from-visible-sources)
supersedes that treatment. Live figure palette grading interpolates between
the two nearest colours; hard nearest-colour boundaries made detailed material
maps speckle at native size. The camera retains zero yaw, 45-degree elevation
and the ruled nine ground-cell rows. Automatic room-fitting zoom was withdrawn
after the owner recalled the accepted UI design; larger interiors do not grant
a different scale. Native visual acceptance remains open.

The next art pass follows the same guide with substantial pegged furniture,
inset cupboard doors, varied glazed care vessels, an open work ledger, folded
and draped textiles, and restrained plants. Worn limestone, oak, limewash and
faded red cloth use an illustrated material sheet. Floor joints retain the
gameplay grid. Contact shading gives props weight without changing the light
or camera contract. The [town execution record](../plans/2026-09-06-town-buildout.md#temple-furniture-and-material-iteration)
owns this candidate's implementation and verification receipts.

The subsequent stairwell pass makes the dungeon entrance read as a descent:
joined stone courses and worn coping define the floor opening, broad tread noses
show foot wear, and a buried doorway recedes into darkness. A small low lantern
initially lit the upper flight; the owner subsequently directed it farther down
to reveal the descent, distinct from the room's west-wall lantern. Visual depth does not alter the authored traversal
link or the player's standing cells. The [stairwell execution record](../plans/2026-09-06-town-buildout.md#temple-stairwell-iteration)
owns this candidate's source and proof receipts.

The next guide-matching pass concentrates wear in readable shapes: staggered,
clipped footing stones, recessed rubble behind actual openings in the plaster,
slightly hewn timber edges, and softly stuffed bedding beneath rumpled cloth.
Lighter limestone and varied floor texture orientation keep the square grid
while reducing the repeated slab appearance. Existing materials are reused;
added noise is not the visual target. The [surface-wear execution record](../plans/2026-09-06-town-buildout.md#temple-stonework-and-textile-iteration)
owns this candidate's implementation and proof.

The subsequent daily-use pass follows travel and work: floor scuffs belong on
frequent routes, dirt gathers beside walls, and a few local repairs interrupt
the regular slabs. Open supplies, an interrupted ledger, used linen, spare
blankets and uneven seating should suggest ongoing care. Keep clear standing
cells clear; props belong on furniture or inside existing occupied footprints.
The [daily-use execution record](../plans/2026-09-06-town-buildout.md#temple-patterns-of-daily-use)
owns the implementation and native proof. Shadow edges should be gentle enough
that standing beside a lantern does not turn a character into a black cutout.

### Room lighting from visible sources

The owner rejected the temple's uniformly lit appearance on September 7.
Lanterns and candles should produce local pools of light with believable falloff;
their placement should explain the illumination. Use restrained indirect fill
to preserve faces, movement feedback and grid readability. Corners and the
dungeon threshold may recede into darkness. A room-wide directional key must
not flatten the temple. The stair lantern belongs farther down the flight so
it guides the eye toward the dungeon while the west-wall lantern lights the room.
The [browser contract](../browser-client.md#house-atmosphere) owns implementation;
source strength, shadow tuning and visual acceptance remain candidate.

### Coastal water by depth and exposure

The owner rejected the current exterior water as short of the selected visual
reference on September 7. Shallows should reveal the bottom and carry smaller,
quieter movement, blending gradually into deeper, more active open water.
Sheltered water and exposed coast should be distinguishable. Preserve the
standing camera, geographic footprint and authoritative movement semantics.
The [coastal-water follow-up](../plans/2026-09-07-coastal-water.md) owns research,
current implementation gaps, the proposed bounded experiment and its proof.
