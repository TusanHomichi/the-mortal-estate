---
last_updated: 2026-09-13
revision: 1
status: Playback and rig visibility implemented; issue 68 carries the configured proof and preserved-state delivery receipt.
public_safe: true
summary: Bounded martial playback and wall-visibility correction, native phase evidence and preserved-state delivery.
---

# Martial motion at walls

## Scope and owners

The owner continued the full-world delivery after its remaining wall-overlap
finding, [issue #68](https://github.com/TusanHomichi/the-mortal-estate/issues/68).
Base: `1044e7e0f9f53eb778cbb17f088a02c7859c82bc`, the merged world cutover.
This Planning record owns execution and proof. [Browser client](../browser-client.md#dungeon-character-motion)
owns playback; [presentation direction](../presentation-direction.md#3d-reopening)
owns the existing camera, body and obstructing-wall treatment. Gameplay timing,
authored geometry, character identity and asset acceptance remain with their owners.

Inspect the actual male/female poses and animation weights, correlate their
rendered placement with accepted movement, and correct causal playback or
visibility defects. Cover takeoff, extended kick, landing and a settled follow-up
punch at the dungeon wall, plus the forge entrance. Reuse the bound assets.
No gameplay timing, route, damage, content, model-scale or save migration belongs
in this slice. Native evidence does not grant visual acceptance.

## Proof and delivery

Use synthetic regression tests for the causal defects and native browser proof
for the actual bound rigs. Capture actual framebuffer draws with their pose and
authoritative route facts; an action-name diagnostic alone cannot establish the
pose in the image. Inspect both bodies across the browser roster, run the selected
fast plan and configured full baseline, then deliver through the authorized Git
lifecycle and save-preserving private-preview refresh. Retain evidence and the
previous working release; remove only this slice's superseded disposable outputs.

## Findings and correction

- The old bone-name filter matched only `Head` in each actual 65-joint martial
  rig. Sampling every joint restores the standing wall-fade treatment for limbs;
  no scenery footprint or body scale changes.
- The actual jab includes additional planar root displacement: native Three.js
  sampling measured forward envelope maxima of approximately 1.53 and 1.43 world
  units for male and female. Holding the bound motion root at its planar bind
  origin reduces those to approximately 0.90 and 0.80, preserving vertical lift
  and articulated tracks. The source assets remain unchanged.
- The bound flying kick reaches extension around phase 0.25 and starts landing
  around 0.45. Uniform route interpolation continued moving the recovering pose.
  Receipt-bound phases now put the accepted route endpoint at contact and keep
  the root there through recovery. The authoritative readiness interval is unchanged.
- The renderer capped pose advancement per frame while route progress and expiry
  used elapsed time. Playback now owns one elapsed presentation clock, including
  fading through an expired reaction during hidden time. Regression tests inspect
  actual transforms, phases and effective weights.
- The old fight capture recorded a clip label after a remote screenshot, leaving
  its exact pose phase uncertain. Native capture now records the framebuffer,
  current sampled pose, weights and rendered anchor in one draw turn. The initial
  corrected live punch contains only the jab; no residual kick contributes.

Initial selected fast verification completed all steps. The native transparency
and cached-shadow proof passed on Chromium, Firefox and WebKit. A three-square
male inspection on the staged candidate captured contact, landing and recovery
at the accepted endpoint, followed by a settled jab and guard. Final configured
proof, exact source binding and installation outcomes belong to the delivery
receipt on [issue #68](https://github.com/TusanHomichi/the-mortal-estate/issues/68);
these initial inspections do not stand in for that full result. The staged forge
inspection also passed with all 65 player joints participating, the obstructing
wall fading, and the same position and visibility after reconnect.

The native scenario roster includes one-, two- and three-square approaches for
both bodies, the existing traversal and defense cases, and both bodies at the
forge entrance. Synthetic presentation cases retain all three distances and
hit/miss/block outcomes. Technical correction and captures do not accept the
candidate appearance or settle owner visual acceptance.
