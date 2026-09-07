---
last_updated: 2026-09-07
revision: 24
status: Deployment-bound rendered world, guarded login and authoritative two-click path controls; artwork remains candidate.
public_safe: true
summary: Rendered entry, guarded login, authoritative path planning, resident presentation and browser verification.
routes:
  - web/**
---

# Browser client

The browser is the active feel surface. It loads an external candidate packet
into Three.js and runs local movement experiments. The feel scene does not authenticate or implement production gameplay legality.
A separate read-only diagnostic observer consumes the authoritative wire for
Workbench capture. The private play shell uses that renderer with server-owned
movement and lifecycle control. The first-expedition study presents the authored
world using verified candidate artwork; artwork deployments bind that renderer
into the build. Diagnostic inspection builds remain available for proof worlds.
[Client architecture](client-architecture.md) owns the client contract;
[presentation direction](presentation-direction.md) owns the visual target;
[Server notes](server-notes.md) owns server implementation and the direct wire proof. Use the [checkpoint](plans/genesis-ledger.md#current-checkpoint)
for deployment and work status.

## Source map

| Work | Start here | Proof |
| --- | --- | --- |
| Private authoritative play | `web/src/play/`, `web/play.html` | actual adapter tests, installed `web/proof/play-proof.mjs` |
| Authoritative capture | `web/src/authoritative/` | shared wire corpus, state/target tests, `tools/run_browser_capture_proof.py` |
| Startup and packet admission | `web/src/main.ts`, `feelScene.ts`, `manifest.ts` | packet and manifest tests |
| Movement, pointing, cooldown | `web/src/walk/` | route, intent, cursor, pointer, facing tests; `web/proof/walk-proof.mjs` |
| Scene assembly and lighting | `web/src/space/SpaceScene.ts`, `palette.ts`, `cardLighting.ts` | geometry, palette, lighting tests and real-tab captures |
| Coastal surface and grass | `web/src/terrainSurface.ts`, `web/src/space/ground.ts`, `coastalGround.ts`, `groundCover.ts`, `grassCover.ts` | surface contact/material/cover tests and native walking captures |
| Foreground visibility | `web/src/space/surfaceOcclusion.ts`, `surfaceAlpha.ts` | surface occlusion tests, native layered-opacity and walking captures |
| Live figures and structures | `web/src/space/figureRig.ts`, `structures.ts` | rig, structure, facing and occupancy tests |
| Camera and comparison controls | `web/src/camera.ts`, `presets.ts` | camera and preset tests |
| Browser/display lifecycle | `web/proof/browser.mjs`, `serve.mjs` | launcher tests and observed renderer probes |

Paths in a row share the first path's directory unless written in full.
The standing TypeScript check rejects unused imports, locals, and parameters.

## Authoritative diagnostic capture

`crates/tme-protocol/src/codec.rs` owns decoding and semantic validation.
Its instance-local WebAssembly byte interface is built from carried source by
`web/proof/build-codec.mjs`; the toolchain declares the WebAssembly target.
The browser test command rebuilds it before running the shared corpus. No
ignored binary or copied TypeScript schema supplies protocol authority.

`observer.ts` owns the read-only native WSS connection and strict frame
replacement. It receives a one-use ticket from the local harness, sends the
current hello, accepts a welcome before updates, and rejects regressed or
conflicting state. Counters remain decimal strings. Disconnect clears authority;
replay starts a fresh recording generation. Recording is bounded to 256 accepted
frames and refuses overflow. No cookie or password enters the page.

`targets.ts` maps actual observer rows to diagnostic squares and occupant
markers. `renderer.ts` uses Three.js meshes for color, a GPU identity pass, and
raycast targets. Terrain hues and cell shading are diagnostic; they confer no
art acceptance or gameplay meaning. Capture copies its frame, image, identity
pixels, camera, and target list synchronously before asynchronous hashing.

`web/proof/authoritative-capture.mjs` exercises live welcome/update and exact
replay across the shared engine roster. It checks raster samples against raycasts,
drives actual pointer events, and compares image/identity/recording bytes plus
sidecar frame facts across replay. The proof page uses the scratch server's
origin and native WSS. Certificate errors are allowed only in its disposable
scratch profile; this diagnostic proof makes no browser certificate-verification
claim. The separate installed play proof below verifies its local authority. The Python control adapter verifies the scratch CA normally.

The [Workbench capture operation](workbench-v0.md#the-capture-path) owns
configuration, source binding, and atomic publication. Run the complete proof:

```bash
python3 tools/run_browser_capture_proof.py --admin-url-file <file> --output <external-directory>
```

This implements authoritative diagnostic capture, not production login,
command reconciliation, candidate artwork integration, or a preview deployment.

## Deployed presentation selection

An artwork deployment builds `play` in `first-expedition` mode. Renderer selection
is bound into that browser artifact, so `/`, `/index.html`, reloads and query
changes all retain the rendered world. Missing or mismatched artwork fails
explicitly; it does not select the overhead diagnostic renderer. The default
`inspection` build remains available for synthetic proof worlds and can explicitly
select the expedition study through its inspection query parameter.
The deployment runbook owns selection of the matching artwork packet.
`web/proof/play-entry-proof.mjs` checks all three entry paths in the browser roster
against an installed artifact; its JSON stdin supplies `origin`, `presentation`,
`output` and an optional scratch TLS `authority`.

## Login startup safety

The HTML login controls start disabled, omit form field names, and declare POST.
Document policy blocks native form navigation and suppresses referrers. Only the
fully initialized client enables sign-in, after its submit handler is installed.
`play/boot.ts` reports failed startup with controls still disabled and removes
legacy credential query parameters without consuming them. Credentials travel
only in the control adapter's JSON login request, never in navigation.
`web/proof/login-startup-proof.mjs` verifies native submission refusal without
JavaScript and disabled controls during delayed or failed codec startup in the
browser roster. Its JSON stdin supplies the installed `origin` and receipt `output`.

## Private authoritative play

`web/src/play/control.ts` owns the serialized control lifecycle and all transient
credentials. `main.ts` maps semantic actions and HUD facts onto the selected
renderer. Four cardinal steps and Wait have direct controls;
pointer movement follows the [two-click path contract](#authoritative-play-controls),
while a movement button or key sends a single-step intent.
Only a validated server frame changes position or grants readiness. Recovery
progress is cosmetic; reaching its end cannot enable action input.

`play/creationPanel.ts` presents immutable server creation options, allocation
bounds and a point-budget display before character selection. Rules own the
allocation and loadout. An uncertain creation response keeps the original draft
and request ID in memory for an unchanged retry; editing that pending draft is
refused. A successful response updates the roster without admitting a socket.
The [server contract](server-notes.md#character-creation-admission) owns atomic
creation and replay; the [class contract](class-training-contract.md#character-creation)
owns profile meaning. Sign-out clears the draft and creation options.

`authoritative/gameplay.ts` reads the Rust-validated frame's character, equipment
and assessed action options. `play/gameplayPanel.ts` presents these in the private
diagnostic shell, grouping each teacher, merchant, bank, locker and NPC by its
server identity. It exposes the existing service capabilities and equipment,
loot, combat and traversal options without calculating their legality. Repeated
service actions in the general list are presented at their service once.

The panel retains group controls across snapshot refreshes, preserving native mouse
gestures, keyboard focus and entered quantities. Unchanged summary text nodes also
remain intact: WebKit can otherwise lose a disclosure click across a refresh. `play/gameplayGroup.ts` owns each
retained group and disables revoked actions immediately. Dispatch uses the latest
snapshot generation; removed groups cannot dispatch through detached controls.
`web/proof/gameplay-panel-proof.mjs` verifies a refresh between mouse-down and
mouse-up, revocation, focus retention and disconnect in the shared browser roster.
Run it with `TME_FEEL_ASSETS=<external-packet> node web/proof/gameplay-panel-proof.mjs`.

Dispatch re-resolves the selected action against the current frame generation;
missing, disabled, ambiguous, disconnected or stale choices are refused. Gold
movement, bank withdrawal and training allow an explicit decimal amount without
changing the server-offered target. Wide values stay strings. Reconnect/sign-out
clear the panel together with authority. These diagnostic controls do not accept
or replace the [production chrome target](presentation-direction.md#chrome-and-actions).

`tools/run_browser_services_proof.py --admin-url-file <file> --output <external-directory>`
builds this client and drives the shared browser roster against fresh PostgreSQL
databases using carried service fixtures. Its local nginx front serves the built
bundle and passes native control/WebSocket traffic to the real server with
normal certificate verification. The proof waits for the frame revision named
by each receipt before asserting its result. Fixture prices, characters and
geography prove integration only; they do not establish historical correspondence
or a finished first land.

One immutable command remains pending until its correlated terminal result.
Reconnect clears authority and obtains a fresh bootstrap, ticket and welcome;
an ambiguous command replays only its original bytes. Old-epoch reconciliation
cannot advance the new epoch cursor. Non-consuming rejection preserves the
cursor for a fresh command id. Transport loss attempts one fresh reconnect;
validation failures, server draining and unsuccessful recovery leave an explicit
Reconnect action. Failed logout retains control credentials for another attempt;
successful logout clears them and all presented authority.

Text size and remappable semantic keys live in one versioned preferences record,
with invalid shapes returning to defaults. No authentication enters browser
storage. The shell supports buttons, keyboard, visible focus and enlarged text.
This is a private diagnostic play surface, not accepted candidate artwork.

`npm --prefix web run build` builds both the feel scene and private play bundle;
`build:play` rebuilds the Rust codec and emits only the play application. The
[development runbook](../deploy/development/README.md) owns installation and
access. Run its `browser-proof --output <external-directory>` operation against
the installed release. It drives two tabs in each real browser through login,
selection, offset actions, cooldown reconnect, movement and logout, checks
transient credentials and 200% text, and writes screenshots plus a sanitized
receipt outside the checkout. It never substitutes control responses or sockets.
Linux trust proof requires NSS `certutil` for Firefox/Chromium: Firefox trusts
the CA in a disposable profile; Chromium adds only a uniquely named CA entry to its user NSS database
and removes it on cleanup. Both retain normal hostname/certificate validation.

## Movement and availability

### Authoritative play controls

The playable client uses the same two-click interaction as the earlier scene:
first click lays a footprint draft, a second click on its endpoint confirms,
and a native double-click confirms that endpoint. Escape or right-click clears
an uncommitted draft. Clicking another square replaces it. Keyboard and movement
buttons remain immediate single-step actions and clear any draft.

`play/pathPlan.ts` proposes shortest routes through server-observed passable
tiles. The shared `walk/shortestRoute.ts` supplies geometry-only search and
stable tie breaking; candidate packet passability never enters real play.
Unknown ground and blocked diagonal corners are refused. A portal may end a
route, but the draft cannot continue through it into another space. The wire
request bounds paths to three steps; the Rust codec validates each request.

`play/control.ts::previewPath` sends the existing non-mutating path-preview
request through the real socket. It correlates replies by request, actor,
epoch, origin and path, without changing the command cursor or authority.
`play/pathControls.ts` owns only discardable draft/confirmation state. A normal
second click reassesses the route; a fast double-click waits for its in-flight
assessment. Refused or partial assessment cannot silently confirm a different
endpoint. The eventual sequenced command remains subject to current server
rules, including changed terrain, resource costs and partial outcomes.

Pending commands and server cooldown block competing movement. A local clock
cannot release readiness. Authority loss, observer movement or another action
clears a draft; unrelated resident updates do not. Reconnect discards previews.
Right-click still opens actor-bound services after clearing an uncommitted route.

`play/pathOverlay.ts` reuses the original sole artwork and footprint placement
in both renderers, with a ground outline and the earlier ready/hourglass/refusal
cursors. Rendered ground picking ignores foreground roof/crown elevations.
After an authoritative landing, movement follows the confirmed route's traversed
prefix when it matches, with a short presentation animation. This animation
does not own occupancy or cooldown. Portal transitions discard the old overlay.

`web/proof/path-controls-proof.mjs` takes private JSON on stdin containing
`origin`, `username`, `password`, `character`, `output` and optional `temple`.
It exercises ordinary browser input and real HTTPS/WSS in the shared roster,
checking non-mutating drafts, exact command counts, native double-click,
cancellation, cooldown, reconnect, and optional temple entry/return. Credentials
never enter receipt files. The [repair record](plans/2026-09-07-path-controls.md)
owns deployment evidence and limits.

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

### Renderer capability

`engines.json` is the executable roster; `engines.mjs` resolves it against the
installed Playwright browsers. Python producers read the same file and require
every engine's live/replay results before offering an atomic capture batch.
`browser.mjs` owns launch and temporary display lifetimes.
`TME_PROOF_RENDERER=auto|hardware|software` defaults to auto. Linux hardware needs
a readable/writable DRM node; headed Firefox and WebKit also need a display or
Weston. Chromium uses headless ANGLE/EGL or SwiftShader. Firefox uses a headed
WebGL session on an existing display, temporary Weston, or software Xvfb.
Linux WebKit uses the headed GTK port and a resizable Weston desktop shell.
Its disposable GTK settings fix font DPI at 96 so monitor metadata cannot
silently rescale CSS coordinates. The native probe checks the actual viewport
and device scale before measuring GPU execution; settings are removed on cleanup.

Every launch proves a WebGL2 context and the requested rendering capability.
Unknown or sanitized names cannot establish GPU use. WebKit reports a masked
adapter string, so `webkit-renderer.mjs` checks a rendered pixel and increasing
DRM execution counters in this launch's tagged WebKit content/GPU process. It
excludes the display compositor and unrelated browser launches. Without that
native evidence, masked WebKit hardware/software configurations are unavailable;
software-only and non-Linux WebKit need their own backend evidence before those
configurations can be claimed. Firefox's ephemeral proof profile disables its
renderer-name sanitization. Missing engines, displays or requested capabilities
return UNAVAILABLE (exit 3); there is no silent substitution.

Linux WebKit's private-CA proof uses the system TLS store: noninteractive sudo
installs one uniquely named temporary anchor and removes it on stop or failed
launch. It requires `install`, `rm` and `update-ca-certificates`. Other anchors
are retained, and certificate errors are never ignored in private-play proof.
The diagnostic scratch-WSS exception remains separately scoped above.

`TME_PROOF_BROWSER=chromium|firefox|webkit` narrows an inspection; complete proof
uses the whole roster. Browser/display resources and temporary trust are cleaned
up on success and failure. Machine setup and capture receipts remain external.
Passing tests or captures cannot accept artwork or close an owner gate.

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
colour, crest lighting and bank-intersection foam. It uses one opaque draw with
no scene-copy or reflection-camera pass. Key direction comes from the same
palette helper as the scene light. This is an original small-wave approximation,
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

The [deployed presentation contract](#deployed-presentation-selection) selects
the private visual study for the authored first-expedition world. Inspection
builds can select it with `play.html?study=first-expedition`. `play/studyReceipt.json` owns the current candidate artwork
manifest digest and binds it to the accepted geography review.
`play/studyBinding.ts` verifies that ancestry against the geography receipt,
then verifies the artwork manifest bytes against the study receipt. It separately
checks the server's master identity, realm, level, bounds and passability window
before presenting a frame. Artwork iteration does not re-sign geography, and an
old or mismatched asset packet is refused. The selected guide digest records
visual provenance only; it grants no runtime or master-acceptance authority.
Exterior contexts move with the observer; interiors supply the whole room.
Missing or mismatched study assets refuse presentation and never substitute
private gameplay content or make the transport report a false protocol error.

`play/studyRenderer.ts` reuses `SpaceScene` and the verified original candidate
assets through the existing external asset boundary. It never creates a local
walk presenter. All positions and actions remain server-owned. Short movement
interpolation follows already committed positions; small within-square figure
offsets keep a player and service NPC distinguishable. The candidate residents
are removed from static scenery and instantiated only from observed actor rows.
Ground-item and corpse markers and the shared fallback actor rig remain
provisional presentation. This study grants no artwork-master acceptance.

### Temple resident interaction direction

**Owner request implemented, 2026-09-07.** Players should right-click the
balm seller herself to open balm buying. The person must be visible and
targetable beside her shelving; a decorative counter is not the service target.
Connect that interaction to her observed actor/service identity and the existing
server-projected purchase options. Opening the interface grants no purchase
eligibility, range exception or automatic transaction. The current private
client also retains its existing service action controls.

The [resident contract](town-resident-contract.md) owns Tomas's circuit, attention
pause, service binding and persistence. `studyActors.ts` retains one visual
instance per observed actor, interpolates committed location changes and keeps
idle animation running across unrelated frames. Occluding opaque scenery blocks
picking; faded surfaces allow it. `actorInteraction.ts` builds the resident panel
from observed identity and current projected offers, closes it on authority loss,
and preserves disabled purchases as disabled.

The receipt explicitly maps resident IDs to candidate figure names, with every
model dependency digest-bound. Missing declared figures refuse the study. The
interior camera retains the fixed angle and nine-row world scale. The current
private-play page still uses its older 768 × 512 diagnostic canvas; it does not
implement the [accepted UI aperture](presentation-direction.md#accepted-chrome-layout).
Full chrome integration must use that aperture and the proportional-scaling
ruling before framing acceptance. Native evidence exercises right-click picking, a valid
purchase, insufficient-funds refusal, observed patrol movement, priest service
access, reconnect and the dungeon return. The
[presentation owner](presentation-direction.md#placeholder-town-interiors) owns
the room and residents' visual direction.

### Skill critique feedback

Protocol minor 9 adds typed private skill-critique feedback. The previous minor
is refused by the shared codec and its native/browser corpus. NPC replies and
critique responses are formatted from typed command-receipt cues; no raw event
or debugging fallback reaches player-facing text. Disconnect clears the study's
world and occupants through the same control-state invalidation.

The native expedition proof is `tools/run_first_expedition_proof.py`, with an
external `--admin-url-file`, accepted study `--assets` directory and external
`--output` directory. Its default uses every rostered engine. The authored
`world.json` supplies catalog, template and seed; the proof routes through the
compiled geographic projection, then operates only through ordinary UI controls
and checks real server receipts. It covers creation, preparation, descent,
encounter, loot, return, instruction, critique, lodge storage and reconnect.
It establishes the private integration loop, not final artwork or exact numeric
fidelity. The [execution record](plans/2026-09-06-first-expedition.md) owns results.
