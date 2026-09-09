---
last_updated: 2026-09-08
revision: 3
status: Clearer shallows, stronger exposed swells and exterior daylight deployed privately; visual and owner-device acceptance open.
public_safe: true
summary: Coastal transmission, regional wave movement, renewed owner correction and native comparison evidence.
---

# Coastal water: readable shallows and regional movement

This Planning record owns the September 7 research and execution follow-up.
[Presentation direction](../presentation-direction.md#coastal-water-by-depth-and-exposure)
owns the owner's visual requirement; [browser client](../browser-client.md#coastal-water-and-exterior-camera-comparison)
owns the current renderer. Implementation owner: browser presentation, with
authored scenic depth and exposure data validated at the asset boundary.
Status: open. On September 8 the owner explicitly included this water pass in the
seven-building town implementation. Native evidence and owner device/visual
acceptance remain distinct requirements.
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

## September 7 source audit

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

## Dispatched experiment

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

## September 8 candidate and measured comparison

The candidate implements a continuous seabed, depth-dependent transmission and
absorption, and smoothly blended scenic depth/exposure zones. Both geometric
swell and fine normals quieten in sheltered shallows. The arrival dock and inlet
use shallow sheltered profiles; the adjacent channel is deeper and exposed.
Validated profile data owns those choices outside the shader. The browser
renderer contract owns the implementation; gameplay water cells remain unchanged.

External packet `town-20260908-r1` retains before/after native captures at 768 by
512 for dock, shelf, channel, inlet and night dock, plus six successive dock
frames per engine. The comparison swaps only the water/terrain modules while
holding candidate town assets constant. Chromium, Firefox and WebKit all rendered
without page errors. The original draft comparison is superseded by
`evidence/water/proof.json` and its final-shader captures.

On local Intel UHD 630, Chromium median frame intervals were approximately
16.7 ms in both variants. Firefox and WebKit dock/shelf/inlet medians were 25 ms
in both variants. Channel medians changed from 21 to 25 ms in Firefox and 24 to
27 ms in WebKit; Firefox's night-dock sample also contained a 228 ms p99 hitch.
These are 180-frame requestAnimationFrame distributions, not GPU timings or a
performance acceptance. Repeat the channel and night cases during the owner
device review; do not hide their tails behind the dock median. The focused
server certification overlapped part of the Chromium samples, so this is a
bounded comparison rather than an entirely idle-machine benchmark.

The new water adds one draw call and geometry, with unchanged texture counts
and no new render targets or scene-copy pass. After scene disposal, both variants
report zero geometries and one remaining texture counter. The equality proves
no added remainder for this pass; it does not establish a zero-resource teardown.
Browser presentation owns identifying that baseline counter if disposal is
reopened. The unchanged coast/dock mask and all seven service round trips passed
separate native and authoritative proofs.

Issue #48 remains open for visual/motion judgment, channel/night performance
follow-up and the owner's Lenovo T495s Vega measurement. Current exteriors and
water remain candidates; this record does not accept the artwork.

## September 8 clarity and daylight correction

The owner judged the first transparent-water candidate insufficiently clear and
asked for a visible shallow bottom, more active deeper water and less flat town
lighting. The next experiment keeps the current scenic profiles and geography.
The source audit found dark surface tint over a low-contrast muddy bed and a
combined exposed swell amplitude of only 0.05 world units. The new candidate
uses lighter sand/stone, reduced shallow absorption, larger exposed swells,
surface highlights and restrained moving light on the shallow bottom. Daylight
fill is reduced relative to its directional key. The
[browser contract](../browser-client.md#coastal-water-and-exterior-camera-comparison)
owns these implementation choices; the
[town iteration record](2026-09-08-town-visual-iteration.md#water-clarity-and-daylight-follow-up)
owns this pass's exact native, deployment and preservation evidence.

The previously inspected primary technical sources were checked again. Rare's
paper supports scattering/crest colour, zoned foam and sun highlights, but does
not establish the reference game's coastal transmission equation. The new
analytic swell/transmission shader is an original implementation in this stack.
The visible result and owner-device performance remain acceptance questions.
