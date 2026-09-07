---
last_updated: 2026-09-07
revision: 1
status: Authoritative footprint controls restored and deployed; full exterior/temple input proof passed in all three browsers.
public_safe: true
summary: Restored footprint drafts, endpoint confirmation, cursor feedback, deployed proof and the remaining exit-framing finding.
---

# Playable path controls

The owner reported that the current rendered game had lost the previous scene's
first-click footprint route and second-click commitment. Inspection confirmed
that the live play entry was still wired to immediate cardinal adjacent clicks;
the earlier scene's route presenter remained separate. This slice restores that
interaction through existing server path assessment and sequenced movement.
[Browser client](../browser-client.md#authoritative-play-controls) owns behavior.

The source base is `ce4ea993d05576f35e99969a2e991504c9d42cd1` with the carried
dirty tree and prior renderer/login repairs preserved. Runtime changes are
confined to browser input, non-mutating preview transport and presentation.
Server rules, timing, protocol, authored geography and account data are unchanged.
The [horseshoe town study](2026-09-06-town-buildout.md#horseshoe-layout-study)
remains the next geography task; this input repair does not implement it.

Shared geometry search and original sole texture generation now serve both the
earlier scene and authoritative presentation. No local simulation or candidate
passability has been attached to the server-controlled character. First clicks
send read-only assessments; confirmation sends one immutable movement command.
Foreground roof/tree picking was also repaired because it displaced the ground
endpoint selected by the user. Matched authoritative route prefixes guide short
walking animation; transitions discard the old space's route.

Focused regression cases cover cancellation, replacement targets, fast
double-clicks, partial refusals, unrelated frame refreshes, position changes,
cooldown, disconnect and epoch correlation. Browser proof must additionally
show visible footprints outdoors and inside the temple, retain the player
position across reconnect, and restore the test character after its route.
Screenshots and host receipts stay in the external visual lab.

## Findings and verification lessons

The first native Chromium run completed exterior movement and the temple's
three-square routes, but the proof clicked below the canvas when returning
through the temple exit. The current fixed framing shows only the northern
part of that edge square. The harness now clicks the visible part and still
asserts the exact drafted endpoint before confirming. This corrects input
automation; it does not change camera geometry or substitute a keyboard move
for pointer proof. The interrupted test character was returned through the
ordinary UI before repeating the complete path.

**Open visual finding:** the temple exit square is clipped at the lower edge
of the current viewport. Presentation/camera ownership must address the entry
framing in a subsequent room review, retaining the fixed angle and tactical
scale. Acceptance requires a visible, comfortably targetable exit at native
play size. The external temple-draft capture records the current framing.

An overlay review also found that disabling depth tests painted the glowing
soles across the character. The final material respects scene depth. The
ground outline remains an input aid; footprint artwork follows terrain height.
Transition tests caught and fixed old-space committed footprints surviving
arrival in a different member. The unit regression preserves that distinction
from normal same-space movement and cooldown.

## Delivered proof

The final release is `b18da2ca2f95d6af90144f8fd043003edec3efe6`, staged
through an isolated source index and activated with a backup and healthy service
checks. The real source index and Git ancestry remain unchanged. The exact
external receipt binds the release, source files, host activation and captures.

Chromium, Firefox and WebKit each passed the complete deployed pointer proof:
first-click non-mutation, second-click commitment, a native double-click,
Escape/right-click cancellation, cooldown input refusal, reconnect invalidation,
three-square routes outside and in the temple, and entry/return across its
portal. Each run issued exactly six movement commands and fifteen read-only
server previews. Each restored the inspection character to its original square
and signed out. No owner character was entered or moved.

The selected verification completed successfully: 511 web tests, type checking,
both builds, documentation routing/links/whitespace and the public-boundary
checks. Runtime source changes are browser-only; no new backend semantics or
storage migration required a Rust/server test lane. Final exterior and temple
captures were opened and inspected; the clipped temple exit remains the explicit
visual follow-up above. The earlier failed proof and discarded depth treatment
are retained separately from final evidence. No artwork-master acceptance or
production HUD integration is claimed by this controls repair.

The final deployed build also passed the existing login-startup safety proof in
all three browsers: absent JavaScript, blocked native submission and delayed or
failed codec startup keep credentials out of native form navigation.
