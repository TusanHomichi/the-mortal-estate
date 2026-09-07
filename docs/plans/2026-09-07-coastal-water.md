---
last_updated: 2026-09-07
revision: 1
status: Open follow-up; source research and current shader audit complete, implementation not dispatched in this slice.
public_safe: true
summary: Transparent shallows, calmer sheltered water, depth transitions and a bounded native performance comparison.
---

# Coastal water: readable shallows and regional movement

This Planning record owns the September 7 research and execution follow-up.
[Presentation direction](../presentation-direction.md#coastal-water-by-depth-and-exposure)
owns the owner's visual requirement; [browser client](../browser-client.md#coastal-water-and-exterior-camera-comparison)
owns the current renderer. Implementation owner: browser presentation, with
authored scenic depth and exposure data validated at the asset boundary.
Status: open; research does not constitute implementation or visual acceptance.
Working index: [issue #48](https://github.com/TusanHomichi/the-mortal-estate/issues/48).

## Evidence and limits

Primary sources inspected on September 7:

- Rare's [2018 technical-art paper](https://history.siggraph.org/wp-content/uploads/2022/09/2018-Talks-Ang_The-Technical-Art-of-Sea-of-Thieves.pdf),
  section 1.1, describes FFT ocean displacement, approximate scattering colour,
  crest and intersection foam, and different foam treatment for calm, normal
  and stormy water. It establishes a combination of effects, not one wave shader.
  Section 1.2 concerns ship-deck water, streams and waterfalls; it does not
  specify the coastal seabed transparency algorithm.
- A [contributing developer's portfolio](https://kstocky.github.io/portfolio/games/sea-of-thieves/)
  explicitly identifies a system for authoring calm and rough water zones. This
  supports regional control; it does not disclose the blend equation or prove
  that distance from shore alone defines those regions.
- Mark Finch's [GPU Gems water chapter](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models),
  section 1.3, describes using bottom depth to control transparency, reflection
  and geometric wave amplitude. This is an independently documented approach
  suitable for evaluating here, not evidence of Rare's exact implementation.

The user's visible-bottom and calmer-shore observations are the target. The
exact reference game's coastal transmission method remains unverified by the
inspected sources. There is no verified claim here that its complete ocean is
cheap on integrated hardware. No external source payload or reference artwork
is imported into this repository.

## Current gap, inspected in source

`web/src/waterSurface.ts` supplies sampled bank height and nearest coastal-cell
distance. `web/src/space/coastalWater.ts` attenuates its three geometric swells
over that distance, but the fragment shader retains full-strength fine ripples.
Deep colour also follows shoreline distance, rather than actual water depth.
The material is opaque and writes alpha 1: the bottom cannot show through it.
Sampled bank depth is used for intersection foam only. Foam has no retained
history or dispersal; there is no authored sheltered/exposed zone field.

Consequences: a deeper channel near land can look shallow, a broad shallow shelf
can look deep, and reduced displacement can still appear restless because the
surface normals keep moving. Colour tuning alone cannot reveal the seabed.

## Recommended bounded experiment

Start with the arrival dock, a visible shallow shelf and adjacent deeper water
at the accepted camera and viewport. Reuse the shared scenic terrain sampler;
keep movement, swimming eligibility and water-cell authority in their existing
owners. Rendering depth does not establish gameplay depth.

1. Give the shallow shelf an actual visible, textured submerged bottom. Verify
   depth and render ordering around banks, rocks, dock supports and actors.
2. Derive optical depth from water height and the bottom; introduce transparent
   transmission with progressively stronger absorption/tint through deeper
   water. Evaluate simple transparency first, then bounded scene-colour/depth
   refraction only if its visual benefit earns the extra pass and memory.
3. Control swell displacement **and** normal-ripple strength with depth and
   exposure. Keep shore distance for edge effects; do not equate it with depth
   or shelter. Blend zones continuously and avoid disconnected tile waves.
4. Add restrained contact foam and moving highlights appropriate to calm water.
   Exposed water can carry stronger crests; reserve caustics and elaborate foam
   history until the shallow/deep distinction reads clearly.

These are proposed original implementation choices, not a requirement to copy
the reference's FFT or engine. Any new adjustable scenic data needs one validated
owner and refusal tests; do not scatter dock-coordinate special cases in shaders.

## Completion evidence

- Matching before/after captures and motion clips of dock shallows, shallow shelf,
  deep water beside land, sheltered inlet and exposed water. Bottom visibility
  must decrease with depth, with quiet shallow motion and no hard zone seam.
- Check oblique water/shore intersections, submerged geometry, actors, dock
  supports, day/night lighting and disposal. Reject sorting halos, detached foam,
  obvious repeated wave patterns and abrupt opacity boundaries.
- Prove unchanged authoritative cells and routes; test any newly authored depth
  or exposure data and shared sampling at boundary cases.
- Chromium, Firefox and WebKit native rendering and transition proof. Measure
  frame-time distributions, draw calls, render-target memory and GPU identity at
  the fixed aperture. A capped requestAnimationFrame sample is not GPU timing.
- Compare on the owner's Lenovo T495s integrated Vega before performance
  acceptance. Preview deployment and device access remain their own actions;
  this record does not claim that measurement has occurred.

Close only after the owner judges the motion and transparency against the target
and the measured baseline supports the selected effect budget.
