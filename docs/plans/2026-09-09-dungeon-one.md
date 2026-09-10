---
last_updated: 2026-09-09
revision: 13
status: Seven-cell framing and integer zoom verified; preview candidate held after upright-sprite rejection.
public_safe: true
summary: Four dungeon floors, verified seven-cell framing and zoom, held art candidate, paired stairs and preserved saves.
---

# Four dungeon floors

The owner expands the initial first-floor dispatch to all four historical dungeon
floors, omitting the fourth-floor locker cutout because town already reserves
storage space. The [geographic authorization](../public-boundary-policy.md#first-land-map-template)
and [gameplay baseline](../gameplay-baseline.md) apply. Temporary generated floor,
wall and door tiles are authorized; final artwork acceptance is not implied.

## Source selection and derivation

The selected neutral source is `dungeon-template-01`, original client revision
1.11, map SHA-256
`7d706510ef3fce815c6339247a8a461b2d034cdaf2bb08aa0dad1d00dbcde063`.
Private evidence includes cell roles, the annotated full map, a primary text-map
boundary note and independently recovered stair correspondences. No raw source
payload or private extraction script becomes a build input.

The four frames have widths 36, 38, 36 and 37, each 43 rows tall. Native horizontal
ranges are 0–35, 35–72, 72–107 and 108–144; authored vertical coordinates reverse
the native axis. Shared boundary columns preserve the cross-floor guild corridor
and grand doorway. Earlier research alignment ranges were search windows, not
exact floor envelopes: using them as envelopes truncated eastern rooms. The
primary text-map note explicitly duplicates the grand-door column. Seven lake
cells at native column 72, rows 30–36 belong only to the third floor; the second
floor represents those outside-footprint positions as rock.

The omitted locker cutout is the 90-cell rectangle at fourth-floor local columns
16–24, rows 25–34. It contains the storage room, open-air moat and rock, with no
true dungeon-floor or water corridor. All 90 positions are rock, with no locker,
altar, stair or portal retained inside. This does not delete a dungeon passage.

Eleven reciprocal stair pairs connect the four depths: three between the first
and second, five between the second and third, and three between the third and
fourth. The first/second-floor guild corridor, two adjacent grand-door pairs and
the second/third-floor water passage retain separate authored transitions. There
are 46 directed graph edges and 199 door endpoints; paired grand-door endpoints
replace local emissions at those cells. The eight existing town/temple members
remain unchanged by this expansion.

The temple stair remains first-floor `(25,7)`, with landing `(24,7)`. Existing
live entrance state shifts by `(19,3)` through the strict offline migration.
Other landings choose an adjacent ordinary floor when available. A second-floor
stair in a concealed alcove lands on itself and uses the existing explicit stair
action to return. These landing choices are provisional; graphic alignment does
not prove original server arrival, permission or timing behavior.

## Implementation and proof

Rules own door legality, concealment, action timing and checkpoint migration.
Authoring owns neutral terrain, initial topology and explicit deferred components.
Every declared entry contributes a reachability root; deferred roots must remain
separate from all ordinary entry components. Unaccounted islands and accidentally
connected deferred roots fail compilation. No permission or portal is inferred
from a graphic-only marker.

Concealed closed doors obstruct sight and movement as masonry. This deliberately
replaces the old action-context test's pass-through behavior. Discovery itself
remains unfinished. The client uses the bounded observer projection for all four
floors, routes through observed local doors, and submits exact server-offered
stairs from double-click or the observer's context menu. Hash-bound provisional
tiles are required by pixel packet version 6; stale versions and missing imagery
fail startup. The [production owner](../pixel-art-production.md#provisional-dungeon-tiles--september-9)
records their preparation and acceptance status.

Four-floor full verification completed with every selected step passing, including
PostgreSQL, native wire/browser checks and the clean clone. The run took 1,235.052
seconds. Native Chromium, Firefox and WebKit each completed the four-depth round
trip, local-door opening, combat, stair return, reconnect and logout. The restored
live-database rehearsal passed exact state conservation and two cold starts with
three characters, two accounts and one persistent world.

The later upright rendering correction passed the focused web/docs/boundary
runner (36.717 seconds). Separate native art proof passed all three engines at
1400-by-1200 and 1920-by-1080: raised scenery, both doorway orientations,
40% cutaway, ordinary movement, side-door opening and black unseen background.
These are functional receipts, not owner visual acceptance. Sources and private
receipts remain in the September 9 four-dungeon production packet.

The owner rejected the side-door proportions and directed a generated whole-room
camera guide. The subsequent construction continuation below replaces its
unreliable proportions with actual-renderer geometry. The
[production owner](../pixel-art-production.md#provisional-dungeon-tiles--september-9)
owns that review. The verified provisional build is now staged and activated
under the standing private-preview authorization; the deployment receipt below
owns the cutover and saved-state evidence.

## Findings and follow-up

- Gameplay rules own concealed-door discovery, cavern corner/vertical access,
  swimming, pits, fire, darkness and portal-garden access. Material distinctions
  are retained, but fire and darkness currently have ordinary floor behavior;
  pits, chasms and portcullises are blocked. No environmental parity is claimed.
- Rules navigation owns paired-door state: current reciprocal endpoints retain
  independent mutable open states. Their physical relationship needs recovered
  behavior and a test covering opening from either side before claiming parity.
- The [gameplay baseline](../gameplay-baseline.md) owns the owner's thieves-guild
  direction and sheriff/disguise research target. Preserve the basement passage
  and reserved surface stair. A concealed entrance near the town edge is the
  current implementation preference; no new surface building is authored here.
- Authoring owns reserved surface and external portal destinations. Their source
  markers grant no invented travel authority.
- Presentation owns finished dungeon artwork. Candidate tiles provide readable
  floors, walls and open/closed doors; they are not accepted masters. The owner
  requires corridor orientation, walls and doors taller than actors, and 40%
  opacity when scenery obscures a player or monster; copied passability stays
  unchanged.
- Browser verification owns the older `expedition-proof.mjs` diagnostic caller's
  stale initial arrival coordinates and retired HUD assumptions. Its allocation
  helper and dungeon coordinates were migrated, but that caller is not claimed
  as complete pixel-client proof. Native pixel proof owns the playable route.
- Verification environment lesson: the runtime private denylist must not be
  exported over Cargo’s synthetic sentinel list. That override produced two
  expected-refusal test failures; rerun through the normal Cargo environment.
  Runtime migration and publication boundary checks retain the real private list.
- Supervised external workers supply bounded implementation and review. Parent
  review caught the search-window boundary error and completed the earlier
  timed-out migration attempt. Worker output does not establish source fidelity
  or replace independent compilation, diff review and gameplay proof.

Work remains uncommitted on `main`, based on
`5d01572b10741a89dc379914b146a39157eb7aaf`, with preserved entry-flow changes.
No Git lifecycle operation is part of this dispatch.

## Town-camera construction continuation

The subsequent owner dispatch keeps town as the first camera reference, allowing
a dungeon-specific treatment if it proves necessary. The new provisional slice
uses town ground spacing, a single material-bound door leaf, fixed hinge/frame
geometry, textured caps and continuous foreground cutaway. The
[production owner](../pixel-art-production.md#provisional-dungeon-tiles--september-9)
owns the calibration and reference method. The previous independently resized
side-door packet is retired atomically; the new packet refuses versions 1–5.

A bounded external geometry review completed in 53.958 seconds. Its hinge,
edge-on projection and per-component depth findings informed parent tests. Its
suggestion to fade the actor instead of the wall was rejected because it did not
match the owner's treatment. Parent review, native capture and final verification
remain necessary; the worker did not edit or inspect repository files.

Focused geometry/packet proof passed 14 tests. Initial Chromium gameplay captures
exposed and then corrected stretched masonry and isolated tile-stripe cutaway.
The current packet passed a fresh four-depth gameplay round trip in Chromium,
Firefox and WebKit. Subsequent narrow-coping native art proof also passed all
three engines at 1400-by-1200 and 1920-by-1080, including fixed frames through
side-door opening, raised walls, black unseen terrain and 40% cutaway. The actual
construction sheet and native captures were opened for inspection. These receipts
remain functional/implementation review, not owner acceptance of final artwork.

The owner then rejected full-cell wall tops as overextended platforms. The
renderer now uses narrow coping and matching thin front-door frames under the
production owner's dimensions. Native inspection also caught duplicate front
elevations where floors adjoined both sides of a wall; geometry proof now refuses
that duplication. A distinct top value separates coping from upright masonry.
This is a visual construction correction; terrain occupancy and actor angles
remain unchanged.

## Private preview cutover

Final web/docs/boundary/Python verification completed in 79.733 seconds, with all
550 web tests and 553 Python tests passing. The unchanged backend/content retains
the full-verification and restored-world rehearsal evidence above. Both the
current four-depth tour and final narrow-coping art proof passed all three engines.

The immutable release source tree is
`12e90ce3db9438d5625e0e7d47f3c6d738868c75`, based on the unchanged HEAD above.
An isolated index carried the release snapshot; the real Git index and HEAD were
verified unchanged. Backup preceded the offline content migration. Its identities
and operations matched the rehearsed plan exactly; checkpoint comparison proved
all non-geographic state conserved. Three characters, two accounts and one world
survive. The server became gameplay-ready before remote access reopened.
Hosted Chromium, Firefox and WebKit passed sign-in, manual creation review
without mutation, native movement round trips at both display scales, reconnect
and logout. Final comparison preserved player location, attributes, resources,
items, banks and lockers. Counts remain three characters, two accounts and one
world, with zero open connections after proof. The served packet matched the
source-bound manifest digest. Hosted captures were opened for inspection.
Machine-local deployment, backup, native and hosted receipts stay outside the
checkout. Artwork remains provisional, with no owner master acceptance implied.

## Lone-occupant centering

The owner requires a lone player to remain centered on their tile, with spacing
adjustments reserved for actual shared occupancy. The browser owner's placement
rule now supplies one anchor calculation to drawing, contacts, picking and wall
cutaway. Review found that destination-based spacing displaced stationary actors
before an approaching occupant arrived. Focused proof covers approach, arrival,
departure, removal and moving actors; contact rings now use the foot anchor exactly.
The larger-crowd layout remains explicitly provisional in the browser contract.

The selected web/docs/boundary runner passed all 553 web tests, typecheck, build
and document/boundary checks in 31.031 seconds. Native proof adds an exact lone
player/tile-center comparison at both display sizes. Final native and hosted
receipts belong to the private September 9 tile-centering packet.


The centering continuation subsequently passed native and hosted proof in all
three browsers and was deployed with the same content and preserved saves. The
owner then requested matching the dungeon player's size to the town doorway
reference. The production owner now records the shared adult height and resized
structural openings, including a newly proportioned original leaf source. This
follow-up retains centered ground contact, narrow coping and the existing camera
axes. Its selected web/docs/boundary run passed in 33.679 seconds, including all
553 web tests. Native Chromium, Firefox and WebKit passed the resized structures,
both door planes, centered actor anchor, movement, opening and cutaway checks at
both display sizes. The actual-renderer construction sheet includes a native town
bank crop beside the same adult reference; native scene captures were inspected.
The new packet was backed up and deployed without content changes or save loss.
Hosted Chromium, Firefox and WebKit passed the matching packet, sign-in, manual
creation review, movement round trips, reconnect and logout. Final comparison
preserved player state, items, banks and lockers; three characters, two accounts
and one world remain, with no open connections after proof. The generated leaf
source is 841-by-1870, preserving the prepared 36-by-80 aspect without stretching
the hardware. Exact prompt, original source, native captures and deployment
receipts remain in the private town-scale packet. Final art acceptance remains
with the owner.

The town-scale release source tree is `234557d8e05b10c85c47eef8b83aef3d74653544`.


## Upright visibility continuation

The owner reports cramped corridors and questions a dungeon-only camera change
because town deliberately presents upright subjects. The bounded candidate keeps
that presentation and the shared adult/door scale. Production owns the enlarged
floor spacing; the browser owns cutaway for observed empty routes. No authority,
visibility, content, collision or timing changes are included.

A supervised DeepSeek geometry review completed in 20.705 seconds. It identified
wall height relative to floor depth and actor-only cutaway as likely contributors.
The parent retained the current ground aspect and tall structures in response to
the owner's consistency concern, choosing more floor area and observed-floor
fading first. Larger cells reduce the number of map squares visible per viewport;
this is a provisional comparison, not final camera or artwork acceptance.
The first Chromium native run passed and its closed/open-door captures were
inspected. Focused tests cover empty observed ground and refusal of blocked,
unknown and absent cells as fade targets. The selected runner passed all 554 web
tests, typecheck, build and document/boundary checks in 40.259 seconds. Native
Chromium, Firefox and WebKit passed at both display sizes, including observed-floor
cutaway, ground-ratio preservation, centered feet, opening and fixed frame geometry.
The actual-renderer construction sheet was refreshed alongside the scene captures.
The candidate was briefly activated before the owner directed a reference-game
walkthrough before major presentation decisions. The previous town-scale release
was restored immediately, with content and saved player/item/bank/locker state
preserved. That candidate was held locally; hosted proof of that withdrawn revision is not claimed.
Private receipts belong to the September 9 visibility packet.

The local reference client signed in using its saved credential, then entered the
world after resizing and reconnecting. Direct VNC input and the saved path/commit
keys permit movement. The owner explicitly requires dungeon observation before
choosing a presentation change. The live character descended the temple stairs
and moved two tiles south in the first-floor entry room. The capture shows square
ground tiles, a roughly seven-cell scene width, narrow caps and structures whose
visible faces occupy about one tile of projected depth. No matching change to
TME is accepted from this observation. The intended doorway circuit was not
completed: the character died while the operator inspected captures. The owner
authorized release; the character returned to the temple with level/experience
unchanged and losses of one constitution, five maximum HP and two maximum
stamina. The owner confirmed no corpse recovery was needed for the unequipped
Martial Artist. The local packet keeps the live capture and recovery record.
Presentation owns further comparison; no larger floor/camera decision is made.

## Square-ground comparison after the reference visit

The owner dispatched a trial with square floor cells and upright walls covering
about one row, then explicitly permitted a steeper dungeon-specific view of both
structures and characters. Presentation direction owns that permission; pixel
production owns the comparison calibration. Existing subject art is retained in
this first pass so the floor and framing difference can be judged independently.
No new art master, physical camera angle, geography or gameplay rule is introduced.

The first native Chromium pass verified square ground, frontal coverage within
one row, lone-player centering, both doorway planes, fixed-frame door opening,
observed-floor cutaway and black unseen background. Its desktop capture exposed
a framing cost: the prior enlargement showed fewer than four square rows. Dungeon
enlargement is therefore bounded separately, and changing levels recomputes the
viewport so town regains its original framing. A supervised DeepSeek review also
identified the visible-row tradeoff and side-wall draw-order concerns. The parent
retains the existing ascending near-edge order and joined corner geometry; the
worker's proposed reverse order and door redesign were not adopted.

The blast-radius search covers dungeon cell sizes, projected wall/cap bounds,
viewport selection on resize and level change, native proof expectations and
prior held-candidate statements in presentation, production, browser and checkpoint
owners. Work continues against `5d01572b10741a89dc379914b146a39157eb7aaf`, preserving
the existing uncommitted work. The final selected-runner and native results
follow below. Artwork acceptance remains with the owner.

The initial square-cell / capped-enlargement pass passed native Chromium, Firefox
and WebKit. The owner then identified the distinction between a seven-cell
window and the current server's radius of seven. The owner selected a fixed
7-by-7 dungeon display, asked it to fill the screen, and directed integer
scaling through pixel correction. This supersedes the temporary 2x cap.
The current renderer selects 49 positions without mutating the server frame,
filters displayed actors and contents to that window, centres the observer,
and excludes caption margins from drawing and pointing. A 32-pixel scenery cell
permits whole-step zoom at phone, handheld, desktop and 4K resolutions. Character
sampling retains the existing finer raster budget.

A built-in image generation produced the two-angle comparison; its prompt,
original output and screenshot input stay in the private production packet.
The owner prefers its 60-degree view as the next artwork guide. It is not live
angled character art or a geometry master. The hunting/safe-area timing proposal
is routed to the gameplay baseline. The native proof now covers four display
sizes and a real stair round trip to verify restoration of town framing.

Final selected verification completed in 44.914 seconds: all 557 web tests in
58 files, typecheck, build, documentation and selected boundary checks passed.
Native Chromium, Firefox and WebKit passed at 1400-by-1200, 1920-by-1080,
3840-by-2160 and 360-by-640. Proof covers the 49-cell footprint including dark
positions, centred player, whole-step zoom, exact repeated floor-pixel blocks,
wall coverage, opening the side door, preserved frame geometry and a temple
stair round trip restoring town display scale. The first fixed-frame proof
exposed a harness assumption that the temple arrival is already on its descent
stair; the final proof walks to the actual stair before returning.

The owner rejected the upright character in this view. The verified candidate
therefore remains local; no activation or hosted proof of this revision is
claimed, and the previous private preview release remains current. Matching
overhead art studies were generated and inspected under the production owner.
Their unresolved foot placement and doorframe fidelity remain explicit, and no
study image was silently installed as a runtime asset. Next work is directional
character and architectural artwork matching the selected guide, followed by
native visual review. Gameplay timing and server observation range are unchanged.
