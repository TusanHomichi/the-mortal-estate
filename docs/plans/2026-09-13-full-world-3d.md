---
last_updated: 2026-09-13
revision: 1
status: Full-world 3D implementation and regression coverage complete; PR and installation receipts own verification and delivery outcomes.
public_safe: true
summary: Atomic full-world Three.js cutover, current-map candidate scenery, shared figures, proof and preview delivery.
---

# Full-world 3D cutover

The [presentation ruling](../presentation-direction.md#3d-reopening) dispatches
full 3D town and interiors with the existing dungeon and Martial Artist foundation.
The owner authorized end-to-end implementation, delivery, preview refresh and
cleanup. This record owns execution; client architecture and browser client own
the standing seam and delivered behavior respectively.

## Scope and ownership

- Replace the two-renderer canvas switch with one Three.js renderer. Share the
  owner-selected male/female Martial Artist bodies and authoritative motion path
  across every authored area, including existing non-martial player characters.
- Rebind original modeled town scenery to the current compiled structures and
  entrances. Reuse compatible original room assets; no map, save or timing change.
- Remove active pixel artwork loading, entry backdrop, deployment packet and
  pointing/proof dependencies together. Refuse retired product renderer shapes.
- Retain strict observer filtering, neutral semantic targets, current action
  offers, context-loss refusal and discardable graphics resources.
- Prove current town entrances, all rooms, dungeon transitions, both martial
  bodies, character visibility at arrival (6,7), reconnect and release startup.
  Investigate the existing dungeon wall/martial-pose finding in the shared path.

The renderer owns drawing only. Authored structures own footprint/door placement;
server frames own observed actors, contents and commands. Assets remain external,
digest-bound candidates. No screenshot becomes an accepted master, and this work
does not resume the paused identity-proof or presentation-adoption experiment.

## Cutover inventory and proof

Search `pixelPacket|pixelReceipt|PixelRenderer|pixel-manifest|pixel-art` across
`web`, `tools`, `tests`, `deploy`, and maintained owners. Search dungeon diagnostics
and asset receipt consumers before moving shared code. Migrate active consumers
and corresponding refusal tests in one slice; historical studies remain dated.

Inspect and run the verification runner's changed-path plan, then configured full
verification before merge. Native release proof covers the browser roster and
actual assets separately from synthetic CI. Stage an immutable release, check
required CI, refresh the private preview with preserved saves, inspect installed
behavior, then remove only task-owned scratch resources and delivered branches.

## Evidence and findings

The implementation starts from main `0c72b4ce17557bc92537fdfeb4f52f4ee902beb2`.
One Three.js renderer now owns all twelve authored areas. Original modeled
exteriors are rebound to current footprints and entrances; original room models
and resident rigs are reused. Both martial player bodies retain their bound
walk and combat clips. The deployment packet contains only the 25 receipt-bound
GLBs. The server binary and all authored content remain unchanged.

[PR #70](https://github.com/TusanHomichi/the-mortal-estate/pull/70) owns exact
source-verification, native browser, CI, merge and cleanup outcomes. Immutable
release and activation receipts in the private installation own the deployed
file map and save-conservation result. This avoids a changing document becoming
a competing installation receipt. Source merge alone proves no deployment or
visual acceptance; private paths, credentials and captures remain outside Git.

The native matrix covers all seven town entrances, all four dungeon floors,
both bodies walking through temple/town and returning, resident services,
entry/creation, doors, stairs, real combat, both defender bodies and the
occupied-hand negative control. Installed inspection admits all three existing
characters across the browser roster, reconnects at the same position and logs
out with zero gameplay commands. The linked PR records observed outcomes.

Visual review caught two distinct occlusion causes. Per-mesh opacity compounded
through overlapping roof pieces; a nearest-surface depth pass makes them blend
once. The retired soft-shadow alias then changed the renderer's shadow sampler
mode on refresh, while repeated cached frames could skip that normalization.
This produced GPU draw errors and missing buildings. The renderer now uses the
supported settlement shadow mode. The initial menu view also requests its first
shadow build before drawing. Native GPU proof passes in all three engines
for overlapping geometry, cached lit frames, restored opacity and resource
cleanup; the retired-mode negative control produces the expected GPU failure.
Live proof now collects GPU console errors as well as JavaScript errors.

Proof setup retains the fixture's original class when selecting a body variant.
Walking captures use a complete three-step accepted route and check the pose on
both sides of capture. Resident-menu proof clicks the actual visible button
position while the live feed refreshes its rows, then checks the accepted command.

[Town visibility #69](https://github.com/TusanHomichi/the-mortal-estate/issues/69)
requires the final bank-side native and installed captures. The prior
[dungeon pose overlap #68](https://github.com/TusanHomichi/the-mortal-estate/issues/68)
continues to own takeoff, impact, landing and settled-pose review; this cutover
does not grant motion-phase or asset-master acceptance. The independent staged
whitespace check caught and removed a trailing blank line before commit; the
runner's unstaged-only gap is filed as
[#71](https://github.com/TusanHomichi/the-mortal-estate/issues/71).
