---
last_updated: 2026-09-13
revision: 63
status: Shared martial playback uses elapsed pose time, bound contact phases and complete rig visibility; linked execution records own proof and delivery.
public_safe: true
summary: Full-world 3D, elapsed shared rig playback, root-motion binding, contact-phase arrival and native pose proof.
routes:
  - web/**
  - tools/run_world_proof.py
  - tools/run_dungeon_proof.py
---

# Browser client

The browser is the active play surface. One world shell consumes server-owned
movement, actors, services and lifecycle. `worldRenderer.ts` uses one Three.js canvas for
town, temple, service interiors and all four dungeon floors. A separate read-only diagnostic observer supports Workbench
capture; explicit inspection builds support synthetic proof worlds. Older study
renderers are excluded from the product. The
[world execution](plans/2026-09-13-full-world-3d.md) owns the cutover and
verification status; the [3D ruling](presentation-direction.md#3d-reopening)
owns the selected visual direction.
[Client architecture](client-architecture.md) owns the client contract;
[presentation direction](presentation-direction.md) owns the visual target;
[Server notes](server-notes.md) owns server implementation and the direct wire proof. Use the [checkpoint](plans/genesis-ledger.md#current-checkpoint)
for deployment and work status.

## Source map

| Work | Start here | Proof |
| --- | --- | --- |
| Private authoritative play | `web/src/play/`, `web/play.html` | actual adapter tests, installed `web/proof/play-proof.mjs` |
| World scenery and binding | `web/src/play/worldRenderer.ts`, `settlementScenery.ts`, `settlementAssets.ts`, `settlementReceipt.json`, `worldSpace.ts`, `worldCamera.ts`, `worldGrid.ts`, `groundTexture.ts` | `web/tests/worldSpace.test.ts`, `worldScenery.test.ts`, `web/proof/world-room-proof.mjs`, `world-exterior-proof.mjs` |
| Shared foreground visibility | `web/src/play/dungeon/occlusion.ts` | `web/tests/worldScenery.test.ts`, `web/proof/world-occlusion-proof.mjs` |
| Live dungeon rendering | `web/src/play/worldRenderer.ts`, `dungeon/` | `web/tests/dungeonRenderer.test.ts`, `web/proof/dungeon-proof.mjs` |
| Shared character motion | `web/src/play/dungeon/actors.ts`, `assets.ts`, `playback.ts`, `motion.ts` | `web/tests/dungeonActors.test.ts`, `web/tests/dungeonMotion.test.ts`, `web/tests/dungeonJumpkick.test.ts`, `web/tests/dungeonPlayback.test.ts`, `web/proof/dungeon-motion-proof.mjs`, `dungeon-phase-capture.mjs` |
| Authoritative capture | `web/src/authoritative/` | shared wire corpus, state/target tests, `tools/run_browser_capture_proof.py` |
| Retired 3D packet admission | `web/src/main.ts`, `feelScene.ts`, `manifest.ts` | packet and manifest tests |
| Retired local movement | `web/src/walk/` | route, intent, cursor, pointer, facing tests; `web/proof/walk-proof.mjs` |
| Retired scene and lighting | `web/src/space/SpaceScene.ts`, `palette.ts`, `cardLighting.ts` | geometry, palette, lighting tests and real-tab captures |
| Retired coastal rendering | `web/src/terrainSurface.ts`, `web/src/space/ground.ts`, `coastalGround.ts`, `groundCover.ts`, `grassCover.ts` | surface contact/material/cover tests and native walking captures |
| Retired 3D foreground | `web/src/space/surfaceOcclusion.ts`, `surfaceAlpha.ts` | surface occlusion tests, native layered-opacity and walking captures |
| Retired 3D figures | `web/src/space/figureRig.ts`, `structures.ts` | rig, structure, facing and occupancy tests |
| Retired camera comparisons | `web/src/camera.ts`, `presets.ts` | camera and preset tests |
| Browser/display lifecycle | `web/proof/browser.mjs`, `serve.mjs` | launcher tests and observed renderer probes |

Paths in a row share the first path's directory unless written in full.

## Full 3D presentation

`npm --prefix web run build` and `node web/proof/build-play.mjs <external-output>`
create the `world` product. One canvas, camera, figure set and pointer seam serve
all areas. Dungeon scenery draws only the current observed seven-by-seven tiles;
settlement scenery draws authored static rooms or the supplied bounded exterior
context. Dynamic actors and contents come only from current observer rows.
`worldSpace.ts` binds the geographic promotion, area, bounds and exact supplied
walkable mask, including duplicate-row refusal. Artwork never grants an action.

`settlementReceipt.json` pins self-contained original scenery GLBs and the current
geography master. `dungeon/receipt.json` pins the rigged body and motion GLBs.
All files load through the external `/feel-assets/` mount with SHA-256 verification;
external model dependencies and missing required clips refuse startup. Current
compiled structures and their paired entrances own building placement. Original
room assets and rebuilt exteriors remain candidates, with private provenance.
No old image, sprite packet, launcher selector or rendering fallback is retained.

`worldCamera.ts` uses the fixed perspective direction with room for exterior
buildings. The existing dungeon camera retains its selected framing; both follow
the interpolated controlled actor. Resize changes camera projection and rendering
resolution together. `world-pointing.mjs` consumes the actual `worldPoints` and
`worldActorPoints`, never a separate pixel calibration. `worldGrid.ts` draws unique
edges of observed passable tiles independently of floor artwork. The same world
coordinates drive ray-plane pointing, path footprints and living actor placement.

Occlusion samples every joint of observed bodies, including extended limbs,
without assuming a rig's bone-name convention. A transparent depth
pass admits only the nearest faded surface before its colour pass, so overlapping
roof pieces blend once at the directed opacity. Opaque materials and draw order
return when the body clears. Scene changes release cloned fade/depth materials;
shared model geometry remains owned by the loaded packet. Context loss clears
presentation and displays a reload message. No renderer fallback is selected.

The world shell fills the viewport. `worldStyle.css` retains direct movement,
resident menus, session controls and feedback while hiding temporary diagnostic
HUD panels. Double-clicking an occupied stair square selects its sole enabled
server-offered traversal; readiness and ambiguity checks remain in the shell.

For room GLBs with only walls and furnishings, the scenery owner supplies solid
wood or stone floor instances from current authored non-void cells. Those floors stay
outside the occlusion set and are disposed with the area, preserving actual room
coverage without borrowing a pixel layer.

### Operation and native proof

`tools/run_world_proof.py --admin-url-file <file> --assets <external-packet>
--output <external-directory>` builds and serves the world on disposable local
authority, starting the seeded player in the temple. Access details are mode 0600
and removed on shutdown. `--proof` runs the browser roster; `--engine` narrows to
inspection. Use `--release <immutable-release>` instead of `--assets` to verify
release integrity and use its browser and server without rebuilding.
`--exterior` covers all seven town entrances, room returns, resize,
reconnect and the controlled character behind the bank. `--body male` or
`--body female` selects the Martial Artist fixture without changing live saves.
Add `--figures --proof --body <sex>` for its bounded temple/town walking, return
and reconnect proof.
`--entry --proof` covers creation and durable roster behavior.

`tools/run_dungeon_proof.py --release <immutable-release> --admin-url-file <file>
--output <external-directory>` proves the existing four-floor, door, stair,
actor-action, male/female combat and defender-block scenarios against the named
release, plus both controlled bodies at the forge wall. Its occupied-hand control attributes the positive block to the unarmed
case. Martial scenarios cover one-, two- and three-square approaches and capture
actual framebuffer draws at takeoff, contact, landing, recovery and a settled
follow-up punch, with current clip weights and route placement. `world-wall-proof.mjs`
checks the actual player's joint sampling, wall obstruction, reconnect and clearing
without sending gameplay commands. Narrowed scenario
or engine runs are inspections. Release bytes and content
must match their receipts; no binary rebuild or fallback is used.

`node web/proof/world-occlusion-proof.mjs <external-output>` measures overlapping
surface composition and restoration with synthetic geometry in the browser
roster. It does not fabricate a gameplay frame. The web verification lane uses
tracked synthetic data; private asset and native evidence are separate.

The native world proofs collect GPU console failures through
`web/proof/graphics-errors.mjs`; a successful JavaScript session cannot hide a
failed draw. `world-occlusion-proof.mjs` proves repeated cached-shadow draws,
restored opacity and the retired shadow-mode negative control, alongside the
nearest-surface blending check. Settlement scenery owns its supported shadow
mode; the renderer retains the ruled basic dungeon shadows.

## Pixel-art presentation

The pixel product and its performance/effects launchers are retired. The
[pixel transition](plans/2026-09-08-pixel-art-transition.md) and subsequent dated
execution records preserve their historical evidence; the current world cutover
supersedes temporary pixel town/interior retention.

### Dungeon character motion

Every controlled character uses the owner-selected Martial Artist models,
including existing characters of other classes. `sex_or_gender_display` equal
to female selects its female variant; otherwise the provisional male body is
used. This changes presentation only, never class or available actions.
Tomas and the balm seller use their bound original rigged resident models.
Other observed actors retain the generic 3D candidate because their rows carry
no class/body identity. This adds no creation
field. `dungeon/receipt.json` binds both self-contained eleven-clip GLBs alongside
the existing candidate. Missing, stale or unbound required clips refuse loading.

Only new accepted state updates supply motion cues; welcome and command replies
cannot replay attacks. Confirmed unarmed fight outcomes rotate four punches;
jumpkick selects the flying kick. When that accepted update also supplies a complete
movement chain, the kick clip spans its bounded presentation interval. Bound
contact and landing phases identify the source motion: the rendered root follows
the accepted route to the shared target by contact, then stays there through
landing and recovery before returning to guard. Later cover cues cannot overwrite
that closing sequence. Blocked
melee feedback otherwise supplies a cover pose without claiming which defense
absorbed the blow. No-sight, not-ready and
ranged block outcomes supply no combat clip. Reactions expire on elapsed local
time, including hidden-tab time, without granting readiness or creating damage.
`playback.ts` owns pose advancement and crossfades on that same elapsed clock;
render cadence never slows a clip relative to its route or restarts an expired
reaction's fade. Diagnostics include the sampled phase and effective clip weights.

The bound martial motion root is validated on every clip. Playback clones its
tracks and holds root translation on the horizontal plane at the bind origin,
retaining vertical lift and articulated motion. Embedded punch lunges therefore
cannot add a second relocation beyond the presented actor anchor. Source GLBs,
body scale and authoritative occupancy are unchanged.

Walking uses a complete visible local actor-moved chain beginning at the previous
cell and ending at the current one. Gait phase follows rendered distance, using
the body's own walk. Rules projects adjacent self-targeting local-door transitions
as movement; paired doors and other transitions retain snap placement. Remaining
authoritative time bounds visual travel; a ready
frame ends it. The fixed-direction camera follows the rendered observer anchor.
Incomplete chains and area transitions snap to supplied placement.
The [gameplay baseline](gameplay-baseline.md#closing-jumpkick-implementation-specification)
owns closing-kick legality and immediate resolution; the renderer never invents
displacement or delays damage until its clip completes. A missing or partial chain
snaps to authoritative placement and permits only the local combat clip. The
[wall-motion record](plans/2026-09-13-martial-wall-motion.md) owns the current
correction, phase evidence and delivery. The [closing-kick record](plans/2026-09-10-closing-jumpkick.md)
retains its earlier proof and owner visual-acceptance limit. The earlier
[motion record](plans/2026-09-10-martial-motion-integration.md) is evidence for the
prior in-place integration, not proof of the new approach.

The observer can open their own actor menu to use an enabled server-offered stair
action, including a landing directly on stairs. Observed stair markers and door
states are drawn; concealed closed doors remain masonry. Door-state changes
invalidate the overlay cache.
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

The default factory creates `WorldRenderer`. The product build fence refuses
pixel, retired study and diagnostic renderer modules. Root, index, reload and
obsolete query selectors keep the same 3D world. Missing assets fail explicitly.
Only an explicit `inspection` build replaces the factory for synthetic worlds;
retired build modes are refused before output creation.

The deployment runbook owns the external packet. Staging validates and copies
only GLBs pinned by the settlement and figure receipts. `play-entry-proof.mjs`
checks root, index and obsolete selector URLs against the installed artifact in
the browser roster. Its JSON stdin supplies origin, presentation, output and an
optional scratch TLS authority. Retired local tool operation is documented in
[browser reference tools](browser-reference-tools.md).

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

`play/entryShell.ts` presents sign-in, character roster, creation and connection
recovery as full-screen game menus. `entryStyle.css` owns the shared frame,
controls and responsive Deck/desktop layouts. `entryBackdrop.ts` paints the
verified 3D menu scene once. It contains static scenery without a synthetic
player or live world state, scales to cover the entry and adds no animation loop.
Disposal removes its canvas and resize listener.
The production world hides the entry completely after admission. Startup still
keeps login disabled until the codec and renderer are ready.

`play/creationPanel.ts` uses class radios and six plus/minus attribute rows.
`creationAllocation.ts` checks the catalog's bounds and exact spend; rules remain
the final validator. A new class starts at its minimums with the whole pool
available. Reset restores those minimums. The form does not consume or expose
the catalog's authored suggested allocation, following the
[entry presentation ruling](presentation-direction.md#entry-and-character-creation).
Name and a complete allocation are required before Create.
Nationalities are omitted from presentation; profile IDs retain catalog meaning.
`creationTheme.ts` owns original class emblems and short presentation descriptions.

An uncertain creation response exposes a copied pending draft from `control.ts`
and locks edits and Back. Retry submits the same draft and request ID. A successful
response updates and selects the new roster entry without admitting a socket.
Back closes an unsubmitted sheet; sign-out clears transient creation state.
The [server contract](server-notes.md#character-creation-admission) owns atomic
creation and replay; the [class contract](class-training-contract.md#character-creation)
owns profile meaning. The [entry execution record](plans/2026-09-09-game-entry.md)
owns verification and visual evidence.

`authoritative/gameplay.ts` reads the Rust-validated frame's character, equipment
and assessed action options. `play/gameplayPanel.ts` presents these in the private
diagnostic shell, grouping each teacher, merchant, bank, locker and NPC by its
server identity. It exposes the existing service capabilities and equipment,
loot, combat and traversal options without calculating their legality. Repeated
service actions in the general list are presented at their service once.

`play/actorInteraction.ts` likewise retains unchanged actor-menu buttons across
snapshot generations. Its semantic signature includes current offers and merchant
labels; revocation retires the old targets. Dispatch uses the latest generation,
and dismissed or detached controls cannot act. The native panel proof also checks
actor-menu gestures across refresh, revocation and dismissal.

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
`play/pathControls.ts` owns only discardable draft/confirmation state. A command
receipt and unrelated ready frames may precede its position frame. Retain the
committed route until displacement/cooldown has been observed, then authoritative
readiness retires it. Space changes and replacement input also clear it; a ready
user can cancel an unlanded submitted route after refusal. A normal
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
A proposed route may continue through an observed **open** door whose destination
is exactly its own realm, level and cell. A closed local door may be the endpoint;
opening ends the move there. The [gameplay ruling](gameplay-baseline.md#local-door-movement)
owns this behavior. Other transitions terminate proposals; server preview decides outcomes.

`web/proof/path-controls-proof.mjs` takes private JSON on stdin containing
`origin`, `username`, `password`, `character`, `output` and optional `temple`.
It exercises ordinary browser input and real HTTPS/WSS in the shared roster,
checking non-mutating drafts, exact command counts, native double-click,
cancellation, cooldown, reconnect, and optional temple entry/return. Credentials
never enter receipt files. The [repair record](plans/2026-09-07-path-controls.md)
owns deployment evidence and limits.

## Renderer capability

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

### Temple resident interaction direction

**Owner request implemented, 2026-09-07.** Players should right-click the
balm seller herself to open balm buying. The person must be visible and
targetable beside her shelving; a decorative counter is not the service target.
Connect that interaction to her observed actor/service identity and the existing
server-projected purchase options. Opening the interface grants no purchase
eligibility, range exception or automatic transaction. The explicit diagnostic
client retains its generic service action controls. The world view uses the
resident interaction directly.

The [resident contract](town-resident-contract.md) owns Tomas's circuit, attention
pause, service binding and persistence. The shared 3D actor presenter retains observed actor
identity and interpolates accepted movement chains. Rigged body meshes supply
pointing without changing occupancy. `actorInteraction.ts` builds the
actor panel from observed identity and current projected offers, closes it on
authority loss, and preserves disabled actions as disabled. Alongside services,
it shows physical attacks and direct or warmed spells whose typed actor target
matches the clicked identity. Coordinate, area, self-targeted and unrelated
actions are not assigned to a creature by location or display name. Dispatch
retains the original character action group and re-resolves the current offer,
including its target and hostility authorization. Opening the panel sends no
command. The retired gameplay HUD remains absent.

The figure receipt maps supported resident IDs to hash-bound rigged GLBs; missing
declared assets refuse loading. Native proof exercises right-click picking,
a valid purchase, insufficient-funds refusal, observed patrol movement, priest
service access, reconnect and the dungeon return. The
[presentation owner](presentation-direction.md#stylization-and-restrained-charm)
owns the room and residents' visual direction.

### Skill critique feedback

Protocol minor 9 adds typed private skill-critique feedback. The previous minor
is refused by the shared codec and its native/browser corpus. NPC replies and
critique responses are formatted from typed command-receipt cues; no raw event
or debugging fallback reaches player-facing text. Disconnect clears the study's
world and occupants through the same control-state invalidation.

The historical expedition proof is `tools/run_first_expedition_proof.py`, with an
external `--admin-url-file`, accepted study `--assets` directory and external
`--output` directory. Its default uses every rostered engine. The authored
`world.json` supplies catalog, template and seed; the proof routes through the
compiled geographic projection, then operates only through ordinary UI controls
and checks real server receipts. It covers creation, preparation, descent,
encounter, loot, return, instruction, critique, lodge storage and reconnect.
It establishes the private integration loop, not final artwork or exact numeric
fidelity. The [execution record](plans/2026-09-06-first-expedition.md) owns results.
