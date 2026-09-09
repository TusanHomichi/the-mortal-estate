---
last_updated: 2026-09-08
revision: 1
status: Pixel product and minimal playfield verified across three browsers; ready for Codex update and exterior construction.
public_safe: true
summary: Verified pixel-art cutover and HUD removal, production-method documentation, exterior-window repair and update handoff.
---

# Pixel-art transition

This Execution record owns the September 8 implementation and evidence. The
[visual ruling](../presentation-direction.md#pixel-art-decision) owns the selected
direction, [production guide](../pixel-art-production.md) owns the art method,
and [browser client](../browser-client.md) owns current rendering behavior.

## Scope and ancestry

After reviewing the final temple, the owner directed a hard transition to pixel
art, documentation of the successful techniques, and subsequent exterior
construction. The owner then requested a stopping point before updating Codex.
The owner also rejected the provisional gameplay HUD, retaining direct movement
until a real UI is designed. This slice completes that removal, the code/default
cutover and production guide. It does not
generate exterior assets or activate a deployment.

Work started on `main` at `10a8800869a03ede0441b34a6e054d2fb499cbdb`, with
existing uncommitted town geography, server migration, 3D experiments and pixel
art work. Unrelated changes are preserved. No commit, branch, merge or deployment
lifecycle was requested in this slice. Generated sources and evidence remain in
an external working root, with machine-specific paths in its local handoff.

## Delivered cutover

- Default product and development root select Canvas 2D pixel art. Root and
  direct index entries agree; query parameters cannot select a renderer.
- Build-time module inspection rejects Three.js and diagnostic rendering in the
  pixel product. Explicit `inspection` builds remain for synthetic proof worlds.
- The former playable 3D renderer, actor/asset binding, receipt and associated
  tests are removed. Retired `first-expedition` and `pixel-temple` build modes
  are refused before creating output; active build callers migrate together.
- Deployment staging binds `pixel-manifest.json` to `pixelReceipt.json`, verifies
  referenced PNGs, excludes private source material and refuses mesh packets.
- The earlier local 3D feel scene remains at an explicit reference entry for
  historical capture tools. It is excluded from the product build and is not a
  fallback. Existing diagnostic Workbench rendering remains available.
- The interim pixel playfield suppresses diagnostic gameplay panels, directional
  buttons, instruction/coordinate readouts and decorative headings. Existing
  direct movement and resident interactions remain, with current-square double
  click selecting the server's sole enabled stair offer. Sign-in/session access
  remains; the real equipment and gameplay UI is deferred at owner direction.
- Authoritative cells, world geography, routes, resident identities, services,
  readiness and save state retain their owners. The temple has the selected
  artwork; other authored areas display an explicit map pending construction.
- The long presentation owner was decomposed into the current visual target,
  production method and clearly superseded 3D history. Architecture, browser
  operation, routing, checkpoint and deployment instructions now agree.

## Art and model handoff

The selected benchmark is the final `tomas-20260908-r5` packet recorded in the
[production guide](../pixel-art-production.md#selected-benchmark-and-its-limits).
Preserve its sampling, room display size, relative scale, material edges and
upright presentation when constructing the next area. The initial
[pixel-temple record](2026-09-08-pixel-temple.md) retains experimental evidence.
No new image generation or provider spending occurs in this cutover.

The owner prefers ChatGPT Images 2.5 if available after the Codex update. The
[tool policy](../pixel-art-production.md) records the verified announcement and
current inability to select or identify a model through the built-in tool. Use
the included generation route; do not substitute a separately billed API or
claim a version that the tool does not report.

## Verification

The selected fast plan escalated removed/renamed and deployment paths to
`portable, web`. That run completed: routing, whitespace, links, private-boundary
checks, Python suites, Rust formatting/build/clippy/tests, Workbench demo and web
install/typecheck/tests/build all passed. After the exterior-window fix and HUD
removal, the affected web/docs/boundary plan also completed, with **531 tests in
50 browser test files**. The product module audit passed in every default build.
The exact changed-path commands and complete outputs are retained with external
evidence; no unavailable capability was counted as a pass.

Deployment copy validation admitted 46 manifest/PNG files from the selected
packet; the native temple proof uses that copied release artwork. Explicit
inspection service proof passed all three fixtures in Chromium, Firefox and
WebKit (nine fresh browser/scenario runs). It does not claim pixel visuals.

Final native pixel proof passed Chromium, Firefox and WebKit, with 13 accepted
commands each. It checks root/index/obsolete-query entry stability, absent
temporary HUD, 1024px desktop and 846px compact room, pointing after resize,
shared grid and furniture occlusion, resident identity/services and purchase,
explicit stair double-click down/up, ordinary native double-click movement,
reconnect/logout, no 3D resources, missing-image refusal and stale-digest refusal.
Actual full-page and room captures were opened in all three engines.

The final default artifact also passed the arrival/temple path-control loop in
all three engines, with 22 accepted commands each: draft cancellation, native
double click, one command per confirmation, cooldown locking, exterior windows,
interior entry/return, reconnect and restoration to the original position.
All scratch native servers shut down through their launcher contexts. The default
working build remains the pixel product; no diagnostic artifact replaces it.

No hosted activation, outside-player test, full gated/clean-clone lane, packaged
Tauri test or low-end/crowd performance claim is made. Earlier private deployments
retain their own historical evidence.

## Findings resolved during cutover

The default build audit initially found Three.js entering through pixel asset
hashing's import of the earlier manifest parser. Hash verification now has one
renderer-independent owner, consumed by both packet loaders and its existing
positive/negative digest proof. The default bundle passes the module audit.

The native arrival path proof found that the temple loader incorrectly required
whole-member bounds. Exterior authority supplies a moving window. Pixel binding
now checks that window against the authored member and compares only its exact
passable subset, retaining missing, duplicate and out-of-window row refusal.
Moving-window regression tests and the native arrival/temple route cover the fix.
No gameplay authority or server context shape changes.

## Findings and next dispatch

The next task starts with the temple frontage and adjoining street in the
current authored town, including an entrance and at least one tree. Reuse the
selected characters to judge scale. Separate ground, structures and foreground
occlusion; align every layer and target to the shared authoritative grid. Prove
an interior/exterior transition and native pointing before extending the town.
Reusable chunks should preserve navigable widths; a whole-town painted image
must not become another collision or occupancy owner.

Open work belongs to the presentation/browser owners: matching Martial Artist
and Tomas gaits, exterior asset assembly, creature/combat/equipment art, low-end
and crowd performance, dead-world treatment and editable-master promotion.
The native temple loop recorded draw p95 around 1.1ms in Chromium, 2ms in Firefox
and 23ms in WebKit. These are local Canvas draw samples from functional runs,
including concurrent browser activity, not device or crowd benchmarks. Before
expanding performance claims, the browser owner should profile the foreground
and sprite draws in an isolated WebKit run and test the intended crowd/device
load. Evidence is the three `pixel-temple.json` reports and matching native
captures in the external cutover packet. This is an open measurement task, not
a claim that the current 30Hz draw budget failed.

Existing 3D bank-view performance findings are retained as historical evidence;
they are not the next rendering assignment. The browser matrix does not establish
Tauri platform support or owner-device performance.

Existing hosted preview artifacts are unchanged by this local cutover. Any
subsequent activation must stage the matching browser and pixel packet together.
After the Codex update, read this record, the current presentation owner and the
production guide; continue exterior construction without reopening pixel art
versus 3D or revisiting settled sampling decisions without new evidence.
