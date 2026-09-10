---
last_updated: 2026-09-09
revision: 1
status: Latest merged playable build activated privately; preserved-state rehearsal and three-browser public-origin proof passed.
public_safe: true
summary: Pixel preview activation, one-square preparatory movement, exact checkpoint preservation and native browser evidence.
---

# Pixel preview refresh

This Planning and Execution receipt records the first activation under the
[standing latest-build ruling](../server-notes.md#private-development-deployment).
That server contract owns authorization; the [development runbook](../../deploy/development/README.md)
owns lifecycle operations. The owner requested that the preview always run the
latest build after PR #52 merged the accumulated pixel work.

The activated source is merge commit `5d01572b10741a89dc379914b146a39157eb7aaf`,
tree/release `71137f1556ecb10c07505420d2f6c254e0260312`. The previous release was
`6d1e66d49f18437c10a523d6f484d153ce3bc051`. The artwork manifest matches the
browser receipt `4bbadd5a0fa1b2f900b6eaf3001f58bd21b85a878ec0fae458480d3a89634928`.
The public preview now serves the pixel town and temple with GPU composition.
This adds no outside enrollment or artwork-master acceptance.

## Saved-world preparation and cutover

The old town checkpoint contained three characters, two accounts and one world.
One created character stood at arrival `(6,8)`, now a trainer-building footprint.
The existing migration correctly refused that checkpoint. After a backup, a
normal authenticated `move_path` command moved that character north to `(6,7)`,
which is passable in both layouts. The command completed its normal cooldown.
No offline relocation, seed reset or migration bypass was introduced.

A fresh backup was restored into a disposable database. The existing explicit
content migration used the old/new definition digests with no NPC retirement or
merchant merge. Every checkpoint field except content identity compared equal
before restart. The new release hydrated the restored world twice, preserving
the three-character/two-account/one-world counts. The rehearsal database was
dropped afterward; it was never exposed as a second playable world.

Activation took another backup, stopped frontend and server writers, prepared
the newest canonical checkpoint and installed it with a digest compare-and-swap.
Matching bootstrap, seed-source and artwork bindings changed together with the
release pointer. The authority became ready before the frontend reopened.
The previous release, saved configuration and checkpoint remain available for
rollback. All mutable checkpoint fields compared equal at installation, and the
account/character/world counts remained unchanged.

The preparation lesson is to retain the database's canonical checkpoint bytes;
pretty-printing a decoded checkpoint makes it an invalid migration input.
Diagnostic JSON and canonical migration artifacts serve different purposes.

## Verification

The deployed code passed the full local lane in 807.490 seconds and both required
PR #52 checks; its final README/whitespace cleanup passed the focused lane.
The [GPU receipt](2026-09-09-pixel-gpu-composition.md) owns rendering, temple,
service, exterior and crowd proof. This activation then verified the actual
public origin in Chromium, Firefox and WebKit:

- Root and direct index select pixel art and WebGL; served manifest digest matches.
- Real UI sign-in, native double-click movement and return, reconnect and sign-out pass.
- Each movement proof restores its starting position; 1280×800 and 1920×1080 use 2x/3x sampling.
- No page errors; the authority reports ready after activation and proof.

The external `preview-current-20260909-r1` packet retains backup references,
canonical checkpoints, migration plan, rehearsal and activation receipts,
three-browser reports and screenshots. A deployed 1280×800 capture was opened
for visual inspection. Physical Steam Deck/Tauri proof and broader character-art
assignment remain with the existing browser/artwork owners.
