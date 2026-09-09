---
last_updated: 2026-09-09
revision: 8
status: Connected scenes, revised Martial Artist preparation, preserved character sampling and pixel-effect maps recorded.
public_safe: true
summary: Source/style correction, standard sampling limits, fine character rasters, scene masks and lighting data.
routes:
  - web/src/play/pixel*.ts
  - web/src/play/pixel*.json
  - web/src/play/pixel*.css
  - tools/run_pixel_temple.py
---

# Pixel-art production

This Canonical document owns the **art-production method**. The
[visual target](presentation-direction.md) owns aesthetic direction;
[browser client](browser-client.md#pixel-art-presentation) owns implemented
rendering parameters and commands; the
[transition record](plans/2026-09-08-pixel-art-transition.md) owns the cutover and
next construction slice. Provider receipts, source images and editable candidates
remain external under the [candidate rule](presentation-direction.md#candidate-assets).

## Selected benchmark and its limits

The owner selected the final pixel temple after the character sampling and
outline pass, then directed a hard transition away from 3D presentation. The
benchmark is the `tomas-20260908-r5` packet, manifest SHA-256
`c6afb650a600d7cc9364e5795fc1dea437c5c08fd0d03e1f2cfd2f0b63406ca7`.
The [temple record](plans/2026-09-08-pixel-temple.md#character-detail-and-scene-integration)
owns its generation and browser evidence. Preserve its look when making the
next assets; a provider's default pixel style is not the target.
The September 9 temple restyle and modest Martial Artist simplification are
subsequent owner-directed revisions, recorded in the
[integration execution](plans/2026-09-08-pixel-exterior.md#temple-and-character-corrections).
Tomas and Maude retain their selected sources.

This accepts the visual direction and reference result. It does not certify
every source as an editable production master or claim finished animation.
The current Martial Artist and revised Tomas have eight standing facings and
hold those poses during travel. Earlier traveler gait work proves the motion
method, not a completed gait for these replacement identities. Maude retains
her existing sprite set. The connected town and restyled temple now have gameplay-integration candidates;
the [exterior execution record](plans/2026-09-08-pixel-exterior.md#gameplay-integration-and-pixel-effects)
owns their selection and evidence.

## Tool roles and model preference

1. **Built-in GPT image generation:** concept design, rich scene plates, focused
   appearance revisions and isolated source illustrations. Supply the selected
   project image as a reference; write the invariants explicitly.
2. **PixelLab Pixen:** prepare an isolated character as a detailed sprite and
   remove generated checker/matte backgrounds. Treat its result as new artwork
   that needs inspection; it can simplify a face or invent accessories.
3. **PixelLab V3 reference characters:** produce eight facings from the reviewed
   sprite. The reference carries the actual look; style options are not reliable
   preservation controls. Inspect every facing before animation or packet use.
4. **PixelLab animation:** build matching movement from the final selected
   directional PNGs. Use explicit source frames and inspect the completed loop.
5. **Retro Diffusion Pixel Fixer:** distinguish the standard deterministic grid
   detector/reconstructor from the neural reconstruction service. Both are
   preparation options requiring visual review, not interchangeable operations.
   The original selected temple plate did not use either; its September 9
   replacement uses the standard pass after scene-style correction. After the exterior comparison,
   the owner retired further neural attempts and directed continued work with
   standard correction. Retain rejected trials as evidence.
6. **Local raster tools:** deterministic crop, packing, mask assembly, export,
   hash/alpha checks and diagnostic contact sheets. Keep semantic generation and
   deterministic preparation distinct in the receipt.

**Owner preference, September 8:** prefer GPT Image 2.5 for future source art when
available. The [official announcement](https://openai.com/index/introducing-chatgpt-images-2-5/)
reports its rollout to Codex on this date. The tool interface inspected during
the cutover had no model selector or reported model identity. Record the actual
identity when exposed; otherwise record built-in image generation with model
unreported. Do not claim an upgrade based on appearance or a prompt asking for
a model. Continue the account-included built-in workflow; this preference does
not request a separately billed API substitution. A Codex update is planned
after this verified stopping point, before exterior source-art work.

## Scene plates and ground

The owner rejected the first assembled exterior despite its functional proof.
Use a full generated scene directly or as the controlling visual guide. Establish
composition and the [upright presentation](presentation-direction.md#projection-and-surface-ruling)
before extracting reusable layers. Passing browser tests does not establish
visual quality. A roof-heavy physical camera blockout is unsuitable as the
camera reference for the selected interior's drawing conventions.

- Establish a coherent whole-scene image as a visual target before judging a
  reusable exterior kit. The temple began as a generated room plate; its selected
  playable plate was not passed through a neural Pixel Fixer. Whole-scene
  composition and separable runtime layers are compatible: the composition owns
  no collision or geography. Compare the assembled native view to the target,
  especially ground quietness, contact and material transitions.
- Start from current project-owned geography and guidance. Preserve entrances,
  traversable widths, room functions and service positions. Art does not author
  collision by implication.
- Use rich material colour, warm inhabited light and restrained wear. Put wear
  where people walk or touch things. Quiet areas let characters and routes read.
- Request ground without a competing regular tile pattern. The visible tactical
  grid is a separate layer derived from observed cells.
- Generate reusable people, trees, facades and props in isolation. Assemble
  sheets from validated sources; do not ask a generator for a production atlas.
- Keep actors out of environment plates. Future animated foliage, lights and
  effects need separable layers. Avoid permanent baked actor shadows.
- A plate can convey depth with floor and selected top planes. Upright subjects
  present their readable fronts consistently, even when that cheats strict
  physical camera geometry.

The first [exterior slice](plans/2026-09-08-pixel-exterior.md) retains generated
surface textures and isolated architecture/tree sources alongside a composed
frontage reference. A generated checker can be removed by a recorded deterministic
exterior mask when it is separable from the subject: preserve the original,
classify the result as a prepared candidate, and inspect the alpha over light and
dark fields. Extraction is not a provider-produced transparent original or an
accepted master. A camera or texture cache must never become another ground or
occupancy owner.

## Connected exterior studies

Establish a continuous district composition against authored geography before
splitting it into view-sized pieces. Share camera, material scale, lighting and
road widths across the whole area. A flat plan supplies location constraints;
the selected art supplies upright presentation. Preserve open space rather than
letting the generator fill every region with decorative objects.

Exact crops of one source share continuity by construction. Prove complete
coverage and lossless reconstruction, then inspect scrolling across horizontal,
vertical and diagonal joins. Report actual source and per-view pixel dimensions;
a requested size or enlarged preview does not prove production-resolution detail.
An eventual independently generated extension needs overlapping context and a
preserved shared region, followed by the same native joining review. A continuous
concept painting alone does not prove that separate-generation workflow or
replace authoritative geography and depth-layer validation.

## Pixel correction and integer display

The [standard fixer](https://github.com/Retro-Diffusion/pixel-art-fixer) measures
an implied grid; the [neural version](https://retrodiffusion.ai/pixel-art-fixer)
reconstructs through a trained model. Record the engine, automatic or explicit
cell size, palette settings, exact input/output sizes and untouched downloads.
The public browser page inspected for the exterior trial states that standard
processing is local and neural processing uses its service. Do not infer one
engine's execution or privacy properties from the other's description.

Automatic grid detection is a hypothesis, not recovered author intent. Reject
results that crush detail, change aspect ratio or redistribute building widths
and open ground. A forced square output does not prove preserved geometry.
Explicit-grid reconstruction is a deliberately chosen sampling grid, not proof
that the input already had that grid. The [exterior record](plans/2026-09-08-pixel-exterior.md#pixel-fixer-comparison)
owns the tested outcomes and their limits.

Keep the original scene and compare identical regions at the intended display
scale. Export with nearest-neighbour integer enlargement and verify that every
output block repeats one native pixel exactly. This proves sampling only; it
cannot certify deliberate pixel clusters, restore missing detail, establish a
controlled palette or accept an artwork master. A composition-sized painting
may still need more detailed source art even after a technically valid pass.
The browser owner retains actual renderer and viewport behavior; a diagnostic
integer export does not change the game renderer. Keep separate the environment
pixel lattice and the source resolution needed by characters. Reducing a detailed
character to the coarser environment grid before enlargement destroys facial
information. Preserve the selected filtered character reduction and its finer
raster budget; explicitly exclude it from a coarse scenery-block assertion.

A coarser grid cannot by itself correct a mismatched painting style. The temple's
256-square trial lost detail and looked mushy; it was rejected. The subsequent
whole-room style correction uses the town as a style reference, retains room
functions and approaches, and is standard-corrected to 512 square. Retain rejected
samplings beside the source. For iterative room edits, inspect the actual opening,
landing, furniture silhouette and visible lamps before updating foreground/light
calibration. A generator's promise to preserve the layout is not that proof.

For a district composition with insufficient local detail, first choose a
logical viewport and generate a detailed neighborhood source from its exact
composition crop. Preserve the original district as the placement guide and
retain overlap for eventual adjoining scenes. Budget source detail per gameplay
view rather than enlarging the whole-district thumbnail. Request better-defined
existing forms, not extra texture; inspect grass and foliage quietness before
correction. Apply an explicit logical grid when automatic detection is plainly
wrong, and compare the corrected scene against both its new source and the
original composition. Regeneration does not preserve shared borders by contract:
prove matching overlap and authoritative placement separately before stitching.

For overlapping generated sources, choose assembly cuts away from principal
facades and retain the chosen masks or cut paths. A hard pixel selection keeps
the native grid intact; feathering can reintroduce softness. Inspect the actual
assembled streets, shoreline and canopy transitions before deriving exact
screen crops. Their reconstruction proof establishes the final crop set, not
identical independently generated overlaps. Preserve a selected reference region
explicitly where possible and prove the retained pixels.

Validate fractional-grid sampling as well as integer enlargement. The connected
study exposed negative triangular vote weights in the standard tool: source
pixels were assigned by their left/top edges while weights used their centres.
The local correction assigns sample centres to cells on both axes. Record this
as a modified standard implementation, retain upstream originals and the patch,
and prove nonnegative weights, bounded output colours, constant-colour stability
and exact recovery of an integer-enlarged source. The
[execution record](plans/2026-09-08-pixel-exterior.md#connected-standard-correction-study)
owns the observed reproduction and resulting artwork. A compact review may use
an explicitly labelled palette-reduced display copy; native acceptance compares
the full-colour source and exact exports.

## Character design and fine sprite preparation

Use natural adult proportions, a comparatively small head and understated eyes.
Preserve personality through posture, facial values and clothing rather than
large eyes or short limbs. The current class name remains **Martial Artist**;
class identity and naming belong to [classes and training](class-training-contract.md).

Tomas's useful identity is young, warm, tired and lean, wearing plain charcoal
working robes and the open hand. Maude can retain an older, fuller body. Similar
display height does not require identical build. No Christian crosses, church
vestments or invented religious lore.

The successful source workflow was a detailed GPT concept, proportionally
prepared for a 256px square Pixen input, followed by an inspected transparent
sprite and eight V3 facings. Keep the complete hair and both soles. Preserve the
original concept before any resizing; a large generated illustration is not
the same thing as a native low-resolution pixel master.

Do not enlarge already-coarse art and call it more detailed. Conversely, more
source pixels cannot preserve a face that the final layout reduces to very few
screen pixels. Judge the face beside the room and other characters at actual
display size. [Browser client](browser-client.md#pixel-art-presentation) owns the
larger desktop presentation and filtered reduction that addressed that loss.

## Alpha, outlines and light

Prompt for real transparent PNG alpha, no backdrop, floor, halo, checkerboard,
border or environmental shadow. Then **inspect the untouched output**. Several
apparently transparent GPT results were RGB images with a painted checker.
They remained disposable references; extraction created a different candidate.

Require RGBA with both transparent and non-transparent pixels. Composite each
candidate over a light and a dark field at source and native display sizes.
Check for fringe, cut-off hair, stray pixels, opaque holes and grey/white matte.

Prefer thin, local-material-coloured edges over a continuous black contour.
Preserve dark accents where forms overlap; a uniformly black perimeter makes a
figure look pasted onto a differently shaded room. Merely replacing black with
a bright colour is also wrong: the first Tomas edge edit produced an orange rim
and was rejected. Neutral diffuse front light and subtle charcoal/brown edges
gave the quieter selected result. Filtering helps sampling; it cannot rewrite
incompatible source linework.

## Shared scale, feet and occlusion

Each room has one authoritative grid for every player, resident and creature.
The projection is presentation; a camera or visibility subset cannot create
different cell identities. Use the same projection for ground ink, routes,
pointing, contact anchors and scene placement.

For pixel shaders, retain explicit height, normal, emission and selected-foliage
maps beside the colour image. Author their meaning deliberately; do not derive
collision, object removal or physical normals from a colour plate. Preserve the
same occlusion and ground anchors in colour and effect maps. A baked tree can
support masked internal leaf motion, but removing it later also needs clean
background art and a server-owned tree state. Do not claim chopping readiness.

Keep separate the source canvas size, measured opaque body height, authored
foot contact, visual footprint and authoritative occupancy. Transparent padding
does not make a character physically small. Hair buns and lifted feet affect
opaque bounds, so review the resulting proportions rather than trusting a
normalization number alone. Compare characters to beds, tables, doors and trees.

For town-scale review, compare unscaled entrance crops side by side with the
same adult-height reference. Measure the clear opening separately from its
frame, arch and steps. Review the [whole-building scale correction](presentation-direction.md#relative-scale-ruling)
in the complete scene afterward: a good isolated doorway cannot prove that the
surrounding building and approach still fit. Retain before/after evidence and
reconcile any changed visual extent with authored lots before promotion.

Draw rear ground and structures, then depth-ordered figures and foreground
occluders. The temple currently repaints authored furniture regions over grid
ink and actors behind them; this does not alter collision. Exterior construction
needs explicit wall, tree and roof occlusion instead of a monolithic image that
cannot place a character behind anything. Ground contact remains fixed while
upright facades, trunks and bodies use their readable facing.

## Movement and animation continuity

- Preserve the committed server route through the receipt/ready-frame handoff.
  An ordinary resident update must not erase that route.
- Interpolate along every route segment, including corners. Advance gait by
  distance travelled, not an unrelated wall-clock loop. Server readiness remains
  authoritative; visual completion cannot unlock actions.
- Use a fixed body scale for one identity across its gait. Do not resize every
  walking frame to equal opaque height: that removes intended weight shift.
- Keep a standing-direction ground pivot across that direction's walking frames.
  Recomputing the anchor from the lowest visible boot or hem each frame caused
  an eleven-pixel source pivot swing and a visible hitch.
- Inspect the loop boundary as well as individual frames, especially walking
  toward the camera. A valid sheet can still contain an uneven stride.
- Generate from the **final** repaired sprite. The Martial Artist's original
  provider state contains rejected ankle hardware; another full-state repair
  softened his upper body. Neither represents the selected repaired PNGs.

## Focused corrections without identity drift

Prefer the smallest edit that can solve the defect. Preserve the original and
retain rejected attempts with a reason. For the Martial Artist ankle repair,
the successful method packed original sprites into two masked sheets, permitted
changes only around the ankles, then assembled the generated pixels inside the
mask and original pixels outside it. Byte comparisons proved both regions.
The provider had changed eight pixels outside its promised mask; its claim was
not proof. Preserve alpha during assembly and inspect all facings afterward.

Do not accept a technically successful correction that changes the face, torso,
clothing detail or silhouette as part of an unrelated repair. A class character must remain
recognizable across facings, animation and later equipment work.

## Native review and durable evidence

For each selected candidate, retain the exact prompt, project-owned references,
untouched outputs, provider/job identity, transformations, masks, hashes, costs,
selected packet, rejected alternatives and specific reasons. Never include
credentials in those records. Do not attribute every part of a mixed pipeline
to one provider or invent the built-in model version.

Show actual room captures and unscaled comparison crops. Label enlarged
diagnostics and synthetic scene compositions explicitly. Use the real browser,
server and direct interaction path to check scale, foreground occlusion, shared cells,
movement, service access and reconnect. Missing assets or stale bindings must
refuse loading. [Verification](verification.md) owns the runner and browser matrix.

Separate measured local rendering costs from product performance promises.
The chosen look does not imply measured exterior, crowd, low-end-device or
animation performance. Record those checks as the corresponding work arrives.

## Exterior integration

The connected town replaces the isolated frontage construction study. Its
[execution record](plans/2026-09-08-pixel-exterior.md#gameplay-integration-and-pixel-effects)
owns the bounded map amendment, prepared layer packet and real-game proof. Keep
subsequent room and island work tied to their authoritative geometry and compare
characters, material scale, entrances and atmosphere in native gameplay views.
Unillustrated rooms remain explicitly labelled maps during construction.
