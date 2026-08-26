---
last_updated: 2026-08-27
revision: 3
status: Owner-authorized visual target owner. The grammar and production rule stand; one exact target-authority packet is resumed under private-archive issue #24 before any presenter or production pipeline is selected.
public_safe: true
summary: The visual target, the production rule that gates scale, and the owner-authorized ten-part target packet used to judge the bounded Nomos presentation experiment without preselecting browser, Godot, or another pipeline.
routes:
  - client/presentation/**
  - content/test-corpus/**
---

# Presentation direction

This document owns **what the world should look like** and the rules that govern
getting there. The client's architecture is
[client architecture](client-architecture.md); what is currently implemented is
[client notes](client-notes.md).

Revision 1 added no art decision beyond the charter. Revision 2 records the
owner's bounded target-packet ruling and evidence order. The visual target is
still the owner's to set, and this document is where the owner's version is
recorded — not where an agent manufactures acceptance.

## The target

> **An animated, weathered tactical map rendered in chunky illustrated pixels.**

The target is a **playable world view**: not a cartographic poster, and not a
miniature 3D diorama photographed from above.

### The grammar

- a fixed oblique or orthographic-oblique gameplay view;
- continuous terrain art over authoritative logical cells;
- broad intentional pixel clusters rather than noisy fake-pixel detail;
- limited material and regional palettes with a strong value hierarchy;
- actors, enemies, doors, stairs, bridges, and interactables enlarged for
  gameplay readability;
- structures built from a few clear planes and strong silhouettes;
- selective outlines and contact shadows rather than a uniform black contour
  around everything;
- restrained environmental motion — water, smoke, embers, foliage, weather, idle
  weight shifts, and concise combat effects;
- information-dense UI that shares the world's art language without overwhelming
  the playfield.

### The dead world

The dead world **should not default to generic blue-grey fog and skull
decoration.** It may be uncannily preserved, historically layered, quieter, or
more legible than the living world. Its visual identity should express memory and
unfinished continuity.

That is a direction, not a palette. The specifics are open.

## The production rule

**No visual production system earns scale until one representative gameplay
micro-scene is accepted at actual play size.**

That scene must prove, together and at once:

1. player and enemy readability;
2. terrain continuity;
3. structure scale;
4. interaction anchors;
5. UI framing;
6. living/dead correspondence; and
7. the ability to create a **second matching asset efficiently**.

The seventh is the one that gets skipped and the one that decides whether a
pipeline is real. A method that produces one beautiful scene and cannot produce
the next one at a sane cost has proven a picture, not a production system.

Acceptance is the owner's, at actual play size, on the real playfield. A capture
scaled up for a review is not acceptance.

**Renderer choice, model provider, and generation tool are implementation
details.** The visual grammar and the accepted masters are the authority. A tool
change does not reopen the grammar; the grammar changing is an owner decision.

## The current target-authority packet

**Owner ruling, 2026-08-26:** resume one exact target packet before Phase 10 S3
and before any presenter is allowed to define what the target was. This is the
visual authority for the bounded experiment in
[`plans/2026-08-26-nomos-presentation-adoption-experiment.md`](plans/2026-08-26-nomos-presentation-adoption-experiment.md).
It does not select Nomos, a renderer, a platform, or a production pipeline.

The packet contains exactly:

1. a hero environment frame;
2. a normal gameplay frame at the actual camera distance;
3. actor scale and silhouette references;
4. a combat frame with overlapping actors;
5. one restrained effect;
6. low-light and dead-world treatment;
7. a UI-overlay frame;
8. a material and palette sheet;
9. an animation or motion timing strip; and
10. explicit invariants and explicit non-invariants.

Together, those views must make all seven parts of the production rule above
judgeable. The target is incomplete if it avoids the real playfield, the actual
camera distance, living/dead correspondence, UI framing, or the cost of making
the next matching thing.

The owner records exactly one target verdict: `accepted` or `rejected`.
Rejection ends the experiment before presenter selection. Acceptance approves
the target only.

## Presenter evidence order

The browser/WebGL presenter is tested first because the authorized upstream
already carries that evidence path. This is an economy of evidence, not a
platform or feel-surface ruling. The current Godot client remains the standing
client and feel surface.

A Godot candidate is required only when the browser result leaves a named,
consequential uncertainty about feel, accessibility, desktop integration,
performance, deployment, or production cost. If tested, it consumes the same
recorded TME facts and the same admitted, pinned Nomos artifacts. It does not
receive a second gameplay interpretation.

The owner judges every candidate at actual play size against the accepted
target. A proceed verdict also requires a second matching asset or scene made by
someone other than the presenter implementation author, without compiler or
presenter source edits. The evidence records elapsed authoring time, iterations,
diagnostics, interventions, and changed files.

## The candidate lifecycle

Visual work distinguishes five states, and the cost belongs at the end:

| State | What it is |
| --- | --- |
| disposable experiment | thrown away by default; no ceremony at all |
| review candidate | offered for judgment |
| owner-accepted editable master | the authority for what a thing looks like |
| deterministic runtime export | what the game actually loads |
| promoted, verified asset | provenance sealed, licensing recorded, certified |

**Expensive certification, provenance sealing, and full verification belong at
promotion.** They must not block ordinary taste iteration — moving a fence,
lowering a roof, trying a palette. A pipeline that makes every experiment
expensive produces fewer experiments, and taste is a volume problem.

Provenance and licensing for every promoted asset are mandatory and are owned by
[public boundary policy](public-boundary-policy.md).

## Retired pipelines carry no authority

Earlier presentation experiments — a native 3D presentation scaffold, an
image-to-3D asset pipeline, a painterly repaint chain over generated atlases, and
a multi-stage generation conductor — are **retained privately as lessons and
evidence only.** They receive no automatic authority here, and none of their
outputs is in this tree.

Re-entry for any of their outputs is through the production rule above — an
accepted representative micro-scene at actual play size — and then the promotion
path. **Never through a migration step that copies a file across**, and never
because a previous project got a good result from it once.

This applies to the technique as much as to the artefact. "We already solved this"
is a claim that has to survive the same micro-scene gate as any new proposal.

## What is implemented today

Nothing that is art. The current world view is a **diagnostic lattice**: flat
colours, one rectangle per visible square, markers for addressable things, and a
banner inside the picture saying exactly that.

It exists to satisfy the renderer seam with real targeting and to discharge the
capture obligation — not to look like anything. It is described, with its
deliberate limits, in
[client notes](client-notes.md#gridworldview-the-current-implementation), and it
is **not a starting point for art**. A pixel-native renderer substitutes for it
behind the seam and inherits its obligations
([client architecture](client-architecture.md#the-renderer-seam)).

Colour in the current view carries **no authority**: hues are spread across
whatever terrain ids the frame contains, so a class can change colour between
frames. That is a property of a placeholder, and it is one of the reasons the
placeholder cannot quietly become the look.

## Who decides

The owner is the final authority for taste, visual acceptance, and the accepted
masters. Agents may present evidence, candidates, and recommendations; **they do
not manufacture acceptance through automated consensus**, and an autonomous studio
approving its own art is an explicit project non-goal.

## Open

Deliberately unresolved, and not to be resolved by implementation:

- the owner verdict on the current target packet and representative micro-scene;
- the final pixel grammar, including grid pitch, palette discipline, and
  animation budget;
- the dead world's visual identity beyond the direction above;
- the UI's typography and contrast palette
  ([client architecture](client-architecture.md#input-and-the-accessibility-floor)
  owns the accessibility floor those must clear);
- which production tools and presenter earn a place, which is decided by the
  production rule and the bounded evidence plan, not in advance.
