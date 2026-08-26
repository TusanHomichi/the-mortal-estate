---
last_updated: 2026-08-27
revision: 3
status: Owner-authorized bounded execution plan, first tracked by private-archive issue #24. Target work may begin; executable integration waits for target acceptance, an authoritative TME recording, and a separately reviewed immutable Nomos dependency point. The clean public-source cut is not external-boundary activation.
public_safe: true
summary: The bounded plan for testing whether pinned Nomos presentation authoring can produce The Mortal Estate's accepted gameplay look while TME retains every gameplay fact, browser and Godot consume the same input, and reusable findings land upstream first.
---

# Nomos presentation-adoption experiment

## 0. Authority and exact inputs

The owner authorized this bounded resumption on 2026-08-26 under the private
development record's issue #24; this plan carries that authority publicly.
The [genesis ledger](genesis-ledger.md) owns the Phase 10 order and final
disposition. [Presentation direction](../presentation-direction.md) owns the
target, accepted masters, visual criteria, and taste verdict. The existing
[boundary map](../boundary-map.md) continues to own every gameplay fact.

The inspected TME input is commit
`087633bca77a8623f1a3834598db1c2de7d9a780`, tree
`e00c2970cf3d85cd66d20dff4f59891c33c9d12e`.

This authorization branch proceeds from TME commit
`8f0c0f524b03e8ec951389beeac5165d4d6bb180`, tree
`d28c4f9f3e0729ef66b2797ff1ebe5e69bf607f8`, which raises the explicit Rust
toolchain baseline to 1.98. No gameplay, protocol, presentation, Godot, content,
or dependency fact changed between the inspected input and this execution-plan
baseline.

Nomos authorized the corresponding evidence program in decision 0022 at commit
`5e0e44cc912b57a1d29cc3e722497c16cf9a1797`, tree
`7e606bde9f91307483307c4af1e0764d81df5c72`. That is an authority point, not an
executable dependency point. A later Nomos issue must establish an immutable,
reviewed, digest-recorded dependency before integration begins.

This plan imports no Nomos gate, decision-document family, or authority into
TME. Each repository owns its own present facts.

## 1. The question

Can a pinned Nomos presentation-authoring and compilation boundary produce
TME's owner-accepted representative gameplay look, including a second matching
asset, without asking authors for raw transforms or final pixels and without
duplicating gameplay authority?

Presentation is the first seam because repeated visual-pipeline reboots are the
observed problem. Nothing here authorizes replacement of TME's rules, server,
persistence, pulse, protocol, client, or live network path.

## 2. Phase P0 — target authority

The owner creates and judges the exact packet defined by
[the current target-authority owner](../presentation-direction.md#the-current-target-authority-packet).
It is tested against the standing
[production rule](../presentation-direction.md#the-production-rule) at actual
play size.

The packet and every editable source receive one content digest. The owner then
records exactly `accepted` or `rejected`, the review size, and the artifacts
seen. Rejection closes issue #24 with a stop disposition. Acceptance permits
prerequisite work; it does not approve Nomos, a presenter, or a platform.

## 3. Prerequisites for executable evidence

All of these must exist together:

1. the accepted target packet and its digest;
2. an immutable, reviewed Nomos dependency point and artifact/schema digests;
3. the named authoritative TME recording below, containing every fact the
   representative scene needs;
4. an authority audit listing each fact crossing the adapter and its sole TME
   owner;
5. a temporary sibling lab outside both repositories that is explicitly
   non-authoritative, unshippable, and unable to satisfy acceptance by itself;
   and
6. exact commands, toolchains, licenses, and platform inputs for the candidate.

The first candidate's sole recorded gameplay input is reserved at
`tests/fixtures/presentation-adoption/identity-proof-observer-frame.json`. It
must be an observed frame produced by the real TME server from the identity-proof
world, not a debug snapshot or a hand-written projection. The recording is
absent at this authorization commit, so P1 is blocked until a separate reviewed
prerequisite change records it, its provenance, and its digest.

At this plan baseline the recording starts from these exact TME-owned inputs:

- `content/lands/identity-proof/world.json` —
  `df8c308b808cba483dfc584e0ee891ea56514939af41091512ee9b74d0574eff`;
- `content/lands/identity-proof/generated/world_template.json` —
  `a2932c57dcb381c5b6dbe18d0820b331955e3874fd8447a3f5a97236c31aad64`;
  and
- `content/lands/identity-proof/simulation_seed.json` —
  `ac3952813d5a06759891932c562c4182a8bcefe3ee9c4776eabf47eab4c76fd6`.

The existing `tests/fixtures/capture/fixture_land_frame.json` is not a
substitute: it is real server output, but it depicts the authoring fixture rather
than the identity-proof settlement. The prerequisite recording may select and
arrange already-authoritative gameplay through ordinary TME inputs; it may not
invent state for presentation. Any source drift is recorded with the new frame
and its candidate evidence rather than hidden behind these baseline digests.

If the target needs authoritative dead-layer material that the current TME
recording cannot supply before S2 exists, stop. The owner may separately release
the already-ruled S2 slice as a required input or narrow the candidate and name
what can no longer be proved. The lab may not invent a dead layer or relabel a
presentation mock as game evidence.

## 4. Phase P1 — browser evidence first

The first candidate uses browser/WebGL because the authorized Nomos baseline
already proves an offline path there. It consumes the recorded TME input through
a TME-owned mapping and produces disposable evidence in the sibling lab.

This is not a rebuild of the browser/DOM board rejected as TME's feel surface in
[settled conclusions](../settled-conclusions.md). It tests Nomos's WebGL
presentation path as quarantined evidence while Godot remains the standing
client and feel surface.

The mapping may rename, select, normalize, or package already-resolved facts. It
may not infer legality, resolve sight, compute movement cost, advance a gameplay
clock, derive life state, or persist game state. Final pixels and local
presentation timing belong to the presenter; gameplay consequences do not.

The candidate must show the representative scene at actual play size and expose
enough diagnostics for a content author to make the second matching asset
without editing compiler or presenter source.

## 5. Phase P2 — second-author evidence

Someone other than the presenter implementation author creates one matching
asset or scene from the approved target packet. The run records:

- elapsed authoring time and iteration count;
- diagnostics and every human intervention;
- every changed file;
- any attempted raw transform or final-pixel instruction, including zero; and
- any compiler or presenter source edit, including zero.

The owner judges the representative scene and the second result at actual play
size. A beautiful first result with an unaffordable or source-editing-dependent
second result fails the production rule.

## 6. Phase P3 — conditional Godot comparison

Godot is tested only if the browser result leaves a named, consequential
uncertainty about feel, accessibility, desktop integration, performance,
deployment, or production cost. The comparison uses the same target digest,
recorded TME input, admitted Nomos artifacts, and authority audit.

Godot does not receive a second rules interpretation, different facts, or a
hand-tuned target. The current Godot client remains the standing client and feel
surface until a final owner disposition and later clean cutover say otherwise.

## 7. Evidence record

Every candidate record binds:

- exact TME and Nomos commits and trees;
- target, recording, schema, dependency, and output digests;
- host, toolchain, presenter, commands, and environment;
- build time, peak disk, artifact size, iteration time, and first-frame time;
- authoring time and cost for the second matching result;
- diagnostics, interventions, changed files, and external dependencies;
- every cross-boundary fact and its sole owner;
- every bypass, duplicated fact, raw transform, and source edit, including zero;
  and
- the owner's visual, platform, cost, and plan verdict.

The initial input is recorded gameplay, not live networking. Authentication,
persistence, live input, and public deployment remain outside the question.

## 8. Upstream-first finding flow

Classify every finding before implementation:

1. existing Nomos-contract defect;
2. reusable missing Nomos capability;
3. reusable presenter/platform concern;
4. TME mapping/profile concern;
5. TME mechanic or content; or
6. duplicate authority or architectural leakage.

For classes 1–3, reduce the observation to an adopter-neutral failing fixture in
Nomos, obtain its required issue and owner authority, land the clean change and
non-author proof upstream, then deliberately update TME's exact pin. No TME name,
content, route, coordinate, palette, mechanic, or governance enters accepted
Nomos code.

Classes 4–5 remain here. Class 6 stops for disposition. TME carries no permanent
Nomos fork, copied implementation, sibling path dependency, compatibility shim,
dual Nomos schema, or dual-path Nomos integration.

## 9. Required proof

A proceed result requires:

1. applicable TME verification on the exact candidate, with every unavailable
   capability named and never represented as passing;
2. exact Nomos proof on the immutable upstream point;
3. an authority audit finding one owner for every cross-boundary fact;
4. public-boundary, provenance, dependency, and licensing checks for anything
   proposed for clean implementation;
5. the target and second matching result passing the owner's actual-play-size
   judgment;
6. an independent rerun of the exact integration proof; and
7. no accepted result depending on the disposable lab after clean work lands in
   the owning repositories.

## 10. Stop conditions

Stop for owner disposition if:

- the target is rejected;
- the mapping or presenter reconstructs gameplay;
- a downstream Nomos fork or copied implementation is required;
- a reusable upstream feature cannot be stated without TME identity or content;
- ordinary content requires raw transforms or final-pixel instructions;
- the second matching result requires compiler or presenter source edits;
- cross-project, predecessor, gaol, private, or unlicensed payload enters TME;
- a deeper rules, server, protocol, or persistence replacement is required;
- browser fails a consequential criterion and no bounded same-input Godot
  comparison can answer it; or
- any required proof is red.

A stop is evidence, not permission to weaken the target or create a framework.

## 11. Final dispositions

The owner records exactly one in the [genesis ledger](genesis-ledger.md):

1. proceed to a clean browser-presentation implementation plan;
2. proceed to a clean Godot consumer of the same admitted Nomos artifacts;
3. return to Nomos for separately authorized narrow upstream work based on a
   minimal adopter-neutral failure; or
4. stop and resume the pre-existing Phase 10 order at S3.

For dispositions 1–3, the owner separately records the new Phase 10 order and
changes a settled implementation-stack or feel-surface conclusion only to the
extent the evidence proved. This experiment is not itself game adoption.

## 12. Non-goals

- no S2–S9 mechanic implementation without a separate release;
- no S10 Phase 10 visual implementation;
- no live-network, authentication, persistence, or deployment rewrite;
- no external product-boundary activation, outside-player admission, or public release;
- no speculative audio, combat, inventory, dialogue, replication, editor, or
  production-asset framework;
- no automatic following of Nomos `main`; and
- no tracked implementation-stack widening without the later owner decision
  required by [settled conclusions](../settled-conclusions.md); and
- no claim that The Signed World applies to TME.
