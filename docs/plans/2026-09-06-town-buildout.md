---
last_updated: 2026-09-07
revision: 13
status: Temple daily-use pass and current-game preview, followed by renderer-default and login URL leak repairs.
public_safe: true
summary: Town and temple iteration, real-game preview deployment, renderer selection and login safety repairs with evidence.
---

# Town buildout

The owner dispatched a cleanup of the recovered narrative documentation where
needed, followed by the remaining town and the dungeon stairs inside the temple.
The [surface brief](2026-09-05-first-land-surface.md) owns the geography and
resident/service directions. [Presentation direction](../presentation-direction.md)
owns appearance and camera; [browser client](../browser-client.md) owns candidate
loading, local movement, portals and proof. This Planning document owns execution.

## Subsequent integration

The [first-expedition record](2026-09-06-first-expedition.md) now owns the
accepted geographic encoding, class-specific services and served temple
destination. The scope and receipts below describe the earlier local candidate
slice; they do not override the subsequent runtime implementation. Artwork
acceptance and the wider first-land surface remain open. The later
[interior ruling](../presentation-direction.md#placeholder-town-interiors)
explicitly designates these rooms as placeholders pending guide images and
visual iteration.

## Interior guidance pass

The owner directed continued work after the first expedition and placeholder
interior ruling. The temple is the first bounded guide: its sanctuary, care
supplies, balm service and guarded descent establish the room's purpose.
Built-in image generation used the final native temple capture as a camera and
layout reference. The generated image, exact prompt and receipt remain in the
external visual lab; no candidate image is a repository or runtime dependency.

Selected temple guide SHA-256:
`ba0183d7832e3d0d7f8b3fb21cab8907b2228638266df364734a993e701a3f91`.
Prompt SHA-256:
`559bab30c981c5e14c5b12d4a98fa93627ad8d44c12478d947e2edebed28fe07`.
The owner selected this direction and dispatched its modeled implementation.
The guide is not an accepted artwork master or a modeled result. Timber joinery, practical lamps, care supplies and
a stocked balm counter are useful proposed vocabulary. The generated floor's
fractures and wear are too dense for the standing calm-field target; simplify
them during modeling. Illustration-derived stair and furniture extents carry
no geometric authority: preserve the compiled footprint, open circulation,
service squares and transition endpoints.

The [interior ruling](../presentation-direction.md#placeholder-town-interiors)
owns the selection and subsequent original 3D implementation at the fixed camera
and actual play size. No runtime, geography, deployment or Git lifecycle
change was made for this guidance pass. Documentation routing, whitespace and
links are the applicable verification surface.

## Temple refinement proposal

The owner requested a more stylized, older and worn interior, an older female
balm seller, and young, friendly, tired Tomas. During the guidance pass the owner
also requested more room or a better arrangement, the seller farther back beside
a shelf, direct right-click buying from her, and a priest who wanders the hall.
[Presentation direction](../presentation-direction.md#placeholder-town-interiors)
owns the amended visual target; [browser client](../browser-client.md#temple-resident-interaction-direction)
owns the implemented interaction surface.

The recovered private profiles and current temple synthesis were inspected
before prompting. They supply character temperament and purposeful furnishings;
their raw text and hidden narrative material remain outside the checkout.
Proposed hair, clothing colours and facial details are guidance choices rather
than claims of recovered exact physical descriptions.

Built-in image generation used the current native temple capture and previous
guide as references. The revised image places the seller in the rear-right
work area beside stocked shelving, a small table and book; Tomas stands mid-step
with an open welcoming hand in the central hall. The care nook moves to the
right, with a bench to the left, a clear front entrance and the left dungeon
descent. Rounded masonry, patched plaster, repaired oak and rubbed edges establish
age through use. The grid remains visible. Dense floor texture should still be
simplified when judging the modeled result at native size.

New guidance SHA-256:
`d6b277a0e12452e2df3593884712e88170bbb3f7a87ef83fcbbc4f29ccd1b91a`.
Exact prompt SHA-256:
`f54caaf945a8f777d9e3194ff8ed577a54d5f58d0a164a3ab526186e459a903d`.

The image, exact prompt, source hashes and inspection receipt remain in the
external visual lab. This is a new design proposal, not an in-game capture,
selected artwork master or revised geography acceptance. No model, land,
runtime interaction, provider 3D request, deployment or Git lifecycle change
was made in this guidance pass. Actual resident motion and buying require the
behavioral proof recorded by the browser owner.

## Temple model implementation

The selected guide now has an original modeled temple candidate. Existing
walkability, room bounds, service squares, exterior and stair/door destinations
are preserved. A packet comparison proves that the only changed manifest fact
is the temple GLB digest; all other asset files remain byte-identical to the
accepted geographic review packet.

The room adds substantial timber joinery, individually modeled stone courses,
quiet flagstones under the existing grid, a linen-dressed altar, a receiving cot,
care supplies, a stocked paneled balm counter and caged lanterns with pitched
hoods. The descent has a stone surround, subfloor masonry and restrained bounced
lamplight. A missing rear face initially exposed scene background through the
stairwell; the model now encloses that wall below the existing opening.

The final material candidate adds deterministic static vertex contact shading,
using the established external authoring helper. It makes joints and shelf
contacts legible while retaining runtime light and shadows. Fifty-one semantic
material groups let foreground meshes fade independently; the GLB has 43,068
triangles. The geometry audit checks connected-component winding and proves the
floor lies below the tactical grid. Source Blender files, exact scripts,
retained texture inputs and audit receipts stay in the external visual lab.

The [browser owner](../browser-client.md#first-expedition-presentation-study)
owns the separate candidate artwork receipt. This removes the initial coupling
between a geographic review manifest and a presentation iteration. Geography
acceptance is unchanged; asset bytes and geographic ancestry each remain
verified. An explicit candidate receipt cannot claim artwork-master acceptance.

The first geometry candidate passed the live dock/temple route in Chromium,
Firefox and WebKit, including Tomas access, balm purchase, reconnect, descent,
return and logout. The final material candidate subsequently passed the same proof in all three
engines, with 53 ordinary UI commands per engine.
The core rules, server, protocol, authoring and land files match all 519 bindings
from the completed first-expedition receipt. No numerical gameplay change or
backend migration belongs to this visual slice.

The model is the first implementation toward the selected guide, with native
appearance review still separate from direction selection. Carried character
studies and the remaining town interiors remain provisional. The dense floor
fracturing from the generated guide was deliberately omitted at gameplay size.

### Temple implementation closeout

The native final receipt SHA-256 is
`9c1d5ce2f305e50c2320accfd6a05bae988cd1487594b6d059ac62fa5e525008`.
The current study receipt owns the selected asset manifest identity; the external
packet comparison and closeout bind its Blender sources, model and captures.
Final native captures were inspected for the fixed framing, readable service
figures, furniture, floor and descent. They are actual client views, not guide
image substitutions.

The web verification lane completed locked install, typecheck, 487 tests across
41 files and both builds. The final asset-binding data passed the focused three
binding/feedback cases again; every native engine then loaded that exact packet
through the authoritative client. Documentation routing, whitespace, links,
metadata targets and boundary checks passed. No backend code changed, so the
previous complete expedition's Rust and persistence receipts remain the relevant
backend proof; this slice adds real PostgreSQL/TLS UI coverage for the temple.

The native UI proof used fresh databases and normal TLS with observed hardware
rendering. All scratch databases were dropped, the task-owned cluster and URL
were removed, and proof browser trust was cleaned up. The base HEAD remains
`ce4ea993d05576f35e99969a2e991504c9d42cd1` on the carried dirty `main`; no Git
lifecycle or remote preview change was performed. Earlier inspection failures
and the first geometry version remain separate from the final material receipt.

## Original scope

- Reconcile the private lore packet with the current smaller temple, lodge,
  map-template and gameplay boundaries. Preserve the recovered originals and
  unresolved mysteries; produce a current private reading route and an explicit
  disposition for stale geography, mechanics, character drafts and open names.
- Expand the existing local arrival candidate to cover the full town envelope.
  Preserve template terrain and the dock approach; author new construction
  inside the town footprint, with readable purposes and signs of actual use.
- Give the existing building reservations coherent functions and complete their
  exteriors. Include the lodge's common locker room and recovered-gear table,
  workshops, provisions, bakery and civic services. Residential lofts and further
  household dressing remain subsequent visual work.
- Owner additions during the slice: put a bank and banker centrally in town,
  include a sheriff, and provide places for the class trainers. The candidate
  groups civic duties at the central bank/hall and trainer spaces around a
  practice court. Coin weight, encumbrance, stamina costs and pushing coin piles
  were recalled as uncertain ideas, not dispatched mechanics.
- Build enterable town interiors. The smaller temple houses the balm stall,
  Tomas's receiving area and an actual modeled stairwell down. Preserve source
  entrance evidence separately from the newly authored interior arrangement.
- The dungeon beyond the descent remains unauthored. Show the stairwell without
  inventing a destination, silently teleporting into a substitute room, or
  claiming that resurrection, commerce, lockers or NPC schedules work.
- Use the established candidate materials and models, adding original geometry
  where needed. Keep the packet, Blender sources and private lore outside the
  checkout. No master acceptance, Git lifecycle or remote deployment is included.

## Proof

Inspect the existing candidate author, compiler, manifest, portal and movement
contracts before edits. Search `schema_version`, `portals`, `StructurePlacement`
and `passabilityFrom` if a contract change becomes necessary; migrate callers and
proof together. Prefer validated candidate content within existing contracts.

Prove source-terrain preservation, connected approaches, intentional building
occupancy, unobstructed portal landings, and the visibly descending stairwell.
Run the repository lanes for changed paths, then native input and visual review
in the shared browser roster: dock to town, every building entry and return,
temple stall and receiving area, stair approach, lodge locker/table circulation,
day/dusk exteriors, and foreground fading. Record actual results and source hashes
in the external receipt, including failures and their resolution.

## Findings and closeout

The recovered lore includes obsolete fixed house lots and a no-inn restriction,
and carries source-mechanics verification instructions superseded by current
owner rulings. Resolve these in the private current reading packet. Character
workshop redlines and unanswered mysteries remain explicitly open; cleanup does
not turn them into accepted dialogue or resolved story facts.

The private current reading packet is reconciled; the recovered originals remain
unchanged. The external candidate now has ten buildings, ten interiors, a lodge
locker wing, central bank and sheriff, four trainer stations and posed resident
studies. The expanded surface has 972 explicit cells; the Rust compiler accepts
472 connected walkable cells and all ten building approaches without diagnostics.

The stairwell exposed the renderer's mandatory opaque floor. The browser owner
now defines explicit, unwalkable floor openings; the candidate uses this for the
modeled temple descent. Sparse cell arrays remain refused. Interior circulation
checks caught isolated cells behind counters and the receiving cot; the authored
staff zones and cot placement now leave every visitor floor cell connected.
The preserved lodge, market and workshop dressing retains its eight compiler-owned
occupied cells. A room-wide shadow receiver initially concealed the lower stairs;
its geometry now uses the same explicit floor openings.

The first material pass also split roof fading by tile colour. Newly authored
roofs now carry colour variation in vertex colours and export one roof surface
each. Native inspection rejected an incorrectly authored terrain asset row;
removing prop-only fields fixed the candidate without weakening the parser.
Updating the checkpoint date exposed four date-bound anchor links. The heading
now has a stable anchor and all four callers were migrated together.
Earlier refused and interrupted runs remain diagnostic history in the external
receipt; only final successful runs count as proof.

**Observed proof:** the selected web, documentation and boundary lanes passed,
including 460 browser tests, typecheck, production build and the provisioned
private boundary checks. Chromium, Firefox and WebKit each passed all ten native
building entry/circulation/return checks, complete compiler-mask comparison,
shore and stair-opening refusal, and restoration of faded surfaces at the dock.
Each engine recorded 66 native arrivals. All three used observed GPU rendering.

The final review has 21 native captures: day/dusk northern town, central street,
bank/sheriff, temple, trainers and lodge in each engine. A final bank-plaque move
below the window changed only that static model's digest. The receipt proves
all movement data and every other manifest field identical to the full
walkthrough packet; fresh bank entry and return also passed in every engine on
the final packet. The view harness initially asked for a route beyond the
candidate's movement budget; splitting that request into legal segments resolved
the refusal without changing movement rules.

The six current private reading documents pass link checks; all ten recovered
original hashes remain unchanged. Banker/sheriff identities, exact dialogue and
the recalled coin mechanics remain explicitly open. The subsequent
[design continuity audit](2026-09-06-design-continuity-audit.md) corrects the
trainer assumption: four generic stations are spatial placeholders pending
reconciliation with recovered class and training designs, not evidence that no
class baseline existed. Fifteen posed residents establish presence and scale; these are not
accepted portraits, animated NPCs or implemented services. Architecture and
household detail remain candidates for further visual refinement.

The local review site serves the verified packet and compiled bundle. The
external receipt binds the native results, source ancestry, private cleanup,
working browser source, Blender sources, model hashes and packaged files. No Git
lifecycle, remote preview update or master promotion occurred.

**Bounded-view finding:** the town crop covers the authored town envelope, not
the remainder of the first land. Scenic sea beyond its edge is renderer fill,
not source-derived coastline. First-land authoring owns replacing that fill in
the wider surface pass; proof must compare the widened source mask and native
views before treating it as finished geography.

## September 7 temple iteration

The owner selected the revised guide and directed implementation using the
available tools. The temple expands from 6×7 to 7×8, with 41 passable cells, rear
shelves and Maude's work area, a side cot and washstand, a waiting bench and a
clear receiving floor. The exterior footprint and both dungeon endpoints remain
unchanged. The temple's town-facing doorway and landing move within the enlarged
room. The authoring receipt records this September 7 amendment; the original
September 6 review remains historical evidence. Artwork acceptance stays open.

Two generated character references follow the selected guide and recovered
public-facing character direction. Owner-authorized image-to-model generation,
rigging and relaxed idle clips produce distinct live residents. Material
preparation restores the generated PBR maps after the rigging output replaces
them with an emissive material. Geometry, skinning, UVs and clip binding are
preserved. The candidates use 1K maps at native scale and softened normal strength;
original downloads, editable inputs and receipts stay outside the checkout.

The room uses original Blender geometry adapted from the previous temple source:
soft joinery, iron repairs, limewash, weathered stone, shelves and care supplies.
The receiving braziers stay cold. Inspection exposed clipping after
enlargement and hard palette speckling on the figures. Palette grading softens
the latter. An initial room-fitting zoom was removed after the owner recalled
the accepted UI layout; the nine-row extent stays fixed. Furniture remains an early model
pass and modular resident clothing is unfinished.

The [resident contract](../town-resident-contract.md) owns movement and service
semantics. The old fixed-position service shape is retired atomically across
source, runtime state, checkpoints, callers and proof. Protocol minor 10 adds
explicit provider identity. The seed validator and its large integration test
owner were decomposed by subject during the cutover.

Verification and final asset receipts are recorded at closeout below. This slice
does not update the private preview, publish assets or perform Git lifecycle work.


### Temple iteration closeout

Work is based on `main` at `ce4ea993d05576f35e99969a2e991504c9d42cd1`, with
pre-existing uncommitted work preserved. No Git lifecycle or preview update is
part of this iteration. Canonical fact ownership remains in the resident,
browser, presentation and geography contracts linked above; this section is an
execution record, not another service or layout definition.

The final Rust lane passes formatting, workspace build, clippy and 1,435 tests.
The final browser lane passes a clean dependency install, typecheck, 494 unit
tests and both production bundles. Python's 548 tests, the Workbench selection
loop, subject routing, Markdown links and all four boundary checks pass.
Step-target validation also passes. These observed lanes cover the portable and
browser plan selected by all 280 currently changed source paths; full merge and
clean-clone certification were not requested or claimed.

The candidate manifest digest is
`60b5d33fdeb75192fda3d3ac962934b82b457711060a004e318b6739aca93d99`.
All 195 referenced asset digests match. The private temple iteration root retains
original downloads, generated references, editable room source, independent
layout amendment, material and packaging scripts, verification logs and native
captures. This pass consumed 76 provider credits. No generated source payload is
added to the checkout.

Findings resolved in this slice:

- Provider rigging introduced emissive material and dropped PBR inputs; packaging
  restores the original maps and verifies their exported bindings.
- A hold-ground behavior swallowed the new circuit. Patrol now follows combat
  priority and has a continuing multi-position checkpoint proof.
- Cadence was initially authored using the wrong unit assumption. The current
  value follows D5 and the resident owner; a test requires continued movement.
- Service checks compared an area and coordinate without the realm. Every
  service gate now compares the complete world position, with cross-realm denial
  proof.
- A new display label exceeded the wire's ASCII contract. The label is corrected
  and native projection tests cover both authored provider squares. Full Unicode
  wire labels remain a protocol-owner product decision, not implied support.
- The old recording fixture's source digest and field path became stale. The
  current harness is migrated; the exact historical input is preserved separately
  and remains bound to its original capture receipt.
- Picking proof initially ignored the shared-square visual offset and later
  clicked a canvas scrolled out of view. The native proof now targets the rendered
  resident after scrolling the canvas into view. Cosmetic sharing offsets no
  longer make a stationary resident turn or play a walking clip.

Remaining presentation work is bounded: furniture and supplies still need a finer
shape and variation pass; resident outfits are monolithic candidates. Presentation
owns those refinements and acceptance requires actual gameplay-camera review.
The existing large-bundle warning remains browser packaging debt: browser
architecture owns measuring and splitting loading boundaries before production
performance acceptance. This pass does not claim that budget.


**Viewport correction following the owner's reminder:** the accepted plate and
its sliced manifest survive in the private visual lab. The presentation owner
records their exact aperture; the browser owner records the diagnostic-page gap.
Automatic interior fitting was withdrawn. The fixed nine-row scale is tested at
diagnostic, design and doubled design dimensions. UI integration is the next
framing slice; current native play-page captures are not accepted chrome proof.


**Final native result:** Chromium, Firefox and WebKit each pass 65 committed UI
actions against an independently created scratch PostgreSQL world, with normal
TLS and hardware-renderer evidence. Each loop creates a character, walks from
the dock, observes Tomas move without player input, approaches his attached
service, right-clicks Maude, proves insufficient-funds buttons disabled, buys a
balm, reconnects with inventory and gold intact, descends into the dungeon and
returns through the temple to town. The final runs retain the fixed nine-row
camera scale. Their aggregate binds identical unchanged browser and authored
world files. The native play-page captures remain diagnostic-UI evidence.
The separate image at the accepted aperture uses explicitly synthetic
observations and is labeled as inspection only.

The final browser lane remains 494 passing tests after the viewport correction.
All scratch databases are removed; the owned temporary PostgreSQL cluster is
stopped and its data, socket and admin URL removed. Logs and private art/proof
sources remain retained for the next iteration. The ordinary development service
and remote preview are unchanged.


## Temple furniture and material iteration

This subsequent art-only pass is based on the same `main` ancestry above, with
carried changes preserved. Presentation direction owns the visual treatment;
this Planning section owns execution evidence. Authored world files are
byte-identical to this pass's starting snapshot. Resident movement, services,
figures, camera and UI geometry are unchanged.

Original Blender work adds substantial furniture joinery, varied pottery,
an open ledger, cloth folds, care supplies and small plants. A generated
four-material sheet supplies the illustrated surface treatment. The retained
asset inventory was searched before choosing to refine the existing original
scene. No new third-party model was imported and no paid model-generation
request was made. All generated images, editable geometry and proof artifacts
remain in the private temple R4 working root.

Export inspection found two defects: diagonal members sampled outside their
atlas quadrant, and mixed vertex-colour layers produced black surfaces.
A repair also exposed lost UV layers. The source now maps atlas regions before
batching, gives every mesh explicit base patina and selects the correct render
colour layer. Packaging requires texture coordinates on every textured
primitive and finite, nonzero exported contact colours. The final scene is
rebuilt from that corrected source; repair attempts are retained as diagnostic
history. Floor scratch marks that resembled interface symbols were removed.

The approved-aperture image is a composition inspection through the actual
renderer with synthetic observations. Live play-page captures remain diagnostic
UI evidence under the existing viewport ruling. Artwork acceptance and full
chrome integration remain open.

The final manifest digest is `cf50f04f0b598d9bb820fd6fdc3c385eb555be666f7ecb2e533194f40132a0c3`.
All 195 asset references pass integrity verification. The room has
184,329 triangles in 105 semantic material batches and occupies
21,218,992 bytes, versus 63,694 triangles, 116 batches and 6,647,060 bytes in
the preceding candidate. Short foreground inspection at the approved aperture
on Intel UHD 630 recorded median 33.3 ms and p95 33.4 ms
frame intervals over 120 frames. This is approximately 30 fps on that host,
not production performance acceptance. Browser architecture owns the remaining
loading and rendering budget. The larger model warrants a separate loading and
render-cost comparison before production performance acceptance; the short
frame sample alone does not establish a regression or its cause.

Further guide matching belongs to presentation: stone courses and exposed
masonry still read more regular than the selected image, and small furnishings
retain some primitive geometry. These are visual refinements for the next
candidate, not claims of an accepted master.

The selected fast verification plan passes a clean browser dependency install,
typecheck, all 494 unit tests, both production bundles, step-target validation,
document routing, whitespace, Markdown links and all four boundary checks.
The existing large-bundle warning remains recorded browser packaging debt.

Chromium, Firefox and WebKit each pass the final 65-action live UI loop against
independent scratch PostgreSQL worlds, normal TLS and the same frozen browser
and authored world bytes. Each run proves the patrol, attached resident services,
right-click purchase, insufficient-funds refusal, durable reconnect, descent and
return. The aggregate binds the candidate manifest, tracked study receipt and
proof script. All scratch databases are gone; the owned temporary PostgreSQL
cluster, socket and admin URL are removed, with logs retained. No preview or
Git lifecycle operation was performed.

The owner proposed testing this candidate on a Lenovo T495s as another laptop
baseline. A future preview comparison should report viewport, browser, power
mode and frame times. Publication and any diagnostic readout are outside this
art pass; the private preview remains unchanged.


## Temple stairwell iteration

The owner requested another visual iteration focused on the dungeon stairs.
This art-only slice uses the same ancestry recorded above and preserves carried
work. Presentation direction owns appearance; this Planning section owns the
execution evidence. The private R5 source reuses the R4 material sheet and
original Blender helpers, replacing the stairwell geometry through a dedicated
source module. No additional generated image or third-party model is needed.

The previous retaining wall projected above its coping, and the rear lintel sat
above floor level. The replacement aligns the retaining courses with their caps,
staggers their joints, dishes the treads with shallow central wear, and places
the small arched doorway below the floor. The open old door leaves and lamp are
repositioned within the well. Depth darkening is explicit artistic vertex
patina; it does not add gameplay lighting or change the camera.

Packaging binds every authored world file to its starting digest, checks
exported material coordinates and contact colours, and updates only the room
asset and candidate receipt. Resident figures and all other packet assets stay
byte-identical to R4. Native screenshots and traversal proof determine whether
the refined entrance remains readable at the existing gameplay scale.

First inspection exposed missing rear masonry above the arch and excessive
foreshortening of the steep flight at the fixed camera. The source closes that
rear wall and uses a shallower modeled tread rise within the same opening.
The failed first render is retained separately; the candidate is rebuilt from
the corrected source before final verification.

The final room contains 197,318 triangles in 114 semantic material
batches, occupying 22,410,492 bytes. Its candidate manifest digest is
`8877ae68481a92cf722b3c43a4890b3407a903aedf584e83ff07d171ca10dc83`.
All 195 asset references pass integrity verification. A separate read-only
Blender check binds the modeled stair envelope to the retained editable scene;
the surround peaks at approximately 0.329 world units above the floor.
At the approved aperture, the same short 120-frame inspection recorded 33.3 ms
median and 33.4 ms p95 intervals on this host. This remains an inspection sample,
not performance acceptance or a claim about the owner's laptop.

Firefox exposed a proof-harness race while recollecting the purchase flow:
DOM readiness was observed before the separately delivered WebSocket command
event. The current private proof waits for the newly sent command and its
matching server receipt before asserting the count and accepted result. It
also binds the proof script before execution and refuses mid-run changes.
The initial run is retained separately; the final matrix uses the corrected
proof against the same candidate bytes. This changes no game timing or service.

The selected fast plan passes the clean browser install, typecheck, 494 tests,
both production bundles, step-target validation, document routing, whitespace,
Markdown links and all four boundary checks. Existing browser packaging and
performance acceptance work remains with the browser owner. No preview update,
Git lifecycle, generated-image request or paid model-generation call occurred.

Final native result: Chromium, Firefox and WebKit each pass the complete
65-action UI loop using the corrected receipt wait, independent scratch
PostgreSQL worlds, normal TLS and hardware renderer evidence. The aggregate
binds identical browser and authored world bytes, the current candidate manifest,
tracked study receipt and unchanged proof script. Stair-approach and return
captures keep the player readable at the existing landing. The inspected
approved-aperture close-up remains explicitly synthetic-observation evidence;
live captures come from the diagnostic play page.

All scratch databases are removed. The owned temporary PostgreSQL cluster,
socket and admin URL are gone; editable Blender sources, failed first inspection,
initial proof failure, final receipts and captures remain in the private R5 root.
The ordinary development service is unchanged. Artwork remains a candidate;
remaining broader room refinements and performance acceptance retain the owners
recorded in the preceding iteration.


## Temple stonework and textile iteration

The owner requested another pass against the selected guide. This bounded art
slice uses the same ancestry and preserves carried work. Presentation direction
owns appearance; this Planning section owns execution evidence. The private R6
root retains the original Blender source and reuses the R4 material atlas.
No additional generated image or third-party model is introduced.

The room replaces regular footing blocks with clipped, staggered stones and
opens the limewash mesh to recessed rubble and mortar. The former flat patch
symbols are retired. Floor slabs receive varied texture orientation and a
slightly lighter material. Bedding uses rounded stuffed geometry and modeled
cloth folds; timber receives restrained broad asymmetry. The existing room
footprint, resident services, stair traversal and camera remain under their
current owners. Packaging binds every authored world file to its starting hash.

The private proof inherits R5's matching-command receipt wait and start-of-run
script binding. Final results and candidate limitations follow below.

During this pass the owner added [contextual idle presence](../presentation-direction.md#settling-into-an-occupied-square)
as standing direction for players and NPCs. It is recorded in its presentation
owner and indexed in settled conclusions; this room-art slice does not implement
that animation system.

The owner then identified the uniformly lit room as the next cause to correct.
The slice includes an explicit authored interior-lighting profile, fixture-only
key illumination and bounded practical shadows. Runtime ownership and validation
live in the [browser contract](../browser-client.md#house-atmosphere); appearance
belongs to the [source-lighting ruling](../presentation-direction.md#room-lighting-from-visible-sources).
The low stair lantern and its hook move about four treads farther down the
existing well, with its light anchor moved alongside it. This separates its role
from the west-wall room lantern without changing the descent link.

The exterior-water request is recorded in the [coastal follow-up](2026-09-07-coastal-water.md).
It includes fresh primary-source research, inspected shader gaps and completion
criteria; this temple slice does not change water rendering.

The final candidate has 222,962 triangles in 114 semantic material batches and
30 materials; the room GLB is 25,232,632 bytes. Room digest:
`63bc77997cf7d10a83c5639692cee2deaea6238a982a24fda84988ff37e12a19`;
candidate manifest digest:
`afeae08f19a9cdf338d7566edf6638467291b52ebad71d36272070a72c78d5ba`.
All 195 manifest references match, all textured primitives retain UVs and valid
contact colours, and every authored world file matches its starting digest.
Read-only Blender bounds prove the moved lantern remains inside the existing
stairwell envelope.

The actual-renderer inspection at the accepted 1462×753 aperture has seven
fixture lights, one restrained ambient fill and no directional light. It records
300 draw calls, 829,731 rendered triangles including shadow passes, and 120
geometries/16 textures. A 120-frame foreground sample reports median 33.3 ms and
95th percentile 33.4 ms on the development Intel UHD 630 renderer. This is a
browser-paced inspection sample, not GPU cost or the requested laptop baseline.

The first fixture pass revealed overly bright orange rear walls, dark figures
and a large shadow from a lantern's own housing. Room-facing anchors, less
saturated light and broader authored falloff correct those causes. An attempted
6.5-unit light range was refused by the existing asset validator; the asset was
corrected without widening its limit. The initial documentation check refused
three research-citation hosts; exact hosts now have reasoned allowlist entries.
Final selected verification passes the boundary, Python, documentation and web
lanes, including 548 Python tests, 497 web tests, type checking and both browser
builds. Logs retain both initial refusals. Room disposal also releases remaining
scene lights and their shadow maps; fixture-effect disposal retains its own owner.

Chromium, Firefox and WebKit each passed the native authored loop against the
same final browser/world bytes and candidate manifest. The runs committed 65,
67 and 65 accepted actions respectively; resident approach paths may differ
with patrol position. Proof covers creation and dock arrival, temple residents,
Tomas moving on server deadlines, actor-bound service interaction, disabled
unaffordable purchases, buying balms, reconnect persistence, dungeon descent,
temple return and town return. Native captures were opened in all three engines.
The private aggregate binds the proof script at run start and the asset,
receipt, browser and world digests; no source changed during these runs.

Temporary PostgreSQL worlds were removed, the owned cluster stopped, and its
data, socket and temporary credential file removed. No development service or
preview deployment changed. Contextual idles, accepted HUD integration, laptop
performance acceptance and owner visual acceptance remain open under their
respective owners. The existing browser-bundle warning retains the previously
recorded packaging follow-up. Final documentation and boundary proof is retained
with the R6 closeout receipt.

## Temple patterns of daily use

The owner approved a bounded pass on floor wear, evidence of daily work, less
orderly furnishings and gentler lantern shadows. Ancestry remains
`ce4ea993d05576f35e99969a2e991504c9d42cd1` on carried dirty `main`. This Planning
section owns execution evidence; [presentation direction](../presentation-direction.md#temple-material-and-framing-experiment)
owns appearance and [browser client](../browser-client.md#house-atmosphere) owns
the explicit lighting metadata. No camera, authored geography or service rule
changes belong to this pass.

The private R7 candidate reuses R6's original Blender construction and material
atlas. Broad floor wear follows the entrance, altar, stairs and care routes;
recesses retain dirt, with two small inset stone repairs. An open packing tray,
set-aside lid, marking stick, ink cup, stained work linen and a tucked stool
share Maude's existing table footprint. A hollowed seat cushion, spare folded
blankets, an angled pillow and a deeper blanket drape soften the orderly room.
The authoring recipe checks the new work-area props against the already blocked
cell. The lower stair lantern stays in its R6 position.

The room's authored shadow radius and strength soften lantern shadows without
adding a light or shadow map. The metadata contract migrates atomically to six
required fields; decoding refuses the previous four-field shape. Existing
single-owner, outdoor-refusal and source-budget rules remain in force.
Native inspection includes a character beside the west-wall lantern, where the
preceding candidate produced its harshest shadow. Candidate metrics and final
verification are recorded below.

During this pass the owner extended NPC idle direction to include looking around
and useful room activities, specifically Tomas straightening the cot and Maude
stirring a preparation. [Presentation direction](../presentation-direction.md#npc-attention-and-room-activities)
owns the requested behavior and its distinction from delivered functionality;
settled conclusions routes to it. The art pass supplies room context, not a new
NPC activity scheduler or contact-animation system.

The final room contains 236,412 triangles in 123 semantic material batches,
29 materials and a 26,297,240-byte GLB. Room digest:
`8e1453387f235ad4022309e3d1c7aea67c8f3ffdb37211c3ff37aa4ce0c88e3f`;
candidate manifest digest:
`dc549664e4a439a556fe350e5ecbab8f701c0e776ed67839655f10e96f4bb707`.
All 195 manifest references verify, all textured primitives retain UVs and valid
contact colours, and authored world bytes match the starting receipt. Read-only
Blender proof confirms the original stairwell envelope.

Inspection at the accepted 1462×753 aperture records seven fixture lights and
one ambient fill, with no directional key. The single shadow map uses the
authored softer treatment. The frame contains 315 draw calls and 884,281 rendered
triangles including shadows, with 129 geometries and 16 textures. A 120-frame
foreground sample reports median 33.3 ms and 95th percentile 33.4 ms on the
development renderer; this browser-paced sample does not establish GPU cost or
performance on the owner's laptop. Full-room and west-lantern captures were
opened; the latter checks the previously harsh near-source shadow.

The initial build refused a linen fringe slightly outside its occupied-cell
envelope. Moving the cloth inward resolved that finding. Blender's default exit
status did not propagate the Python assertion; subsequent invocations explicitly
use `--python-exit-code 1`, with packaging and native proof still required.
The initial small repairs read as raised plates; a bounded relief adjustment
makes them nearly flush, and the full recipe authors the same final height.
The lighter indirect palette helps nearby character readability while keeping
the source-lighting contract. Failed-build evidence and first-pass captures are
retained beside the final sources.

Selected verification passes all 497 web tests, type checking, both browser
builds, document routing and links, whitespace and public-boundary checks. An
earlier pass covered the code before the final asset; the final run rebuilt
against the final candidate receipt. The existing large-bundle warning remains
under the previously recorded browser packaging follow-up.

The native action loop exposed two refresh defects in the diagnostic panel:
replacing buttons could lose a press/release gesture, and replacing unchanged
summary text could lose WebKit disclosure activation. Retained controls now
resolve current offers and generation, preserve focus/amounts and refuse revoked
or disconnected actions. Native regressions reproduce both failures and pass in
Chromium, Firefox and WebKit after the fixes. Final scratch temple loops pass
with 65, 67 and 65 commands respectively; the count varies with Tomas's current
patrol position. Each covers the dock, town route, service attention, buying,
dungeon stairs, return and reconnect. All runs bind identical final browser,
world and artwork bytes. The owned scratch PostgreSQL cluster is removed.

The owner expanded this slice to replace the obsolete static preview with the
whole current playable game, including exterior travel and every current
interior. The obsolete development installation was explicitly retired by the
owner. The new persistent installation uses the first-expedition world, the
matching optimized Rust server, browser/codec and verified artwork. The release
copies only manifest-listed assets, not the private source packet wholesale.
A separate Git index captures its source tree without changing the working index
or refs. The development runbook owns repeatable stage/activation and recovery.

Installation exposed two old assumptions: copying the test actor one cell east
put it in water, and copying equipment IDs created duplicate ownership. The test
actor now shares the declared traversable spawn with empty inventory; the
original authored actor and loadout remain intact. Real bootstrap validation and
regression tests cover this. Remote inspection also exposed an internal-port
redirect and HTTP Basic interference with credential-free codec fetching. The
frontend now emits relative redirects; the game's own login replaces Basic on
game routes. Sign-in and client assets are reachable; gameplay still requires a
provisioned account and public account signup remains unavailable. VNC retains
its separate existing gate. Failed attempts remain in the private receipt set.

The owner also established continuous visual scrutiny across all visible areas;
[presentation direction](../presentation-direction.md#continuous-visual-review)
owns that standing rule. Future public preview account creation is recorded in
[server notes](../server-notes.md#private-development-deployment), without opening
outside admission in this slice. Deployed-host verification follows below.

Deployed source-tree identity: `f4f75e6f3441019589be5069c2e89ded41a6959a`. The actual remote
Chromium run passed account login, character creation, dock movement, the town
approach, temple residents and services, balm purchase, dungeon descent/return,
town return, reconnect and sign-out. A final source-bound activation shipped
identical runtime file hashes, took a database backup and restored readiness.
The HTTPS sign-in route returns 200 without an internal port; an unauthenticated
game session returns 401 and the unimplemented account-signup route returns 404.
The private receipt records the exact release and host-configuration digests.

Visual review of the deployed dock at arrival (9,31) found a strongly repeating
bright water pattern and abruptly terminated grass strips. Screenshot digest:
`0d4d927e27a68e8d88c2dffb63000ca82b9e0d70a99805b067afe4240e996f7a`.
These concrete observations join the existing coastal-water/ground follow-up,
owned by browser rendering and presentation direction. Acceptance requires
quiet, readable shallows, visible shore depth and natural vegetation transitions
at the fixed camera, without hiding the gameplay grid. They are not accepted art.

Operational follow-up: `deploy/development/operations.py::restore_drill` still
expects exactly two characters. The current deployed world has an additional
UI-created character, so that old assertion would refuse an otherwise valid
restore drill. The deployment owner must replace the fixed-count assumption
with proof covering seeded and subsequently created characters; a three-character
backup/restore test is the required evidence. Ordinary backup and activation
succeeded; no successful restore drill for this new preview is claimed.

Post-activation smoke passes on the actual HTTPS host in Chromium, Firefox and
WebKit: game login, dock movement, reconnect with retained position and sign-out.
The final activation and those reads leave the owner's character at the dock.
All selected portable checks pass, including Rust build/lints/tests and the Python
suites; all 497 web tests, type checking and both browser builds also pass.

Two more deployed views remain explicit visual follow-ups. The temple frontage
and neighbouring roofs still read as plain, repetitive boxes; the exterior
architecture owner must carry the warmer temple material/shape vocabulary into
that approach without changing footprints or camera. The first dungeon landing
is still a bare placeholder of long uniform wall/slab runs; its interior owner
needs a guidance-led, source-lit dressing pass preserving stairs and passability.
The opened town-return and descent captures, hashes and locations remain in the
private visual-review receipt. Neither view is treated as finished because it
was traversed successfully.

## Preview renderer default repair

The owner reported the overhead diagnostic map on the newly deployed preview.
The root redirect supplied a study query, but direct index navigation omitted it
and selected diagnostics. A native Chromium reproduction confirmed that exact
path. Artwork deployments now compile the expedition renderer selection into
the browser artifact; the frontend serves the ordinary index without a mode
redirect. Query changes cannot downgrade an artwork deployment to diagnostics.
Inspection builds retain explicit diagnostic/study selection for proof worlds.
This slice changes renderer selection, not the unfinished HUD design, geography,
artwork, accounts or saved world. The private receipt records entry-path proof,
selected verification, matched release activation and deployed gameplay smoke.

During the renderer repair the owner reported credentials in the browser URL.
A native no-JavaScript reproduction proved that the unguarded login form used
GET before initialization; it intercepted synthetic markers before network send.
Host inspection confirmed the affected provisioned credential also appeared in
request logs. The account password was rotated through the existing operator
command, its old password was refused and its replacement accepted; account and
character identity were retained. Only the affected credential-query fields were
redacted from retained log events. Public preview access logging now omits query
strings and referrers, and the frontend redirects legacy credential query URLs.

The HTML form now starts disabled, has unnamed credential controls, declares POST
and blocks native form navigation through document policy. A small bootstrap
module leaves sign-in disabled on startup failure and clears legacy credential
query parameters. The initialized control adapter remains the only login sender.
Native regression proof covers absent JavaScript, stalled and failed codec load,
and successful normal login alongside the renderer entry-path checks. Release
activation restarts the server, invalidating its transient sessions.

Final deployed proof passed in Chromium, Firefox and WebKit: all three entry
paths selected the rendered world; absent JavaScript, native submission, stalled
codec and failed startup remained safe; actual login, rendered town, an
authoritative wait, reconnect with retained position and sign-out succeeded.
The owner's replacement credential also passed a separate browser sign-in and
sign-out without entering or moving a character or placing credentials in URLs.
A synthetic credential-query request redirected to the clean root, and its
markers were absent from the public access log. The matched release identity is
`955aa8af583bb6ea73e888927e0435910bd6e45c`.

The selected portable and web verification completed successfully, including
497 web tests, type checking and both browser builds. Proof-harness lessons:
Firefox may throw an empty DOM exception when policy refuses native submission,
so assert that no request was sent rather than requiring a successful method
return. Stalled startup proof must await installation of the intercepted route
handler before invoking its failure callback. Initial rejected HTTP preflight
was replaced by trusted HTTPS; final public proof used the real host in all
three engines. The external receipt retains failures separately from final proof.

## Horseshoe layout study

The owner subsequently directed the
[horseshoe arrangement](2026-09-05-first-land-surface.md#horseshoe-town-arrangement).
The next exterior study should compare a blockout against the settlement
envelope, dock arrival sightline, temple/dungeon access and every service
entrance, then review it at the fixed gameplay camera. Reserve the central
feature's space while its identity remains open. The layout has not been
implemented or deployed by the renderer/login repair; those checks prove the
existing world. Updated geographic receipts and native traversal proof belong
to the subsequent authored layout migration.
