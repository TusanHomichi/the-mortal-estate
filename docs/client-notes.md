---
last_updated: 2026-09-05
revision: 24
status: Browser scene pass 03 deployed to the existing private preview; individual cooldown implementation verified by the real client/server proof; local regression closeout complete.
public_safe: true
summary: Current browser scene experiment and retained client implementation, including independent cooldown presentation and authoritative frame proof.
routes:
  - web/**
  - client/**
  - tools/run_client_live_proof.py
  - tools/run_pulse_capture.py
---

# Client notes

The client is a thin, renderer-neutral shell over the authoritative wire. It
owns intent, reconciliation, and presentation of authoritative state; it owns no
gameplay rule and no world fact of its own. This document records the decisions
that shape it, in the order someone new to the tree needs them.

The standing architecture these decisions sit inside is
[client architecture](client-architecture.md), and the visual target is
[presentation direction](presentation-direction.md). Server-side counterparts
live in [server notes](server-notes.md).

## Current implementation

| Surface | Implemented | Still outside it |
| --- | --- | --- |
| Browser (`web/`) | Three.js candidate-packet scene, local route drafting and individual movement cooldown, spaces and portals, live figure rigs and gait clips, lighting, wind, and camera comparison | Authentication, authoritative wire consumption, the accepted chrome/action interface, and its proportional layout |
| Retained Godot shell (`client/`) | Sign-in, server wire codec, command reconciliation, diagnostic world view, and live/capture harnesses | Accepted production art and the browser feel role |

The browser is the active feel surface. Its local passability and action timing are
experimental inputs, not gameplay authority. The September 3 chrome, actions,
and scaling rulings are [targets](presentation-direction.md#chrome-and-actions),
not completed features. `web/src/main.ts` still opens the feel scene directly,
and `web/src/camera.ts::resizeFeelCamera` fixes vertical extent while deriving
horizontal extent from aspect ratio. Its vertical frame now derives from the
ruled nine cells (`9 × √2 × sin(30°)` world units); equal world extent across
display shapes and the proportional interface remain for the layout slice.

Unless a section explicitly says browser, the wire, credential, HUD, cooldown, and
capture implementation below describes the retained Godot shell. Its continued
proof does not establish that the browser implements those contracts.

## Browser operation and proof

From the repository root:

```bash
npm --prefix web ci
TME_FEEL_ASSETS=/absolute/path/outside-checkout npm --prefix web run dev
```

Use the loopback URL printed by Vite. `web/vite.config.ts` serves candidate
assets only for the dev server, from an absolute directory outside the checkout.
A static `web/dist/` build does not package or serve that private packet. An
absent or refused packet produces the scene's absence banner.

The browser uses query parameters, for example `?preset=night,wind&zoom=-1`;
`web/src/presets.ts` owns their vocabulary and bounds. `TME_FEEL_PRESET` belongs
to the retained Godot experiment, not this browser entry point.

The [standing web lane](verification.md#browser-evidence) uses synthetic
fixtures. These optional real-tab commands additionally need a candidate packet
and Playwright's installed Chromium and Firefox, plus `DISPLAY` or Xvfb for
Firefox. [The walk experiment](#the-web-feel-scenes-walk-experiment) records
the engine launcher and incomplete-run behavior:

```bash
TME_FEEL_ASSETS=/absolute/path/outside-checkout node web/proof/walk-proof.mjs
TME_FEEL_ASSETS=/absolute/path/outside-checkout node web/proof/capture-packet.mjs \
  --out /absolute/capture/output --query preset=night --width 1280 --height 800
```

The walk proof uses a 1280 × 800 viewport and writes to `TME_CAPTURE_OUTPUT`
or a named temporary directory. The capture command requires `--out` and accepts
repeated `--query` values and explicit dimensions. These captures are evidence
for review; passing them cannot accept art or close an owner gate.

## The credential model (owner ruling D7)

The client persists no credential. There is no saved-username file, no
plaintext-password opt-in, no remembered-login record, and no read of any
predecessor credential path. `client/config/dev_credentials.gd` is the whole
model:

| Variable | Meaning |
| --- | --- |
| `TME_EX_USERNAME` | sign-in username for this run; trimmed |
| `TME_EX_PASSWORD` | sign-in password for this run; taken exactly, since its edges may be meaningful |

Anything not supplied there is typed once per run and never leaves memory. The
password reaches exactly one place — the masked field on the sign-in screen —
and `client/adapter/secret_redactor.gd` keeps it out of transport errors and
debug summaries. `client/tests/test_secret_redaction.gd` proves both halves: the
transport redaction, and that resolving credentials writes no user-directory
file at all.

The namespace is the same `TME_EX_*` one the endpoint resolver uses, and it is
the client-side sibling of the server's credential files under
`CREDENTIALS_DIRECTORY`. This is a private-development model. A durable
credential store, if one is ever wanted, is a separately accepted design that
uses a real platform secret facility — not a fallback behind this one, and not a
revival of the predecessor's file.

The sign-in screen therefore has no *Remember password* checkbox, no *Forget
saved login* button, and no credential-status line that reports storage state.
One label states where this run's credentials came from, and it never quotes
one.

## One world on the wire (owner ruling D4)

The client speaks the post-cut wire. No envelope carries world identity, because
there is one world:

| Retired | Current |
| --- | --- |
| `facet_id` on any envelope | absent |
| `observed_facet_revision` | `observed_world_revision` |
| `facet_revision` | `world_revision` |
| `POST /v3/characters/switch-facet` | absent |
| `facets` in the session bootstrap | absent |
| `facet_id` on a character summary | absent |
| `RejectionCode::wrong_facet`, `future_facet_revision` | absent, `future_world_revision` |
| draining reason `facet_transfer` | absent |
| control errors `facet_not_available`, `transfer_rejected` | absent |

The player-visible world chooser is gone with the wire it spoke: character
selection continues straight into the world shell, and the only route out of a
session is sign-out or reconnection.

Two guards changed shape rather than disappearing, because their reason
survived the identity they used:

- **Accepting a welcome.** The connection state machine used to require the
  welcome's world identity to equal the selected character's. It now requires
  the account to have selected a character that the session bootstrap actually
  lists. The admitted socket is the binding; the selection is what must be true
  for the binding to mean anything.
- **Accepting a state update.** The authoritative state used to require the
  update's world identity to equal the welcome's. It now requires a
  welcome-installed frame to exist and the server sequence to advance. An update
  arriving without authority, or out of order, is still refused.

Internal server vocabulary still says "facet" in roughly 1,200 places; that
rename is recorded by private-archive issue #4 and does not reach the client,
which no longer uses the word at all.

## The wire fixture corpus is read, not copied

`tests/fixtures/wire/` holds fifteen fixture files and is a contract shared by
both sides of the wire. `crates/tme-protocol/src/client_fixture_tests.rs`
asserts its exact inventory from Rust; `client/tests/test_wire_codec.gd` asserts
the same inventory and runs every case through the GDScript codec.

The client reads that one corpus rather than keeping a copy, because two copies
of a contract drift and the drift shows up as a passing test on each side. It is
reached as an absolute path derived from the project root
(`TestSupport.wire_fixture_root()`), not as a `res://`-relative one: Godot
normalises `res://..` away inside `DirAccess` while `FileAccess` follows it, so
a relative path would silently list the Godot project directory and the
inventory assertion would pass against the wrong directory. Tests never ship —
the export presets exclude `tests/*` — so a source-tree path is honest here.

The corpus deliberately keeps retired predecessor shapes inside **reject**
cases, so the client proves it refuses them:

- `reject_retired_facet_directory`, `reject_retired_facet_id_field`
- `reject_retired_facet_id_on_command`, `..._on_path_preview`,
  `..._on_social_message`, `..._on_state_update`
- `reject_retired_observed_facet_revision`, `reject_retired_facet_revision_field`
- `reject_retired_facet_not_available`, `reject_retired_transfer_rejected`
- `reject_retired_wrong_facet_code`, `reject_retired_draining_facet_transfer`

Those literals must survive any future rename.

## The world-view seam

`client/presentation/world_view_seam.gd` is the one seam between the world shell
and whatever draws the world. The shell owns intent, reconciliation, and every
HUD surface. A view owns two things the shell cannot do for itself: turning an
authoritative frame into something a player can see, and turning a pointer
position into a semantic target.

Everything the seam asks for is expressed in world squares and frame rows —
never in meshes, cameras, or pixels. Where the world may be drawn is the shell's
decision, not the view's: `WorldViewHost` in `WorldShellScreen.tscn` is inset to
clear the HUD's top rail, cue banner, resource plate, and edge launchers, so a
view's own text stays legible and no addressable rectangle sits under a button.
A view is never told about the HUD; it is given a rectangle. The contract:

| Member | Meaning |
| --- | --- |
| `present_frame(frame, generation)` | the only way authority enters a view; an empty frame means "no authority" and must present as absence |
| `clear()` | drop all presented state |
| `observation_center()` | the square the frame is observed from |
| `semantic_target_for_coordinate(square)` | the target at a square, or empty |
| `semantic_target_for_display_position(position)` | the target under a pointer, or empty |
| `pointer_surface()` | the control whose local space pointer positions use |
| `present_cooldown(state)` | progress within the reported action interval; carries no readiness authority |
| `show_pending(start, path)` / `show_preview(preview)` / `clear_interaction()` | the movement-draft presentation |
| `set_reach_grid_transient_active`, `toggle_grid`, `grid_control_state` | the reach-grid preference |
| `present_feedback(kind)` | present one cue and return what was presented |
| `semantic_primary_pressed` / `_released` / `_pointer_moved` / `semantic_secondary_pressed` | pointer facts, in `pointer_surface()` local space |

Two rules bind any implementation. Targets a view emits or returns come from
`WorldTargets`, so the interaction director and the ground tray see one identity
space no matter what drew them. And a view never sends a command and never
mutates authoritative state: it emits pointer facts, and the shell decides what
they mean.

### `WorldTargets`: the neutral half of pointing

`client/presentation/world_targets.gd` derives, from an authoritative frame
alone, every addressable thing in the world: its identity, its square, and — for
corpses, loose items, and gold — how close the controlled character stands to
it. Ground state further than one square away is not addressable at all; ground
state within reach carries `examine` or `manipulate`, so no caller re-derives
distance. Results are sorted by identity, so two builds of one frame are
identical.

This is the owner the interaction director and the ground tray depend on. They
previously reached into the retired renderer's hit registry for the same
vocabulary, which is why neither was liftable as-is; pointing them at a neutral
owner is what made them renderer-independent.

The screen-space half of the old registry — anchors, hit shapes, containment
maths — retired with the renderer, and `GridWorldView` now owns it behind
`semantic_target_for_display_position`. The camera-depth half did not come back:
a flat lattice has no depth to order by, so draw order is the whole rule.

### `GridWorldView`: the current implementation

`client/presentation/grid_world_view.gd` draws the authoritative frame as a
square lattice of flat colours: one rectangle per visible square keyed by terrain
and passability, one marker per addressable thing standing on one, the controlled
character distinguished, and the reach grid and movement draft outlined over the
top. A banner inside the picture says exactly that — flat colours, no art, no
assets, a diagnostic lattice and not the game's renderer.

**It is not art direction and not the game's renderer.** It is the cheapest
honest presenter that satisfies the seam, and specifically the presenter
obligation the Workbench spec places on whatever view is current: for one
nominated frame, emit a capture plus an identity sidecar. A pixel-native renderer
substitutes for it behind `WorldViewSeam` and inherits that obligation unchanged.

Its targeting is real, and that is the whole reason it exists:

- every drawn thing is a `WorldTargets` target occupying an exact screen
  rectangle, held in draw order;
- `semantic_target_for_display_position` answers by walking those rectangles from
  the last drawn to the first, so the answer is the topmost thing actually under
  the pointer — never the nearest anchor;
- occupants sharing a square are laid out side by side inside it rather than
  stacked, so no two addressable rectangles ever overlap and no pixel has two
  owners;
- a target's **anchor** is a pixel that target actually owns. A square is
  overlapped by whatever stands on it, so its anchor sits in the strip above the
  occupant band. Every anchor resolves to its own target, by construction.

Two consequences are deliberate. **Only addressable things are drawn as
markers** — ground state further than one square from the controlled character is
not a `WorldTargets` target at all, so it is counted in the status line rather
than drawn as something that would resolve to the square underneath. And
**colour carries no authority**: hues are spread across the terrain ids present
in the current frame so neighbouring classes stay tellable apart, which means a
class can change colour between frames. Identity comes from the sidecar, never
from a pixel's colour.

### `CaptureEmitter`: the presenter obligation, discharged

`client/presentation/capture_emitter.gd` writes one nominated frame as three
files that are meaningless apart:

| File | What it holds |
| --- | --- |
| `capture.png` | the viewport, straight off the texture |
| `capture.identity.pgm` | one 16-bit index per pixel naming which entry of the target list owns it, zero for none |
| `capture.sidecar.json` | frame generation, camera identity, viewport size, the digests of the other two, and the full target list with each target's identity, kind, coordinate, presentation layer, anchor, and hit shape |

The raster is exact **by construction, not by agreement**: it is the same
rectangles the frame was drawn from, filled in the same order, so a marker
overwrites the square beneath it exactly as it did in the picture — and exactly
as pointer resolution reads it. `Image.fill_rect` into a `FORMAT_RG8` image does
the filling in engine code, and that format's bytes already are big-endian 16-bit
samples, which is what the Netpbm `P5` container wants.

**A headless run cannot capture.** Godot's headless display driver produces no
viewport image at all, so the emitter asks `DisplayServer` first and refuses with
that reason rather than writing a blank picture with a confident sidecar. Both
capture routes therefore run under a real or virtual display.

Two harnesses drive it, both under `client/tests/` and both excluded from export:

- `capture_fixture_frame.gd` — the ordinary route. Mounts the view alone,
  replays one recorded authoritative frame, captures. Seconds, no server.
- `live_capture.gd` — the accuracy reference. Mounts the shipped `ClientRoot`,
  signs in, is admitted, captures the frame the real server sent, and records
  that frame so the ordinary route has something real to replay.

`live_session.gd` holds the mount-and-admit sequence both `live_capture.gd` and
`live_server_play.gd` use, so there is one copy of what the shipped scene does.

See [Workbench V0](workbench-v0.md) for what the Workbench does with all this.

## Individual cooldowns made visible

The [September 5 timing ruling](boundary-map.md#21-authoritative-individual-deadlines-d5)
replaces shared rounds with authoritative individual deadlines. Observer contract
8 carries `logical_time`, `ready_at`, spell times, and other deadline values as
canonical decimal **milliseconds**. The former contract 7 is refused.

`client/presentation/action_cooldown.gd` derives the observer's remaining duration
from the frame. Its first frame supplies enough information to animate a bar;
there is no cadence measurement. Updates for the same deadline preserve progress,
duplicate frames cannot extend it, and an idle character's bar stays ready.
Only `can_act` grants readiness. A full local bar with no confirming frame still
leaves actions locked. `CooldownMeter` labels the same state in text and shape.
The world view and feedback director consume that presentation state; neither
owns gameplay time. The retained diagnostic grid still steps terrain discretely
while interpolating eligible markers.

### The web feel scene's walk experiment

The browser feel scene's local experiment lets one click draft a direct route of
one to three squares; an impassable step or farther target clears the draft.
A second click or double click commits a fresh three-second interval. Further
clicks, double clicks, Escape, and right-click cannot replace or cancel a committed
move. Escape and right-click clear drafts. The local square changes when the
complete interval ends; the next move receives another complete interval.
Packet-layout passability and timing remain disposable local stand-ins,
with pure claims in `web/tests/` and the real-tab proof in
`web/proof/walk-proof.mjs`. The proof reads its packet from `TME_FEEL_ASSETS`
and refuses with exit 3 without it — a tracked path is never a default — and
writes its captures to `TME_CAPTURE_OUTPUT` or a named temporary directory;
`web/proof/capture-packet.mjs` photographs any packet under any query string
for owner comparison, and both share `web/proof/serve.mjs`, which serves the
scene on a free loopback port and stops the server's whole process group.
Both run in **two engines**: `serve.mjs` resolves Chromium and Firefox from
Playwright, the walk proof runs itself once per engine in a child process
with its own server, tab, and capture directory (`<root>/chromium/`,
`<root>/firefox/`), and the capture tool shoots every query in every engine
as `<query>-<engine>.png`. Chromium renders headless; **headless Firefox has
no WebGL at all** (probed 2026-09-03 — no preference enables it), so Firefox
runs headed with software GL on a display: `DISPLAY` when the environment
has one, otherwise an Xvfb the launcher starts and stops itself. An engine
Playwright has not installed, or a Firefox with no display and no Xvfb,
refuses the run with exit 3; `TME_PROOF_BROWSER=chromium|firefox` narrows a
run to one engine for a quick look and is never how a proof is claimed.
**Movement presentation (owner direction, 2026-09-05).** The figure follows the
committed route over its full interval, with no return-to-start replacement path.
Run and sprint still identify route lengths; both use the candidate's jog clip.
The presenter exposes the committed and landing timestamps and records presented
positions in its bounded trace. The real-tab proof checks the exact three-second
interval, refusal of competing input, forward travel, final position, facing,
and portal landing in Chromium and Firefox.
Outdoors the camera stays centred on the caretaker and re-centres on each
landing; inside a building it belongs to the space — centred on the room's
grid and unmoved by landings — and follows the caretaker again on the way out
(owner ruling, 2026-09-02; `cameraFocusFor` in `web/src/camera.ts`, proven by
the walk proof's portal crossing). Routes
of one, two, or three squares are labelled walk, run, or sprint.
Its cursor is a plain pale arrow when ready, gains an hourglass while a route is
committed, and gains a diagonal refusal bar over an unauthorable square; the
scene has no beat meter.
A `zoom` query value of a whole number from −3 to 3 scales the frame's world
height by a quarter per step, negative outward, so two tabs can show the ruled
frame beside a candidate; it is a comparison aid for the owner's viewport
ruling, the ruled frame remains the default, and the label names any step in
force.
Every packet prop placement carries a finite `elevation` from zero through six
world units and a `card_height`: the world height the card's **image** spans,
feet at the elevation, top at elevation plus card height — not the subject's
own height. A low, long thing rendered at the ruled angle is mostly its depth,
and sizing it by its own height drew beds and tables doll-sized beside the
caretaker; the render harness knows the projected height and the placement
now carries it (owner ruling, 2026-09-03; the `nominal_height` key is retired and
refused; introduced in schema 3 and retained in current schema 6). The card centre is
the anchor plus half the card height for view-facing and wall-plane cards; a
`floor` facing lays the card flat on its cell just above the ground, its up
toward north, for things that are genuinely flat — a rug — never for anything
with a side; the client cannot judge flatness from a picture, so the prop's
asset row declares it (`flat: true`) and a floor placement of a card not so
declared is refused. A placement without the elevation key is refused rather than
treated as floor-standing.
**The caretaker is a live figure (owner ruling, 2026-09-03; schema 4).** The
packet carries a `figures` group beside the sheets: each figure is a rigged
glTF, the files its glTF names (buffers and textures, flat, by basename), a
clip library, skinned outfit parts on the same skeleton with their own named
files, the figure's treated-card palette inline, a rim darkening, and the
name of its idle clip; the top level says which figure the start places
(`caretaker: {figure}`), and the client carries no caretaker of its own —
`caretaker` left the required prop sheets. Every file is digest-bound like a
sheet. `web/src/space/figureRig.ts` decodes a figure once per packet
**from verified bytes only**: the glTF loader's URL modifier resolves every
name a glTF asks for against blob URLs of the verified payload and refuses an
unlisted name, so no unverified fetch can occur; a figure whose clip library
lacks its idle clip is refused at decode. Each space instances the figure
with a cloned skeleton, clones and patches every standard material so its lit
colour snaps, in gamma space, to the nearest palette colour after a rim from
the view-space normal (the patch refuses to build if three's anchor moves, as
the card wrap does), plays the idle clip on every part through its own mixer,
casts and receives the scene's shadows, and drops the contact-shadow blob the
card had. Facing follows each presented route segment in eight ground directions,
including the last segment when a slow frame skips the landing. While idle, the
figure looks toward the pointer's ground cell; a committed route owns facing
until its landing. The rig's forward axis is +Z, rotated about world up, and
portal transitions carry the heading. The stage exposes `data-caretaker-figure`,
`data-caretaker-clip`, `data-caretaker-facing`, and `data-caretaker-yaw`; the walk
proof checks all eight pointer headings and the final heading after movement
in both engines. These corrections follow the owner's September 4 request to
fit the figure to the scene and face its movement/look direction.

Movement categories select authored clip names; they need not select different
clips. The September 4 candidate binds both run and sprint to the restrained jog
following the owner's concern about the exaggerated three-square sprint.
Changing between categories that share a clip preserves its stride phase.
The candidate's figure and every outfit part are uniformly reduced to 80% in
the external asset packet; the renderer imposes no fixed actor height. The
caretaker's accepted look and further animation classes remain open.

**Static structures (schema 6, September 4 scene-fit pass).** Every space now
carries a `structures` list. Each placement names a digest-bound, self-contained
static GLB, a ground anchor, a quarter-turn yaw, and a bounded inclusive cell
footprint. The verified bytes must embed their buffers and images and carry no
skins or animations. Structures decode once per packet; space instances share
their geometry and materials, with disposal after the packet scene stops.
The footprint blocks the local walk stand-in just as a roof footprint does;
visible eaves do not define occupancy. Schemas 1–5 are refused by name, and a
space without `structures` is refused. Geometry and all external candidate
payloads remain outside the checkout. This supports the assembled cottage in
the existing outbuilding site; it does not add an interior or a portal to it.

The same pass corrects ground and wind-card shader output to the renderer's
sRGB output space and balances outdoor ambient, key, and lantern intensities
in `web/src/space/palette.ts`. Interior lighting retains its separate palette.
The pure proofs cover embedded structure refusal, occupancy and corner blocking,
eight headings, bent routes, skipped landings, and shared-clip stride continuity.
These are candidate implementation changes; visual acceptance remains the owner's.

Light touches a card's shape through two things that ship together
(`web/src/space/cardLighting.ts`). A prop asset row may carry a **normal
sheet** beside its colour sheet — `normal: {file, sha256}`, verified like any
asset, decoded as data rather than colour, and required to match the colour
sheet's pixel size — whose vectors are world-aligned in the card's own frame
(red right, green up, blue toward the viewer; a surface facing straight up is
`(0, 1, 0)`), so a mirrored placement mirrors its normals through the card's
tangent and the sheet is never flipped by hand. Only a prop row may carry one;
a row without one is a flat card. And every card's direct light, key and
practical alike, uses one **wrapped diffuse**, `(N·L + w) / (1 + w)` with
`w = 0.5` (a width of 1 silvered the iron fence under the night key in the
2026-09-03 capture pair), patched into three's physical material at its own Lambert line and
written into the wind card shader directly, so a card lit from the side or
behind keeps a readable shadow side instead of going black. The patch refuses
to build if a three upgrade moves that line. The kit cards' sheets come from a
normal pass of the same render that produced their colour. The current
caretaker uses the live rig's geometry normals; its earlier card used a
silhouette-derived normal sheet.
Every sheet is decoded **straight, never premultiplied**
(`createImageBitmap` with `premultiplyAlpha: "none"`, `web/src/space/textures.ts`).
The browser's default round trip — premultiply on decode, un-premultiply on
upload — zeroes the colour of every fully transparent pixel, which a colour
sheet survives and a normal sheet does not: its flat surround turns black,
filtering pulls that black into the silhouette ring as a backward normal, and
the specular term lights it as a white outline around every card at every
edge. Found on the served caretaker on 2026-09-03; the sheets' transparent
surround is itself filed as #29, because the durable contract is a normal
sheet with nothing to damage.
Packet-derived passability now reserves every wall tile from occupancy while
leaving its door tile crossable.
Fixtures build wall-attached structure from batched material geometry,
block their tile, and own their practical light; only the fire inside the hearth
remains a view-facing card.
Outdoor tree and grass cards share one world-position-phased wind field; decoded
art supplies the canopy weight that lets leaves stir apart from rooted trunks,
and deterministic grass clumps remain one non-blocking, non-shadowing instanced
draw that dedicated interiors omit.
Walls that would cover the caretaker's tile fade without moving the camera or
changing their accepted height; the selection rule lives in
`web/src/walk/wallOcclusion.ts`.
The packet makes the scene a set of disposable spaces joined by door
portals: landing on the last door square swaps to the target space and tile on
the same completion, while rebuilding presentation-only passability, hover,
occlusion, and camera focus. Closed exterior footprints keep pitched roofs as
always-on dressing and block every covered tile except a portal; dedicated
interiors carry no roofs or weather, render their camera-near walls only through
the sill, and use their own props and practical lights.

### Spell preparation and feedback

The preparation band distinguishes server-reported spell preparation from a local
command awaiting a result. Spell readiness uses its own deadline in milliseconds.
Feedback presentation uses the current action's local presentation deadline where
one exists; the profile's presentation windows remain the fallback.

### `CanonicalDecimal`: one owner for the wire's wide numbers

Logical milliseconds cross the wire as canonical decimal strings and never pass through
a float, because a `double` stops being able to tell integers apart at 2^53 and
the shared fixture corpus carries exactly those boundary values. Presenting remaining time
means differencing two of them, and three copies of that comparison logic had
grown up independently — one in the adapter, one in the HUD, one in the
interaction director. `client/adapter/canonical_decimal.gd` is now the one owner
of comparison, bounded difference, and increment, and all three call sites read
it.

### Proving it

`client/tests/test_action_cooldown.gd` covers independent offsets, complete
durations, duplicate frames, wide timestamps, and refusal to infer readiness.
`tools/run_client_live_proof.py` exercises accepted actions at different offsets
against the real server. The capture driver photographs one action's progress.
The [cutover plan](plans/2026-09-05-individual-action-cooldowns.md) records which
proof has actually completed during this slice.

## Proof surfaces

**The retained Godot suite.** The suite runs headless:

```bash
cd client && <godot> --headless --path . -s res://tests/run_all.gd
```

`--suite=NAME` selects one suite. The world shell's tests are three suites, not
one: `input_bindings` (what a key is bound to and the accessibility floor),
`pointer_movement` (drafting a move and what authority does to the draft), and
`world_shell_actions` (ground, inspection, exact server-offered actions, unsafe
confirmation, the domain surfaces). They share `client/tests/shell_test_support.gd`,
which holds every frame builder and screen helper as a static function and
asserts nothing — a helper that asserted would put a failure in a file whose name
says nothing about what failed.

`run_all.gd` preloads every suite, so one
unparseable test file breaks the harness rather than one suite — and
`test_support.test_all_client_scripts_parse` additionally loads every `.gd` file
in the tree, so a script nothing preloads still has to parse. A new `class_name`
needs Godot's class cache refreshed before the suite can see it:

```bash
cd client && <godot> --headless --path . --import
```

**The live proof.** `tools/run_client_live_proof.py` is the phase-6 stop point
made runnable. From an empty database it creates a scratch database, migrates
it, enrols one account, writes a bootstrap manifest naming that account, starts
the real server, fronts it with a loopback TLS proxy carrying a throwaway
authority, and drives the shipped `ClientRoot.tscn` through sign-in, character
selection, admission, an authoritative update, a command round trip, and
sign-out:

```bash
tools/run_client_live_proof.py \
  --admin-url-file <postgres superuser url file> \
  --godot <pinned godot binary>
```

Everything it needs lives in this repository except the superuser URL, which is
used only to create and drop the scratch database. It is not part of the
standing baseline: it needs PostgreSQL 18, `openssl`, `psql`, and the pinned
Godot binary.

The proxy is deliberately dumb — it copies bytes and adds no headers — so the
`Host` and `Origin` the client sent are exactly what the server validates. That
matches the server's proxy-header rule, which accepts all three `X-Forwarded-*`
headers or none.

The provisioning behind it lives in `tools/live_server_harness.py`, which the
live proof shares with `tools/run_fixture_land_capture.py`. The two differ only
in which world they serve and which client script they drive; one copy of the
provisioning is why they cannot drift apart.

**The capture proof.** `tools/run_fixture_land_capture.py` serves the compiled
authoring fixture — the same land the Workbench's logical projection comes from —
and photographs the frame the real server sends, under a virtual display:

```bash
tools/run_fixture_land_capture.py \
  --admin-url-file <postgres superuser url file> \
  --godot <pinned godot binary> \
  --output <directory> [--record-frame tests/fixtures/capture/fixture_land_frame.json]
```

It also records that frame, which is how the ordinary capture route gets a real
server frame to replay. Re-record both together; see
`tests/fixtures/capture/provenance.md`.

**The cooldown capture.** `tools/run_pulse_capture.py` retains its historical
command name and photographs three points within one accepted action's cooldown
against the real server and the `first_land_structure` corpus fixture.
Its samples must share one `ready_at`, progress by at least 0.30 in total,
and agree with the frame's readiness. It checks the action's full three-second
interval independently of the implementation constant. Separate offset actions
are proved by `tools/run_client_live_proof.py`.

```bash
tools/run_pulse_capture.py \
  --admin-url-file <postgres superuser url file> \
  --godot <pinned godot binary> \
  --output <directory>
```

## What these phases did not do

These are limitations of the retained Godot world-view implementation, not a
claim that the separate browser experiment contains no rendering or candidate art.

- **No renderer, still.** The presentation scaffold retired with its assets.
  `GridWorldView` is a diagnostic lattice with real targeting; it is not the
  game's look and makes no claim to be.
- **No art, no assets, no atmosphere.** Flat colours and rectangles.
- **No in-session character switch.** Character selection enters the world; the
  route back is sign-out. This matches the predecessor's behaviour, which also
  required sign-out to reach character selection.
- **No capture of anywhere but the observed frame.** A capture shows the squares
  the client was sent, bounded by the observation radius — not the whole land.
