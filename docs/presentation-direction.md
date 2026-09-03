---
last_updated: 2026-09-03
revision: 25
status: Owner-accepted visual target. Revision 23 records the viewport frame ruling, the prop scale ruling, the roof-weight direction, and the painted scene references as review candidates; revision 24 records the interior camera ruling; revision 25 records the chrome rulings — the screen's structure, its materials, the three gauges, and the log on the world; no renderer, asset set, or production pipeline is accepted.
public_safe: true
summary: The owner-accepted orthographic 2:1 dimetric target with visible-joint tile and relative-scale grammar, ordinary world-up structural geometry, card-based things, rooted shared-field vegetation wind, calm-field texture discipline, dedicated interiors, always-on exterior roofs, and the production rule and bounded-experiment context used to judge future presentation evidence.
routes:
  - web/**
  - client/presentation/**
  - content/test-corpus/**
---

# Presentation direction

This document owns **what the world should look like** and the rules that govern
getting there. The client's architecture is
[client architecture](client-architecture.md); what is currently implemented is
[client notes](client-notes.md).

Earlier revisions established the charter-level direction, the bounded target
packet, its evidence order, the first owner-selected projection, and the
surface-pattern correction found while calibrating it. Revision 5 records the
owner's exact packet acceptance. Revision 6 records the then-current
high-oblique projection and screen-cell contract, both now superseded by
Revision 12. Revision 7 records the relational scale discipline that survives
that replacement and makes logical cells a common ruler rather than a common
object size.
Revision 8 recorded the now-superseded roof-off consequence; its cell-ruler
conclusion survives, so footprint, wall, opening, and roof modules — not a
whole-building card — establish scale. Revision 9 records the projection consequence: every elevated subject
uses one shared diagonal screen direction for height while the tile plane remains
screen-aligned. Revision 10 makes the structural construction consequence
explicit: base runs remain H/V and corresponding elevated vertices use that
shared direction. Revision 11 records the generated-source isolation and alpha
validation rule proven during disposable visual iteration. Revision 12 records
the owner's selection of the 2:1 dimetric challenger after native-size real-3D
projection and matched production-art comparisons. Revision 13 scopes the old
browser-first order to its paused experiment and removes the now-settled camera
pitch from the open calibration list. The visual target is still the owner's to
set, and this document is where the owner's version is recorded — not where an
agent manufactures acceptance.

## The target

> **An animated, weathered tactical map rendered in chunky illustrated pixels.**

The target is a **playable world view**: not a cartographic poster, and not a
miniature 3D diorama photographed from above.

### The grammar

- a fixed orthographic **2:1 dimetric** gameplay projection at an intimate,
  local play scale: square world cells project as diamonds, both perpendicular
  wall faces remain readable, and standing geometry uses ordinary world up;
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

### Projection and surface ruling

**Owner ruling, 2026-08-31:** the world uses a fixed orthographic **2:1
dimetric** gameplay projection. The camera has no perspective convergence, uses
a 45-degree yaw around world up, and looks down at a 30-degree elevation. A
square logical ground cell therefore projects as a diamond whose full screen
width is twice its full screen height. The choice is dimetric rather than true
isometric: the shallower camera presents actors and vertical structures more
frontally while retaining an orthographic 3D world.

Standing geometry uses ordinary world up. Actors, walls, doors, posts, props,
trees, and monsters receive no camera-facing shear and no shared northwest lean.
Their volumetric construction, animation, lighting, self-occlusion, and shadows
remain coherent in world space. Facing rotates a subject around world up; it does
not change that subject's up direction or turn it into a billboard.

An actor's feet/contact sit at the centre of its authoritative square ground
cell. A nearby wall edge or doorway threshold does not replace or move that
cell-centre anchor. Its rendered body may cover portions of several projected
diamonds, but visual coverage does not redefine occupancy.

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

This ruling selects the packet's visual projection. It does **not** select a
renderer, engine, platform, logical-to-screen implementation, or production
tool. Those remain downstream evidence questions.

### Tile assembly ruling

**Owner ruling, 2026-08-30, retained under the 2026-08-31 projection change:** a
restrained material joint may remain visible between terrain cells. Adjacent
tiles must agree on apparent scale, material family, value hierarchy, lighting
convention, and their quiet edge band; cracks, stones, grass blades, and other
interior clusters do not have to continue pixel-for-pixel across the boundary.

The terrain cell is one square logical world cell, projected by the accepted
camera as a 2:1 screen diamond. The former **64 × 64 screen-pixel cell**, its
one-pixel perimeter, and its 62 × 62 interior belonged to the superseded
screen-square projection and carry no authority into the dimetric world. No
raster source resolution, projected diamond bounds, native tile-edge length, or
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
Roofs are always-on exterior dressing on closed footprints and match the walls'
motif. Entering a door loads the building's dedicated interior space; no
roof-off cutaway or interior occlusion logic exists, and a roof owns no
footprint, occupancy, or sorting fact.

An actor — player or monster — remains anchored by its feet/contact point at the
centre of exactly one authoritative square ground cell. Its body is not clipped
to the projected diamond: ordinary world-up height and volumetric width may cover
neighboring screen cells where the silhouette needs them. Walls, doors, posts,
props, trees, and other elevated modules retain their own ground contacts and
world-space construction. Visual extent never changes occupancy, walkability,
targeting identity, or collision; those remain facts supplied to the renderer
seam rather than inferred from projected bounds.

The joint belongs to the world art. It is not a debug grid, editor overlay,
selection cue, or substitute for authoritative targeting. This tolerance allows
independently authored or generated tile candidates to enter the ordinary
candidate lifecycle when they are visually coherent after assembly; it does not
promote a generator output, waive provenance, or let a tile bypass owner visual
acceptance.

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
[`plans/2026-08-26-nomos-presentation-adoption-experiment.md`](plans/2026-08-26-nomos-presentation-adoption-experiment.md).
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

The 2026-08-31 dimetric ruling supersedes the packet's camera geometry and every
intervening projection experiment. Any screen-square high-oblique, true
isometric, or other camera visible in those digest-bound artifacts is explicit
non-authority. Their remaining palette, material, surface-density, silhouette,
motion, UI, and living/dead evidence stays useful where it does not depend on a
retired projection. The packet's prior P0 clearance is historical for that
bounded experiment; a production implementation of the accepted dimetric target
must still earn the representative micro-scene verdict under the current
production rule.

## Paused-experiment presenter evidence order

This order belongs only to the paused bounded experiment recorded in the
[experiment plan](plans/2026-08-26-nomos-presentation-adoption-experiment.md).
It is not an active production order and dispatches no presenter work. Resuming
it requires the fresh current-target evidence and separate owner direction named
by that plan; a future presentation slice that does not resume that experiment
is not browser-first by inheritance.

The browser/WebGL presenter is tested first because the authorized upstream
already carries that evidence path. This is an economy of evidence, not a
platform or feel-surface ruling. The current Godot client remains the standing
client and feel surface.

A Godot candidate is required only when the browser result leaves a named,
consequential uncertainty about feel, accessibility, desktop integration,
performance, deployment, or production cost. If tested, it consumes the same
recorded TME facts and the same admitted, pinned Nomos artifacts. It does not
receive a second gameplay interpretation.

The owner judges every candidate at actual play size against the accepted
target. A proceed verdict also requires a second matching asset or scene made by
someone other than the presenter implementation author, without compiler or
presenter source edits. The evidence records elapsed authoring time, iterations,
diagnostics, interventions, and changed files.

## The candidate lifecycle

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
[public boundary policy](public-boundary-policy.md).

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
owned by [public boundary policy](public-boundary-policy.md#generated-content).

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

Nothing that is art. The current world view is a **diagnostic lattice**: flat
colours, one rectangle per visible square, markers for addressable things, and a
banner inside the picture saying exactly that.

It exists to satisfy the renderer seam with real targeting and to discharge the
capture obligation — not to look like anything. It is described, with its
deliberate limits, in
[client notes](client-notes.md#gridworldview-the-current-implementation), and it
is **not a starting point for art**. A pixel-native renderer substitutes for it
behind the seam and inherits its obligations
([client architecture](client-architecture.md#the-renderer-seam)).

Colour in the current view carries **no authority**: hues are spread across
whatever terrain ids the frame contains, so a class can change colour between
frames. That is a property of a placeholder, and it is one of the reasons the
placeholder cannot quietly become the look.

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

`client/presentation/feel/` holds that scene. It is a **bounded experiment
beside the world-view seam**, not a presenter behind it: it consumes no
authoritative frame, emits no targets, sends no command, and replaces nothing.
`GridWorldView` remains the current world view. Whether anything in the feel
scene earns a place behind `WorldViewSeam` is decided by the production rule
above, not by this section.

The browser feel scene now carries a local walk experiment under the same
non-authoritative rule. Its movement lands on the beat: a drafted route of one
to three squares lands whole when the beat strikes, and nothing slides between
squares (owner ruling, 2026-09-02). Readiness shows in the cursor; the scene
draws no pulse. Walls that would hide the player's tile fade gently rather than
the camera or the wall height changing (owner ruling, 2026-09-02).

Its construction follows the lab's conclusions rather than re-deriving them:
the camera is orthographic at 45-degree yaw and 30-degree elevation, sized so
one ground cell projects as a 179-pixel-wide diamond at 1280 × 800; ground
cells are flat quads carrying material swatches in continuous world-plane
coordinates; walls are real geometry built from a member profile in world
units (plinth, sill, plaster, posts at every cell boundary, braces, lintel and
door, cap front and cap top) so courses never seam; standing subjects are
world-up billboards anchored at their cell centres at their nominal heights;
and the scene's only warmth comes from actual light sources — the lantern and
candles — as the owner ruled in the lab.

**Viewport ruling (owner, 2026-09-02).** The frame is 6.31 world units tall
at the 1280 × 800 minimum play surface — one comparison step out from the
lab's 5.05-unit frame, which is superseded — so the whole closed footprint of
a building, roof included, sits in frame and the player stands about a fifth
of the frame height. The ruling was made against painted full-frame references
of the courtyard and the ground room (lab, review-candidate tier) in which the
player stands about a sixth of the frame height; a further step out is
expected once cards are re-baked at a higher density, and the client's `zoom`
comparison step stays for that judgment. Rulings made beside it: **prop scale
is one metre to one world unit** for three-dimensional source models rendered
to cards, superseding the lab's 0.79 mapping, because metre-true furniture
read as dollhouse scale beside the references; **the wall profile stays** and
the roof gains its missing weight — steeper pitch, deeper eave, a darker and
larger-scaled shingle swatch; and the two painted references are accepted as
placement and scale targets at the review-candidate tier only, never as
masters. Masters come from this project's own build once it matches them.

**Interior camera ruling (owner, 2026-09-02).** Outdoors the camera is the
player's: centred on the caretaker and re-centred on each landing. Inside a
building the camera belongs to the space: it is centred on the room and
stops following the character, so a room that fits the frame is seen whole,
as the painted references frame it. It follows the player again on the way
out. The rule keys on the space, not the player's position: a dedicated
interior is the space that carries no weather.

Anything that belongs to a building — a hearth, a doorway, a window, a shelf —
is built from material swatches over math like the walls, because it is seen at
an angle and must have a side; free-standing things — a person, a tree, a table,
a stone — are painted cards at the camera angle, placed clear of walls, and a
flat wall-plane card is for genuinely flat things only. Characters are modelled
and rigged in 3D, rendered at the ruled camera angle into sprite sheets with the
pixel-cluster treatment, and shipped as cards; the raw mesh never enters the
scene, while how many frames are rendered follows the still-open question of
whether figures animate at all (owner ruling, 2026-09-02).

Vegetation cards take motion from one world-space wind field, phase from their
position rather than a shared loop, and hold the root while art-derived canopy
weight lets leaves move apart from trunk; grass follows the same rooted rule
(owner ruling, 2026-09-02).

**Chrome rulings (owner, 2026-09-03).** The screen is one built object and
nothing on it floats: no windows, no panels, no glossy overlay. Its structure
follows the mid-1990s online tactical RPG the owner holds as the feel
reference, read from the owner's own screens and written here in this
project's words: the play view sits top-left; a column on the right holds,
top to bottom, one large region switched by a strip of carved tabs — what is
worn, the rings, the sack, a single thing examined, who is near, and what is
prepared — then the two hands as recessed slots, then three glass gauges with
a number under each, then a door mark to leave; a strip along the bottom
holds a row of carved action tiles. **The three gauges are health, stamina,
and mana, left to right; one the character has none of is shown empty, not
hidden.** The chrome is built from the estate's own materials — the packet's
plaster, timber, and fieldstone — with serif ink for type, and it is lit by
the world's practical light so it belongs to the scene. The game is mouse
driven: there is no mode button, and the route preview and its walk, run, or
sprint word live on the world, not in the chrome. Events and speech are
written over the play view's bottom-left as plain coloured lines with no
box, never in a chat window. The sack shows its things as pictures at their
own sizes, placed freely; those pictures are an asset class of their own, not
scene cards. The play surface's ruled frame is unchanged by the chrome: a
larger screen shows more world beside the column, not a larger world. The
look of the chrome — its painting, not its structure — is not yet accepted;
the lab's layout compositions are evidence for the structure only.

**The candidate-asset rule.** The scene's art is candidate material from the
lab, and none of it is tracked. The scene loads assets only from the directory
named by the environment variable `TME_FEEL_ASSETS`, through a manifest that
binds every file to its digest; a mismatch is refused. With the variable unset
the scene presents absence inside the picture and still runs. This is the
same discipline the credential model and the capture output use: a private
input is named out of band, never defaulted to, and never a tracked path. The
tracked proof exercises the loader and the geometry against a tiny synthetic
fixture under `client/tests/fixtures/feel/`. Presets — time of day, rain, fog,
wind — are selected by `TME_FEEL_PRESET`, not by input actions, so the
accessibility floor is untouched.

Nothing this scene renders is an accepted master. Acceptance of any of it is
the owner's, at play size, through the production rule.

## Open

Deliberately unresolved, and not to be resolved by implementation:

- the owner verdict on the representative micro-scene;
- the exact native tile-edge length, joint width, camera distance, and actor
  scale within the ruled 45-degree-yaw, 30-degree-elevation framing;
- the exact palette discipline and animation budget within the ruled pixel and
  surface grammar;
- the dead world's visual identity beyond the direction above;
- the UI's typography and contrast palette, and the painted look of the
  ruled chrome structure
  ([client architecture](client-architecture.md#input-and-the-accessibility-floor)
  owns the accessibility floor those must clear);
- which production tools and presenter earn a place, which is decided by the
  production rule and the bounded evidence plan, not in advance.
