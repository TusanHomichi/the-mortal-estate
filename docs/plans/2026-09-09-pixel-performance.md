---
last_updated: 2026-09-09
revision: 3
status: Sustained crowd baseline retained; its rendering findings are addressed by the subsequent GPU composition slice.
public_safe: true
summary: Controlled pixel-renderer profiling, composition repairs and follow-up sustained crowd measurements.
---

# Pixel rendering performance

This Planning and Execution record owns the September 9 performance slice.
The subsequent [GPU composition slice](2026-09-09-pixel-gpu-composition.md) owns
the owner-authorized follow-up and its current verification state.
The owner requested improving performance before adding further visual features.
[Browser client](../browser-client.md#pixel-rendering-performance) owns the
implemented rendering contract; [production](../pixel-art-production.md) retains
art preparation and [presentation](../presentation-direction.md) the visual target.
No artwork, geography, gameplay timing, runtime dependency or hosted deployment
change is part of this slice.

Work started on main at `10a8800869a03ede0441b34a6e054d2fb499cbdb`, preserving the
existing dirty checkout. The external `pixel-performance-20260909-r1` packet
retains before-source snapshots, baseline bundles, measurements and captures.
The existing artwork packet remains pinned to manifest SHA-256
`4bbadd5a0fa1b2f900b6eaf3001f58bd21b85a878ec0fae458480d3a89634928`.

## Baseline and diagnosis

The preceding [integration measurements](2026-09-08-pixel-exterior.md#verification-and-remaining-work)
flagged slow Firefox and WebKit rendering. The new native profile uses the actual
product bundle and disposable PostgreSQL-backed authority, with 2.5-second
warmups and eight-second stationary samples. Screenshots, resizing and pixel reads
occur outside measurement windows. Timed renderer callbacks and canvas/WebGL
operations identify CPU submission costs; submission cadence is not monitor
scanout or a physical Steam Deck measurement.

At 1280-by-800 and 1920-by-1080, baseline Chromium submitted about 22 frames per
second despite drawing in under 10ms. Firefox submitted about 20/18 in the temple
and 14/8.5 outdoors. WebKit submitted about 17/9.5 in the temple and 12/6 outdoors.
Firefox's outdoor composition averaged about 63/105ms; WebKit additionally spent
about 20/32ms copying the finished WebGL frame into a visible 2D canvas.

The limiter restarted its interval at each actual callback. Slightly late or
rounded 60Hz callbacks therefore stretched a nominal 30Hz period to three refresh
intervals. Each frame also rebuilt foreground and actor lighting masks, resized
scratch canvases, uploaded unchanged maps and the unchanged colour LUT, and
copied the finished GPU image back to Canvas 2D. The artwork's pixel count alone
was not the diagnosis.

## Repair and proof

Render deadlines now preserve their phase and skip missed deadlines without
catch-up bursts. Unit proof exercises rounded 60Hz timestamps and long stalls.
Scene composition and its GPU textures are retained until a snapshot, input,
resize or moving sprite changes them. Shader time continues advancing for fog,
wind, rain and flickering lights. Foreground material/normal masks are cached at
native scenery resolution, keyed by source rectangle and alpha identity. Actor
maps retain their finer raster and bounded elevation variants. Scene/scale
changes invalidate their caches. The LUT uploads only when its selected image
changes. WebGL presents directly in the visible canvas; ordinary map views use
the same pass with effects disabled. Context loss clears presented actors and
pointing, stops rendering and displays a reload message.

The full town foreground map pair set requires about 6.6 MiB at source resolution,
versus about 59 MiB if cached at 3x. This is the map-array payload calculation,
not total browser or GPU resident memory.

A controlled comparison compiles the retained before-effect implementation beside
the current one and supplies identical room/town imagery, actors, foreground
layers, time, 2x/3x sampling and repeated camera offsets. All 48 cases are
pixel-identical in Chromium, Firefox and WebKit; reusing uploaded textures also
has zero changed channels. An initial independent preparation of the actor normal
mask changed a few Firefox channel values by one level. Reproducing the original
source-in operation order repaired that cause without relaxing equality.
The shared three-engine effect proof also passed all weather profiles, foliage
mask isolation, integer pixels, height-aware fog and front/back light response.

The native resident proof exposed a timing assumption under changed cadence: a
fixed 1.4-second delay did not guarantee a patrolling resident had reached its
latest observed endpoint. The proof now clicks the renderer's current presented
body bounds, including interpolation and shared-square separation. It still uses
normal right-click input and requires the matching resident dialog; no service or
command is synthesized.

## Verification and remaining limits

The selected repository run passed all Python, boundary, web and documentation
steps in 75.595 seconds, including 537 browser tests, TypeScript checking and the
production build. Fresh temple proof passed in all three engines. The three
exterior walks each accepted all 72 commands, visited all seven service buildings,
proved tree occlusion and reconnect, and retained zero scenery-pixel mismatches
at 2x and 3x. The combined external `walk-final.json` points to each engine's
actual report and retains the original resident-pointing failure and repair.
WebKit temple and exterior captures were opened for inspection.

Final isolated performance samples passed in all three engines, using the local
Intel UHD 630 hardware renderer. Intermediate optimization runs overlapped
functional activity and are exploratory only; the final profile ran with no other
task-owned browser proofs or builds active. Its motion windows issued real
movement commands and reported composition-frame cost separately from reused
frames. The profile launcher refuses an exterior-only starting position
because this proof visits both scenes from the temple; the conflicting flags
were observed to exit with an argument error before accessing supplied paths.

Stationary submission rates and 1080p renderer callback p95, before to after:

| Engine / scene | 1280 x 800 frames/s | 1920 x 1080 frames/s | 1080p draw p95 ms |
| --- | --- | --- | --- |
| Chromium / temple | 22.2 → 30.0 | 22.7 → 30.0 | 8.4 → 0.4 |
| Chromium / town | 22.3 → 30.0 | 22.0 → 30.0 | 6.9 → 0.4 |
| Firefox / temple | 20.0 → 30.1 | 18.4 → 29.8 | 4 → 1 |
| Firefox / town | 14.1 → 30.0 | 8.5 → 30.0 | 124 → 1 |
| WebKit / temple | 16.9 → 28.9 | 9.5 → 29.3 | 102 → 1 |
| WebKit / town | 12.4 → 29.9 | 6.2 → 29.3 | 159 → 1 |

Eight-command movement windows at 1920 x 1080, after the repair only:

| Engine | Temple frames/s | Town frames/s | Temple / town composition-frame p95 ms |
| --- | --- | --- | --- |
| Chromium | 30.0 | 30.0 | 3.2 / 4.4 |
| Firefox | 30.0 | 29.5 | 2 / 4 |
| WebKit | 22.6 | 22.5 | 73 / 78 |

These motion windows include normal server readiness waits as well as sprite
travel. Composition-frame costs isolate frames that actually rebuild the scene;
the idle cache is not evidence that moving frames meet the same budget. No
controlled before-motion sample was collected, so the table makes no claim of a
measured movement speedup. All three engines also refused pointing after induced
WebGL context loss and displayed the reload message.

**Remaining finding — WebKit moving-scene composition.** The browser rendering
owner retains this limitation: changing 1080p scenes still exceed the 33.3ms
render budget. In the measured temple/town movement windows, Canvas composition
averaged 5.3/6.6ms and texture uploads 5.2/4.3ms across all submitted frames,
including reused ones; changed frames were much more expensive. A subsequent
renderer slice should investigate composition and texture-transfer costs, and
prove any repair with the same real-command profile, exact-pixel comparison and
native gameplay walks. Reducing character detail or disabling atmosphere is not
an accepted repair. The current bounded improvement does not close this finding.

Physical Steam Deck, packaged Tauri, large crowds and longer sessions retain
separate proof requirements. No new atmosphere features or artwork were added.
No Git lifecycle, persistent-world migration or hosted activation was performed.

## Sustained crowd follow-up

The owner raised the cost of ten visible people before adding further features.
This follow-up measures one and ten continuously moving full-detail figures with
the actual renderer at both target viewport sizes, in the temple and town.
The working packet maps full artwork only to the observer, Tomas and Maude;
other actors currently use placeholder marks. A live test with ten arbitrary
actor identities would therefore understate the intended sprite workload.
Completing multiplayer appearance assignment remains with browser presentation;
this benchmark does not invent gameplay class or identity mappings.

`pixel-crowd-scene.mjs` is an explicit synthetic presentation fixture. It verifies
the current packet normally, then assigns copies of its three figure designs to
ten fixture identities in memory. The renderer is compiled into a separate
benchmark bundle. Fixture inputs use the carried geography and alternate along
passable squares; no server, game entry point, asset receipt or world seed is
changed. Renderer callback instrumentation and statistics have one shared owner,
`pixel-profile.mjs`, also used by the existing native performance proof.

The fixture alternates movement inputs every 400ms, before the renderer's normal
travel interpolation finishes. After a 4.5-second warmup, each eight-second sample
requires all requested figures to have detailed artwork, remain on screen and
move, with more than 90 percent of submitted frames rebuilding composition.
An initial two-second update interval was correctly rejected as idle by the
moving-figure assertion. Captures and measurements stay in the external
`pixel-crowd-20260909-r1` packet. The complete `crowd-matrix.json` identifies all
three single-engine reports and their actual source paths. All 24 measured cases
rebuilt composition on every submitted frame; the ten-figure WebKit temple
capture was opened to inspect the workload. Runtime renderer files retain the
hashes from the preceding verified slice.

This proof measures rendering cost, not ten real network players or server load.
The figures share three designs and their raster caches; the existing shader
still selects at most four shadow casters. Broader appearance diversity and more
complete shadow coverage require their own workload evidence.
The camera follows the moving observer in both crowd sizes. A collected-workload
PASS is not a claim that the renderer met its 30-frame target. The earlier native
movement averages include readiness waits; they must not be quoted as sustained
movement throughput. The composition-frame costs and this continuous fixture
expose that distinction directly.

Measured frame submissions per second with one to ten continuously moving
figures; these are local hardware results, not physical Steam Deck measurements:

| Engine / scene | 1280 x 800, one → ten | 1920 x 1080, one → ten | Ten-figure 1080p draw p95 ms |
| --- | --- | --- | --- |
| Chromium / temple | 30.0 → 30.0 | 30.0 → 30.0 | 8.7 |
| Chromium / town | 30.0 → 30.0 | 30.0 → 30.0 | 7.5 |
| Firefox / temple | 29.7 → 26.3 | 19.6 → 19.1 | 75 |
| Firefox / town | 30.1 → 28.4 | 20.5 → 19.4 | 67 |
| WebKit / temple | 18.9 → 17.6 | 10.5 → 10.1 | 78 |
| WebKit / town | 19.4 → 18.6 | 9.8 → 10.7 | 67 |

Short samples contain scheduling variation; WebKit's slightly higher ten-figure
town result does not establish that more figures are faster. The large fixed
cost is present with a single moving figure. WebKit's one/ten temple composition
averaged 30.2/33.1ms per frame and texture uploads 29.9/25.4ms. Outdoors these were
37.5/39.4ms and 28.4/21.4ms. Every changed frame uploaded all three full-size
colour/material/normal textures. Additional actor-mask preparation also causes
bounded-cache churn, but it is smaller than the full-scene work in this profile.

**Follow-up finding — sustained composition, now also Firefox.** The browser
rendering owner should address the fixed cost of rebuilding and transferring
scenery and lighting buffers whenever actors or camera move. GPU-resident scene
data and sprite composition are the next architectural candidate to investigate;
this is an inference from the recorded operation costs, not a verified repair.
Any adoption must retain fine character detail, foreground ordering, lighting and
native interaction, and meet the same sustained one/ten benchmark in all engines.
The original readiness-wait averages cannot close this finding.

The subsequent [GPU composition cutover](2026-09-09-pixel-gpu-composition.md)
closes this rendering finding with the same continuous workload, retained
artwork, native interaction proof and direct texture-residency assertions. That
record owns the repair's measurements and remaining capacity limits; the values
above remain the before-change baseline.

A separate external experiment supplied `willReadFrequently: true` to every 2D
canvas without changing runtime source. In WebKit's 1080p ten-figure tests it
fell to 9.3 frames/s indoors and 8.9 outdoors, with draw p95 of 93/115ms. That hint
was not adopted. The preserved experiment is diagnostic evidence, not a product
option or a substitute for the baseline matrix.
