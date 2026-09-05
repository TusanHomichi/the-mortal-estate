---
last_updated: 2026-09-05
revision: 3
status: Local feel preview plus authoritative diagnostic browser capture; production gameplay integration and visual acceptance remain open.
public_safe: true
summary: Local feel implementation, authoritative diagnostic capture, source map, operation, and two-engine proof.
routes:
  - web/**
---

# Browser client

The browser is the active feel surface. It loads an external candidate packet
into Three.js and runs local movement experiments. The feel scene does not authenticate or implement production gameplay legality.
A separate read-only diagnostic observer consumes the authoritative wire for
Workbench capture; it does not change the candidate preview.
[Client architecture](client-architecture.md) owns the client contract;
[presentation direction](presentation-direction.md) owns the visual target;
[Server notes](server-notes.md) owns server implementation and the direct wire proof. Use the [checkpoint](plans/genesis-ledger.md#current-checkpoint-2026-09-05)
for deployment and work status.

## Source map

| Work | Start here | Proof |
| --- | --- | --- |
| Authoritative capture | `web/src/authoritative/` | shared wire corpus, state/target tests, `tools/run_browser_capture_proof.py` |
| Startup and packet admission | `web/src/main.ts`, `feelScene.ts`, `manifest.ts` | packet and manifest tests |
| Movement, pointing, cooldown | `web/src/walk/` | route, intent, cursor, pointer, facing tests; `web/proof/walk-proof.mjs` |
| Scene assembly and lighting | `web/src/space/SpaceScene.ts`, `palette.ts`, `cardLighting.ts` | geometry, palette, lighting tests and real-tab captures |
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
replay in Chromium and Firefox. It checks raster samples against raycasts,
drives actual pointer events, and compares image/identity/recording bytes plus
sidecar frame facts across replay. The proof page uses the scratch server's
origin and native WSS. Certificate errors are allowed only in its disposable
scratch profile; production certificate verification remains unproven in the
browser. The Python control adapter verifies the scratch CA normally.

The [Workbench capture operation](workbench-v0.md#the-capture-path) owns
configuration, source binding, and atomic publication. Run the complete proof:

```bash
python3 tools/run_browser_capture_proof.py --admin-url-file <file> --output <external-directory>
```

This implements authoritative diagnostic capture, not production login,
command reconciliation, candidate artwork integration, or a preview deployment.

## Movement and availability

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

`manifest.ts` validate current schema 6. Schemas 1–5 and absent
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

### Figures and structures

`figureRig.ts` decodes the rig, clip library, outfit parts, buffers, and textures
from verified bytes. The glTF URL modifier resolves only listed basenames to
verified blob URLs; unlisted requests fail. Missing idle or movement clips fail.
Instances clone skeletons and materials, apply the packet palette and rim,
play clips through a mixer, and cast/receive shadows. The candidate asset packet
reduces the figure and outfit to 80%; the renderer imposes no fixed actor height.
Accepted appearance and further animation classes remain open.

Every space carries `structures`. Each placement names a digest-bound static
GLB, anchor, quarter-turn yaw, and bounded inclusive footprint. GLBs must embed
buffers/images and contain no skins or animations. Decoded geometry/materials
are shared by space instances and disposed when the packet scene stops. Local
occupancy uses the footprint, not visible eaves. An outdoor building model does
not imply an interior or portal.

Spaces connect through explicit door portals. Landing on a portal swaps space
and tile at the same completion, rebuilding local passability, hover, occlusion,
and focus. Closed exterior footprints block covered tiles except portals; roofs
remain visible dressing. Interiors omit roofs/weather, shorten camera-near walls
to their sills, and use their own props/lights. `wallOcclusion.ts` selects walls
to fade over the presented figure without changing camera position or wall height.

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
display shapes, proportional production chrome, and authoritative browser
integration remain unfinished. There is no separate engine entry point.

The [web lane](verification.md#browser-evidence) proves locked dependency install,
typecheck, synthetic unit tests, and build without candidate assets or a GPU.
Optional real-tab evidence requires a packet, installed Playwright Chromium and
Firefox, and ffmpeg for the walk sequence:

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

`browser.mjs` owns browser and temporary display lifetimes.
`TME_PROOF_RENDERER=auto|hardware|software` defaults to auto. Linux auto selects
hardware when a DRM render node is readable/writable; Firefox additionally needs
a display or Weston. Otherwise auto selects software. Chromium runs headless
with ANGLE/EGL for Linux hardware or SwiftShader for software. The tested Firefox
build needs headed WebGL: hardware uses an existing display or a temporary
Weston headless GL compositor; software uses an X display or temporary Xvfb.
No persistent desktop service is created.

Every launch logs its observed WebGL2 renderer. Unknown/sanitized or software
identity cannot pass a requested hardware proof. Only ephemeral Firefox profiles
disable renderer-name sanitization. Missing engines, displays, packets, or
requested renderers return UNAVAILABLE (exit 3), with no silent substitution.
Browser/display processes and temporary sockets are cleaned up on success and
failure. `TME_PROOF_BROWSER=chromium|firefox` narrows a quick inspection; a complete
browser proof uses both. Machine setup and capture receipts stay in local evidence.
Passing tests or captures cannot accept artwork or close an owner gate.
