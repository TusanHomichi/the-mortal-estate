---
last_updated: 2026-09-09
revision: 1
status: GPU composition complete; repository, three-engine native and sustained crowd proof passed.
public_safe: true
summary: GPU-resident colour and lighting composition removes sustained full-frame Canvas transfers while retaining pixel artwork and gameplay.
---

# GPU pixel composition

This Planning and Execution record owns the owner-authorized GPU compositor
cutover following the [sustained crowd measurements](2026-09-09-pixel-performance.md#sustained-crowd-follow-up).
[Client architecture](../client-architecture.md) owns the standing boundary and
[browser client](../browser-client.md#pixel-rendering-performance) the implemented
pipeline. Artwork, geography, authoritative movement, resident services and
atmosphere equations retain their existing owners.

The work starts at `10a8800869a03ede0441b34a6e054d2fb499cbdb` on main, preserving
the existing dirty checkout. The external `pixel-gpu-20260909-r1` packet retains
the before-source snapshots, comparisons, captures, benchmark bundles and logs.
The artwork manifest remains
`4bbadd5a0fa1b2f900b6eaf3001f58bd21b85a878ec0fae458480d3a89634928`.

## Scope and implementation

The prior pipeline rebuilt and uploaded three full-screen colour/material/normal
images whenever a figure or camera moved. A single moving figure triggered most
of the cost. The owner authorized retaining those surfaces on the GPU and
composing figures there before adding further visual features.

`pixelCompositor.ts` owns three RGBA render targets in the same WebGL context as
the final effects pass. It retains immutable prepared-image textures, submits
ordered quads into each surface, and samples them directly for atmosphere.
Foreground material and normal samples use the same source rectangle and alpha
as the colour layer. Character height gradients and normal masks are evaluated
by the draw shader, removing fractional-elevation scratch-canvas churn. Camera
movement changes quad coordinates, not image contents. Texture resources are
explicitly invalidated or released on scene/scale changes and disposal.

Canvas 2D remains a preparation tool for the selected fine character rasters,
text, contacts and ground ink. `pixelOverlays.ts` retains ink in authored pixel
coordinates and keys it on observed tile facts and input overlays; camera or
actor-only updates reuse it. Changing preparation invalidates its GPU texture.
Unillustrated map areas use the same GPU output and prepared labels/tile images.
The retired full-screen Canvas colour and lighting composition paths are removed.

The effect equations, four-caster/eight-light selection, 30Hz rendering cadence,
integer scenery scale, fine character raster, picking and server authority are
unchanged. This adds no rendering dependency, runtime flag, wire field or
alternative product entry. No publication, hosted activation or Git lifecycle
is included.

## Proof and findings

The shared profiler now distinguishes framebuffer composition draws from the
final default-framebuffer submission. A frame that composes on the GPU counts
as changed work; it cannot be mistaken for an idle reused frame. Final sustained
proof additionally requires no texture upload or Canvas drawing inside
the warmed movement windows.

The final isolated `crowd-final/verification.json` passed all 24 cases across
Chromium, Firefox and WebKit: one/ten moving figures, temple/town and
1280×800/1920×1080. Every measured frame composed on the GPU; every warmed window
recorded zero texture uploads, texture updates and Canvas image-composition calls.
All cases submitted 29.65–30.09 frames/s against the existing 30Hz render cap.

The following compares ten continuously moving figures at 1920×1080 against the
preceding crowd baseline. Costs are instrumented renderer callback p95, not GPU
completion time. Frames/s are submissions, not display scanout measurements.

| Engine / scene | Before → GPU frames/s | Before → GPU callback p95 ms |
| --- | --- | --- |
| Chromium / temple | 30.0 → 30.0 | 8.7 → 1.5 |
| Chromium / town | 30.0 → 30.0 | 7.5 → 1.2 |
| Firefox / temple | 19.1 → 29.9 | 75 → 2 |
| Firefox / town | 19.4 → 30.0 | 67 → 2 |
| WebKit / temple | 10.1 → 29.7 | 78 → 1 |
| WebKit / town | 10.7 → 29.9 | 67 → 2 |

These runs used hardware-backed Intel UHD Graphics 630/Mesa rendering; WebKit
also recorded DRM GPU execution. The workload retains the preceding benchmark's
three shared figure designs and four-caster shadow cap. It establishes the
rendering improvement for that workload, not ten network players or ten unique
texture sets. The fixed full-scene transfer finding is closed by this cutover;
physical-device and broader-capacity proof remain separate below.

The before/after renderer comparison freezes time and presentation input while
varying scene, sampling scale, figure count and fractional positions. All 72
cases completed across the three engines, including the unillustrated bank.
Chromium scenery-only temple and town cases are pixel-identical. Firefox's
outdoor transparent composition differs by at most two channel levels; WebKit's
scenery-only temple differs by one level at 8/18 pixels for 2x/3x.

With ten figures, the illustrated scenes have mean absolute channel differences
below 0.014 on the 0–255 scale. Chromium's maximum difference is four levels,
WebKit's five. Firefox's largest difference is 20 at contact-shadow antialiasing
edges; a targeted buffer read confirms the material and normal values at those
pixels are identical. Placeholder map labels use transparent cached glyphs and
therefore differ from the former opaque Canvas LCD antialiasing. The comparisons
retain images and error distributions rather than claiming whole-frame bit
equality. Temple, exterior and bank before/after captures were opened for review.

Three-engine shader proof passed weather profiles, foliage isolation, integer
pixels, height fog and directional lighting. Added composition checks prove
orientation, source cropping, transparent overlap, layer order, zero uploads
when reusing images, one upload after explicit invalidation, resized target
sampling and deletion of all three framebuffers.

The Chromium temple walk exposed an immediate-read assumption after a viewport
resize. The proof now waits for a rendered projection at the requested width.
The renderer also invalidates both kinds of pointing while the resized canvas
is blank, restoring them only after drawing. The frozen comparison explicitly
proves this interval refuses pointing, then restores it after a draw.
The exterior proof likewise exposed a projection read during that blank interval.
The shared `pixel-pointing.mjs` now waits for a rendered viewport and reads its
projection, scale and bounds atomically. Action readiness still comes separately
from the real server; the helper never synthesizes a command or frame.

The final three-engine temple walkthrough passed resident menus and purchases,
descent/return and resizing. Each exterior walkthrough accepted all 72 native
commands, visited all seven service buildings and checked tree occlusion, dock
travel, reconnect and missing-art refusal. Both integer-sampling sizes had zero
mismatches. The final Chromium exterior run includes the shared pointing helper;
the earlier Firefox/WebKit exterior passes precede it. The subsequent three-engine
temple and native profile runs cover the current helper and resize behavior.

The final isolated native profile passed all three engines at 1280×800 and
1920×1080, with stationary samples and eight real movement commands per scene.
Submission rates were 29.7–30.1 frames/s. At 1080p, temple/town movement callback
p95 was 0.5/0.9ms in Chromium, 1/2ms in Firefox and 1/1ms in WebKit. Each engine
also passed forced context-loss refusal of pointing and visible error handling.
These gameplay samples include action-readiness waits; the continuous crowd
measurement below owns the sustained-motion conclusion.

The selected meta, web, documentation and boundary verification completed with
all 537 unit tests, typechecking and both browser builds passing. The external
packet retains `verification-final.log`, `walk-final.json`, `effects-closed`,
`comparison.json`, `profile-final` and `crowd-final` with the observed results. The final native
temple and exterior captures were opened for visual inspection. The prior failed
Chromium resize runs remain as evidence of the repaired finding.

The lesson from this slice is to measure continuous changed frames separately
from gameplay waits, then verify residency directly. Low callback averages that
include idle reuse cannot establish moving-scene throughput. Render-target
orientation, alpha ordering and explicit invalidation have independent GPU proof
because visual similarity alone does not establish those contracts.

Physical Steam Deck and packaged Tauri performance, broader figure diversity,
larger crowds and extended sessions remain separate proof requirements. The
existing incomplete assignment of full artwork to other player identities is
not silently resolved by this rendering change.
