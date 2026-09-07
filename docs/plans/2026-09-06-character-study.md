---
last_updated: 2026-09-07
revision: 3
status: Cleaned character candidate integrated and verified in the private study across all three engines; equipment and final motion cleanup remain open.
public_safe: true
summary: Selected character guidance, provider candidate cleanup, private study integration, source inventory, loader findings and remaining visual work.
---

# Character modeling study

This Planning record owns the bounded character trial and its evidence.
[Presentation direction](../presentation-direction.md#stylization-and-restrained-charm)
owns the owner's visual amendment and subsequent preference for the first
generated adventurer. The [live-character ruling](../presentation-direction.md#live-characters)
owns the runtime medium, modular equipment and animation contract.

## Selected guidance

Two original isolated adventurers were generated with the built-in image tool.
The owner selected the first figure after the second pushed the character
further toward cartoon proportions. The initial implementer judgment that the
first was too realistic is superseded by that explicit preference.

Selected untouched image SHA-256:
`727dc0c9ca73fc6e4139ac668d4903d8f0e359e8f134b009b2c7f0e9431db078`.
The second image is retained as comparison history, not the modeling target.
Images, exact prompts, modeling sources and inspection evidence remain outside
the checkout. The guide is no runtime dependency or accepted production master.

The selected file is RGBA with both transparent and non-transparent pixels.
Inspection over light and dark backgrounds at source size and at 80/120-pixel
figure heights found a clean isolated silhouette. These small figure comparisons
test image readability; they do not substitute for a live model at game scale.

## Original Blender trial

Two original procedural studies explored the outfit, proportions and rig. The
second adjusts toward the selected adult silhouette. The editable source,
Blender file and self-contained GLB retain separate clothing geometry, an
18-bone skeleton and four original motion sketches: idle, walk, run and sprint.
No downloaded character mesh or generated-image texture was incorporated.

**Visual finding:** the trial remains below the selected guide. Facial forms,
hair locks, sleeve construction and cloth shaping still read as assembled simple
volumes. The model is retained as a disposable modeling/rigging experiment and
has not replaced the game's existing character. Increasing geometric detail
alone is not evidence that this visual finding is resolved.

The current trial has 93 skinned mesh objects and 26,328 triangles. These are
observed draft counts, not an accepted character budget. Clothing is separately
editable but has not been packaged as independently swappable runtime parts.
Foot planting, joint deformation, clip quality and mesh consolidation remain
work for the character-source pipeline before promotion.

## Initial capability and proof

At the original Blender trial, the available image-to-model integrations both reported disabled. Configuration
presence checks found no provider credentials or configured local generation
endpoint. The default loopback address answered a generic health request but
had no generation route; that response is not a usable model service. No
provider request, subscription or integration-setting change was made.

Original Blender generation and GLB export completed. A standalone Three.js
inspection in hardware Chromium loaded the final GLB, found all 18 bones and
four clips, and observed skinned-vertex movement in every clip without page
errors. Captures include a zero-yaw, 45-degree camera and a small-scale view.
This proves the bounded asset experiment; it is not served-game integration,
the repository's complete three-engine proof or visual acceptance.

The original trial recommended using the selected first guide as the input
for an image-to-model comparison, followed by Blender cleanup, rigging, modular
outfit preparation and native-scale review. The subsequent dispatch below
supersedes that trial's unavailable-capability checkpoint.
The current primitive-based trial supplies useful export and rig evidence but
does not earn adoption as the character design.

The broader environment refinement remains routed through presentation direction
and the [town execution record](2026-09-06-town-buildout.md). This character trial
changes no temple packet, accepted geography, gameplay or deployment and performs
no Git lifecycle operation.

## Meshy trial

**Owner dispatch, 2026-09-07.** The owner obtained Meshy, supplied an API
credential and pointed to its official agent-integration documentation. This
authorizes a bounded provider trial using the already selected original guide.
It does not accept the generated model as an artwork master. Credentials and
all provider payloads remain external; no repository or runtime dependency on
the service was introduced.

The trial uses Meshy 7 standard image-to-3D, A-pose, 4K base-colour textures,
PBR maps and approximately 30,000 remeshed triangle faces. Input-image
enhancement is disabled to preserve the selected appearance. The unmodified
provider mesh before remeshing is also retained. Original generated output,
rigged GLB/FBX, walking/running GLBs and separate animation-armature GLBs are
downloaded before provider expiry. The rigging request uses a 1.7-metre study
height; this is a candidate scale, not a new global character-height ruling.

Observed credit use is 30 for generation and 5 for rigging. API balance changed
from 3,500 to 3,465; the two task receipts account for the full difference.
Meaningful request parameters, exact input hash, task identities, untouched
downloads and editable Blender studies are retained in the external receipt.

Generated GLB SHA-256:
`e849fad2c42532faa1daa1fba781bf9557fe660e19afa3a38755987a55cce947`.
Rigged GLB SHA-256:
`1179d73db9701e54fd80a02bef0156c2a4f1024f10920dde37b82c115fc456b6`.

**Visual assessment:** the generated candidate preserves the selected figure's
adult proportions, face, swept hair, layered outfit and repairs substantially
better than the original procedural trial. Native views include the inferred
rear of the outfit, rather than judging only the matching front. It remains an
unaccepted candidate pending the owner's visual review.

**Geometry findings:** the static result has one mesh with 31,243 triangles;
rigging changes the exported mesh to 31,237 triangles and adds 24 bones.
The static GLB's many split boundary edges largely represent UV/normal seams.
An audit-only coincident-vertex weld at tolerance 0.000001 leaves one connected
component, 26 boundary edges and 41 nonmanifold edges, with finite positions and
no degenerate faces. The audit changes no downloaded geometry. The character
pipeline owns locating and resolving those remaining geometric defects before
promotion, with a repeated connectivity audit and visual comparison as proof.

The combined outfit is not yet independently swappable equipment. Shared
skeleton integration, equipment separation, hands and joint deformation,
foot planting, runtime material treatment and texture/memory budgets remain
character-source work. The provider's successful rigging task settles none of
those production questions.

### Material correction and final inspection

The provider's rigging export discarded the generated normal and
metallic/roughness maps and added full-strength colour-texture emission.
Prepared copies restore the original generation's PBR material and remove that
emission. Every target UV belongs to the original UV set at the audit's
five-decimal precision; the provider's converted base-colour image also retains
the original layout. The preparation proves identical geometry-accessor bytes,
nodes, skins and animation data. Unused provider texture payloads are removed
when rebuilding the self-contained GLB; untouched downloads remain separate.

Prepared rigged GLB SHA-256:
`f70bd99ed43b0bc3ef00b997b6f190963227369df2f6cc145ceca3cbc2eca805`.
Prepared walking GLB SHA-256:
`e5b03a80513daceecaa42f23ac09b30418fa151871daf5d7031393f4a2cf56b8`.
Prepared running GLB SHA-256:
`1c4f90821450baffc78b30bc30fb3e0fd0dd22d0066b4a06c38ce3624b9c05a0`.

**Observed proof:** the static model and final prepared walking/running models
loaded in hardware Chromium, Firefox and WebKit without page errors. Both
motion clips produced finite changes in skinned world-space vertex positions
in every engine. Final materials have normal and metallic/roughness maps and
zero emission. This is standalone asset inspection, not served-game integration
or artwork acceptance. Native captures include front, rear, three-quarter,
fixed-camera and small-scale views; a two-second capture records the prepared
walking clip. Editable Blender sources preserve the prepared rig and materials.

Inspection fixes are retained in the external evidence: engine configuration
objects initially overwrote capture filenames, and Blender's bone-display helper
initially entered the rigged model's scale calculation. Final captures use engine
names and exclude bone-display helpers. Motion measurements explicitly transform
skinned vertices into world metres rather than reporting provider-local units.
These are capture-tool corrections and change no downloaded character geometry.

## Client integration and sourcing pass

**Owner dispatch, 2026-09-07.** Continue the adventurer and source suitable
assets from the retained collection, CC0 libraries and the available provider.
The [sourcing ruling](../presentation-direction.md#asset-sourcing-order) owns
the standing selection order. An external searchable inventory records 1,152
local glTF/GLB files, including alternative exports, and six included licence
receipts. It distinguishes discovery from completed per-asset provenance and
visual review. Current official source pages were checked; no new community
model or CC0 asset was downloaded during this pass.

### Prepared character packet

The rigged mesh has 13 stray triangles, each joined through one three-face edge
with two boundary edges. Removing only those triangle indices leaves 31,224
triangles and zero open boundary edges. Position, UV, normal and skin-weight
attributes retain their source values. Repeated Blender inspection confirms
two remaining four-face surface contacts, approximately 4.5 and 4.6 millimetres
long. These remain explicit production-retopology findings rather than a claim
of manifold geometry. The original provider downloads are unchanged.

The candidate exports one clothed rig, its verified binary and three separate
1K PNG material maps, plus a compact four-clip library. The original high
resolution materials remain in the external source archive. The balanced
32-colour candidate palette includes separate cloth, leather, skin and linen
ramps; the material remains responsive to scene lighting with no emission.
An editable Blender file retains the cleaned mesh, skin, materials and all
four actions.

Walking and running use the existing provider clips. Two additional requests
supplied an initial idle and a sprint. Native inspection found the first idle
too braced for ordinary town activity; a third request supplies a relaxed idle.
The initial idle remains comparison evidence. These three requests consumed
nine credits; the observed balance is 3,456. Clip choice changes no authoritative
action deadline or movement availability.

The private study artwork receipt selects the character packet while retaining
the accepted geography binding and previous temple model. Its sole manifest
change from the modeled temple packet is `figures.caretaker`; all existing
packet asset bytes remain identical. No land layout, service location,
geography promotion receipt or artwork-master acceptance changes.

Candidate asset-manifest SHA-256:
`51eab2b8be8f387b0d5b7f8120ef57698e0aee79106b4e124ee6ad14f4b60846`.

### Integration findings

**Resolved export mismatch.** Embedded texture URLs do not belong to the
figure's verified filename table. The loader could catch that texture failure
and still produce an almost black material. Separate PNG sidecars follow the
existing packet contract; JPEG sidecars are refused by its format validator.
The client now preserves dependency-resolution and load failures across a
successful loader callback and refuses the figure, disposing parsed sources.
[Browser client](../browser-client.md#figures-and-structures) owns that behavior.

**Remaining character-source work.** The clothed body is a visual candidate,
not independently swappable equipment. Production work must supply clean body
and clothing boundaries on a shared skeleton, resolve the two surface contacts,
and review joint deformation and foot planting across the full action set.
Proof requires equipment swaps without gaps or intersections and native motion
review, not merely matching bone names. Texture and triangle counts here are
measured candidate choices, not global accepted budgets.

The native decoder/instancer probe observes all four distinct clips and all
three 1K maps with zero emission in hardware Chromium, Firefox and WebKit.
Each clip produces finite world-space deformation. The sampled lowest geometry
point stays approximately 5 millimetres above the nominal floor during idle;
walking reaches 11 millimetres below it, running 25 and sprinting 8. These
penetrations require foot-contact cleanup by the character-source owner. The
airborne portions of running and sprinting are not themselves floor defects.
Both deliberately unlisted and corrupt texture inputs are refused by the real
decoder in all three engines, complementing the loader-success regression tests.

**Remaining actor presentation.** The private study still uses a shared fallback
figure for actors without their own modeled resident. That existing placeholder
also receives this candidate, so identical figures can appear together. The
client presentation owner must replace the fallback with explicit actor visual
selection and native proof for each role before the town cast is production art.
Class, identity and gameplay state remain server-owned.

### Native integration result

Hardware Chromium, Firefox and WebKit each completed 55 ordinary UI commands
against a fresh PostgreSQL database and the actual first-expedition world.
The proof covers character creation, dock movement, temple entry, Tomas access,
balm purchase, reconnect with preserved inventory/gold/location, dungeon
descent, temple and town return, and logout. No page or console errors occurred.
The authored-world file hashes were identical before and after. Captures record
the real private play canvas; the separate decoder probe and non-authoritative
temple inspection are labeled as such in their external receipts.

This is local candidate integration. No Git lifecycle, remote preview update,
deployment or artwork-master promotion was performed.
