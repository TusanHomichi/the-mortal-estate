---
last_updated: 2026-09-11
revision: 2
status: Runtime exports and native proof complete, with explicit artwork and gameplay limits retained.
public_safe: true
summary: Bounded dungeon character playback, authoritative event consumption, male/female walk variants and proof plan.
---

# Martial motion integration

The owner accepted the shared flying-kick, guard, punch and block study as good
enough to wire into play, then requested Meshy walking clips including a female
walk. This Planning record owns execution and proof. Presentation direction owns
visual acceptance; browser client owns delivered behavior.

Work began on `main` at `8936766cd550fbd0714561d32ac48c1038356fb4` after the
repository audit merge. Existing gameplay-baseline and settled-conclusions edits
recording the owner's martial combat direction are preserved in this slice.
The owner subsequently authorized commit, PR, merge and cleanup after completing
the integration; full local verification and both CI jobs precede merge.

## Scope

Use the existing private asset mount and digest receipt for baked male/female
character GLBs with reusable clips. The controlled Martial Artist's existing
identity selects the body; unspecified sex display uses the provisional male
body. Other actors retain their existing candidate because the observed actor
rows do not carry their class/body identity. No creation or wire schema change.

Consume visibility-filtered events only from newly accepted state updates.
Confirmed unarmed fight outcomes select fist variants; confirmed jumpkick uses
the flying-kick clip; blocked melee outcomes select a generic cover. The public
blocked outcome does not identify hand, shield or armor source. Neither animation
nor local timers create attacks, damage, readiness or target movement.

The existing jumpkick rule can attack at range but does not commit the closing
movement described by the later owner ruling. Keep its visual clip in place.
The complete three-tile entry remains with the future combat-rule slice; this
integration must not fabricate an actor-moved event or a landing coordinate.

Walking consumes the exact current visible actor-moved chain, including local
doors; transitions and incomplete chains snap to authoritative placement.
Gait phase follows rendered distance. The observer's remaining deadline bounds
visual movement, and a ready frame finishes it. Town/interior rendering remains
with its current area selection. Individual animations can interrupt one another;
no animation completion gates controls.

## Proof plan

Prove event filtering, ranged/no-sight/not-ready refusal, duplicate snapshot
handling, hidden counterpart safety, complete movement chains, two-body clip
binding, resource disposal and body choice with focused unit tests. Verify the
baked exports and native walking/combat playback, including body variants, in the
browser roster against disposable authority. Run the selected repository runner,
stage an immutable matching release, prove it, then refresh the private preview
under standing authorization while preserving saved state. Record actual results
and unresolved limitations here at closeout.

## Observed delivery and findings

Two original shared-rig bodies now carry guard, walk, four punches, four blocks
and flying kick. The male uses Casual Walk; the female uses Walking Woman.
Acquisition used six authorized credits. Runtime exports retain the original
65-bone rest skeletons and weights, bake the shared driver result, embed textures,
close walking endpoints and remove walk/kick planar travel. Five surface samples
per clip survived independent GLB reimport within 0.014 mm across all 22 clips.
Private source, conversion scripts and digest receipts remain outside the repo.

The immutable candidate passed all fifteen existing dungeon scenarios and six
additional male/female motion scenarios in Chromium, Firefox and WebKit, each
with observed hardware rendering. Real server movement selected each walk;
confirmed fight and jumpkick feedback selected their clips and returned to guard.
Reconnect retained authoritative position without replay. Both new asset files
were independently refused when missing or digest-mismatched. No runtime or
animation-binding errors were observed. Native captures were visually inspected.
Block selection is covered by wire-shaped unit fixtures; a deterministic incoming
block was not added to the native scenario, so this record does not claim one.

PR review found that local doors emitted internal world-transition events that
were absent from observed movement. Rules now projects only adjacent, same-level,
self-targeting door transitions through the existing visible movement event.
Paired doors, other transitions and hidden actors remain excluded. A regression
test consumes actual committed closed-door and open-door path events; native
motion scenarios now require walking onto a closed door and sprinting through
the opened doorway. This correction requires fresh release and full/native proof;
the delivery PR carries the final receipts.

The first selected verification run was COMPLETE: docs, real-denylist boundary
checks, TypeScript, all 581 browser tests and the production build passed. Parent
review removed a minimum travel duration that could exceed the server remainder,
refused partial movement chains, made punch variety independent of sequence gaps,
and included events in same-sequence conflict refusal. The delivery PR owns the
final `python3 tools/run_verification.py --scope full` receipt. The private
immutable-release receipt owns installed source, served-file comparison,
activation and saved-state preservation. A later source tree can inherit native
evidence only when every delivered file and contract matches the tested release.

The supervised native-proof worker was stopped after more than twelve minutes
without an artifact. The parent implemented and ran the bounded native proof.
Initial fixtures were corrected after real authority refused a class-ineligible
skill and an unowned second player; neither failure was counted as a pass.

Remaining work: presentation owns body/clothing refinement, brighter character
fronts and visual acceptance of the new walks. Client/protocol identity work owns
extending model choice beyond the controlled character and exposing a player
body choice; this slice adds neither. Rules owns the complete closing flying kick,
obstacles and impact placement under the recorded gameplay direction. No standing
ordinary-walk or attack deadline, stored world, or save schema changed.
