---
last_updated: 2026-09-03
revision: 3
status: Implemented 2026-09-03 (PR web/live-figure) to the approved spec — packet schema 4 with the figures group, the caretaker as a live rig decoded from verified bytes, the palette material, idle clip, two-way facing, no contact blob; proven in both engines. Client notes revision 20 is the implemented truth; this document is the record of the plan and its open list (§6, §6a).
public_safe: true
summary: The caretaker becomes a live rigged mesh in the feel client — rig, outfit parts, and clip library carried by the packet under the same digest discipline as every sheet, the painted grammar reproduced in the character's material, the card retired — with every claim labelled decided, proposed, or open.
---

# Live figure rig — spec and plan

## 0. Authority and inputs

**Decided (owner, 2026-09-03), recorded in
[presentation direction](../presentation-direction.md#the-in-engine-feel-scene)
and [settled conclusions](../settled-conclusions.md):** characters are live
rigged meshes in the client; equipment is skinned parts on the shared
skeleton; motion is animation clips; the painted grammar is reproduced in the
character's material. The card ruling of 2026-09-02 stands for every other
class. The desktop is the web client in a Tauri shell; every real-tab proof
runs in Chromium and Firefox.

**Evidence this spec rests on:** the lab (`camera-v36`, rounds three and
four, 2026-09-03), where the CC0 Quaternius universal rig ran live under the
ruled camera with outfit parts and library clips, and a nearest-colour lookup
into the figure's treatment-P palette with a rim of 0.25 placed it beside the
treated furniture at play size without reading as a different medium. Lab
evidence is evidence, not implementation: nothing crosses from the lab but
the conclusions written here and the packet content the lab builds.

**Owners of the facts this slice changes:** the packet contract and the
client's implemented behaviour — [client notes](../client-notes.md); the
client's standing contract — [client architecture](../client-architecture.md)
(no change proposed); the visual target — presentation direction (already
ruled). No gameplay fact moves: the caretaker's cell, facing, route, and
landing are unchanged, and the rig is presentation of a state the walk
presenter already holds.

**What the repository already proves and this slice migrates:** the caretaker
is placed by the client as a `props/caretaker` card at `card_height` 1.38 at
`start`; a packet placement naming `caretaker` is refused; the walk presenter
moves one object (`caretaker.card`) and mirrors it by facing; the walk proof
asserts the caretaker's centring by projecting that object. Tests naming the
caretaker card (`feelScene.test.ts`, `manifest.test.ts`, the walk proof) are
migrated in this slice, under the no-half-migration rule.

## 1. The question

Can the caretaker stand in the feel scene as a live rig — carried by the
packet, verified like every other asset, lit by the scene, dressed by parts,
moved by clips, and coloured in the cards' grammar — with the card gone and
every proof green in both engines?

## 2. The packet contract (schema 4)

**Decided (owner approved the shape, 2026-09-03).** The manifest gains one group,
`figures`, beside `terrain`, `walls`, `props`, `roofs`. A figure row:

```json
"figures": {
  "caretaker": {
    "rig": { "file": "figure-caretaker.gltf", "sha256": "…" },
    "sidecars": [ { "file": "figure-caretaker.bin", "sha256": "…" },
                  { "file": "figure-caretaker-basecolor.png", "sha256": "…" } ],
    "clips": { "file": "figure-clips.glb", "sha256": "…" },
    "parts": [ { "file": "part-peasant-body.gltf", "sha256": "…",
                 "sidecars": [ … ] } ],
    "palette": [[13,7,7], [18,10,8], …],
    "rim": 0.25,
    "idle": "Idle_Loop"
  }
}
```

- **Every file is digest-bound**, rig, sidecars, clips, and parts alike; a
  mismatch refuses the packet, as today. Sidecars exist because glTF names its
  buffers and textures by relative URI; the client resolves those names
  **only** against verified bytes (a loading-manager URL modifier onto blob
  URLs of the verified payload), so no unverified fetch can occur. A glTF that
  names a file the row does not carry is refused at decode time, not at draw.
- **`palette`** is the figure's 28-colour treatment-P palette, inline: it is
  small, it is content, and it must travel with the figure it was derived
  from. `rim` is the silhouette darkening. Both are the material's inputs.
- **`idle`** names the clip the figure plays when the walk state is idle.
  Which other clips exist and when they play is open (§6).
- **The top level gains `caretaker: { figure: "caretaker" }`**, naming which
  figure the start places. A packet without it is refused; the client no
  longer carries a caretaker height constant.
- **Retired in the same cut:** schema 3 is refused by name; `caretaker`
  leaves `REQUIRED_PROPS`, so a packet no longer needs a caretaker sheet, and
  a packet still carrying one is accepted only as an ordinary listed prop
  that nothing places. The synthetic test manifests, the lab world builder,
  and the served packet move together.

## 3. The client

- **`web/src/space/figureRig.ts` (new)** owns the figure: decode (glTF +
  sidecars + clips + parts from verified bytes), the palette material patch
  (three's physical material at `opaque_fragment`, refusing to build if a
  three upgrade moves the anchor, as `cardLighting.ts` does), the mixer and
  clip selection, and the object the presenter moves. `SpaceScene.ts` (903
  lines) gains only the call sites; the refactor threshold is respected by
  not growing it.
- **Placement:** the rig's root at the caretaker's cell centre on the ground
  (the same anchor the card had), scale one metre per world unit as ruled.
  Facing rotates the rig about world up: the card's mirror becomes a yaw of
  0 or π (decided 2026-09-03; eight-way facing by route direction is open).
- **Lighting:** the scene's lights and shadow map act on the rig directly;
  `castShadow` and `receiveShadow` on every mesh; the caretaker's contact
  shadow blob is removed (the rig casts its own; decided 2026-09-03). The card
  wrap (`CARD_DIFFUSE_WRAP`) does not apply — the rig has real normals. Decided
  2026-09-03: the palette lookup runs after lighting, in gamma space, luminance-weighted
  nearest colour, rim from the view-space normal; block grain not included.
- **Motion:** an `AnimationMixer` per part, the same clip on each (parts
  share the skeleton by name), ticked in the scene's update. Idle plays
  always in this slice (decided 2026-09-03). Movement still lands whole on the strike; no walk
  cycle is played here — the walk between pulses is the next slice (§6a),
  not this one, because it changes a settled presentation row and wants its
  own test.
- **Proof surfaces:** the stage carries `data-caretaker-figure` (the figure
  key once loaded) and `data-caretaker-clip`; the walk proof asserts both in
  both engines and keeps its centring assertion by projecting the rig's
  root.

## 4. The lab side

- A packet builder step that emits the `figures` row: copies the rig, its
  sidecars, the clip library, and the chosen parts from the CC0 kits into the
  packet directory with digests, and derives the palette by the treatment-P
  path from a render at the ruled camera (the lab's `render-figure.mjs` and
  `treat2.py` functions, as in `camera-v36` round four).
- Provenance recorded in the packet's own notes: Quaternius Universal Base
  Characters, Universal Animation Library 1, Modular Character Outfits —
  Fantasy [Standard]; each CC0 1.0; tier 5 material, never in the tree.
- The served packet regenerated at schema 4 and deployed with the client in
  one cut.

## 5. Proof

- Unit: manifest parsing of the `figures` group and the top-level
  `caretaker` (accepts a complete row; refuses a missing sidecar, a bad
  digest, an unknown idle clip name is *not* checkable at parse and is
  refused at decode); the material patch's anchor refusal; the URL modifier
  refusing an unlisted name.
- Real tab, both engines: the walk proof against the served packet with
  `data-caretaker-figure` asserted, all landings and portal crossings as
  today; captures at night and dusk, courtyard and room, for the owner's
  compare beside the last card frames.
- Fast lane over every changed path; the web lane in CI.

## 6a. Proposed next slice — the walk between pulses

**Owner direction, 2026-09-03:** nice animations are what we should do, and
that means revisiting the walk. The authority does not move: the server
snaps the character to its cell on the strike, exactly as today. What
changes is presentation only — during the pulse, after a route is
committed and before the strike lands it, the client plays the walk clip and
carries the rig along the committed route so that it arrives on the target
cell as the strike lands. The presenter still snaps to the authoritative
cell on the strike; if the landing differs from the route it was walking,
it snaps and the walk was a lie the pulse corrected, never a position the
game believed.

This supersedes, if it passes its test, the standing "the figure stands on
its square until then" wording of the movement row in settled conclusions;
the boundary map already permits presentation to remain fluid between
beats and forbids it to imply a walkability it did not receive — a rig only
ever walks a route the client holds as committed. Proof: a capture sequence
across one pulse in both engines, and the walk proof's landing assertions
unchanged. The owner judges the feel; the test may fail it.

## 6. Open

- Which clips beyond idle and walk, and what triggers them (landing, attack,
  spell, death, sitting). The mechanism is decided; the extent is the owner's.
- Eight-way facing by route direction versus the two-way mirror carried over.
- Whether the block grain is wanted on the live figure at play size.
- The caretaker's own look: this slice ships the bare universal rig in the
  peasant parts as the stand-in; the dressed caretaker waits on the character
  source pipeline (Meshy resub or a modelled base body).
- Whether any other class crosses to live 3D (none pending; the hearth is
  the precedent).

## 7. Closeout (2026-09-03)

Implemented as specified, with two facts learned on the way that the spec
did not foresee and the implementation records:

- The sheet decoder decoded every verified asset as an image; a figure's
  glTF, buffers, and clip library now sit under `figures/` keys and the
  decoder skips them (`isFigureKey`), with a test.
- The facing yaw is ±π/2 about world up — the rig's +z front turned along
  ±x, the axis the card mirrored across — rather than the 0/π the spec
  wrote; two-way either way, and the presenter's contract is unchanged.
- The packet grew by 17 MB (the clip library is 7 MB; the kit's 4K textures
  are downscaled to 1 K in the lab's packet step, since the palette collapses
  their colour anyway).

Proof, at the reviewed tree: 140 unit tests (the review added refusals for
an unlisted file, an unpatchable material, a clip that does not bind to
skeleton bones, a part on another skeleton, a palette over 32 colours, more
than 16 point lights in a space, and disposal of decoded sources and cloned skeletons); fast
lane COMPLETE; captures in both engines at night and dusk in the courtyard,
the room photographed by the walk proof's interior capture; walk proof PASS
in Chromium and Firefox with `data-caretaker-figure` and
`data-caretaker-clip` asserted and both portal crossings walked. Draw calls
501 → 527 with the figure's six meshes. On the served packet the idle
clip's 65 targets are all skeleton joints on the rig and on every part.

## 8. Stop conditions

Stop and report rather than widen if: a verified glTF cannot be decoded
without an unverified fetch; Firefox on software GL cannot skin the rig at a
usable frame time; or the palette material cannot be patched at a stable
anchor in the pinned three version.
