---
last_updated: 2026-09-08
revision: 2
status: Seven-building town and exterior/water candidates deployed privately; visual and owner-device acceptance remain open.
public_safe: true
summary: Reduced-roster migration, generated art guidance and historical ten-building candidate evidence.
---

# Horseshoe town study

Planning record for the owner-dispatched next layout study. The
[surface brief](2026-09-05-first-land-surface.md#horseshoe-town-arrangement)
owns the arrangement and retained boundaries. This record owns this study's
execution and evidence; it accepts neither geography nor artwork.

## Current dispatch

The owner selected the [current town roster](2026-09-05-first-land-surface.md#current-town-building-roster)
on September 8 after reviewing generated guides and choosing the buildings to
retain. The [guide record](2026-09-08-town-art-guides.md) owns the resulting
art-direction packet. The ten-building blockout and native results below are
historical evidence for that earlier candidate, not proof of the new program.

The next authored cut must remove the retired buildings and interior members,
reclaim their ground, consolidate the selected retail roles at provisions and
retire the associated standalone resident/service instances. It must migrate
land contracts, authored maps, paired passages, seed/catalog references,
reviewed anchors, presentation packets and proof targets together. The current
merchant definitions for the omitted shops still carry the shared provisional
`trail_wares` capability; removing their visible models alone would leave live
services and world members behind. Resolve concrete inventory disposition in
content, preserving the existing balm seller and retained services.

Search the authored land, authoring contracts, browser bindings, native proof
scripts and deployment inputs for the retired member/service identifiers before
making the cut. Explicitly account for any persistent actors or items in removed
members before changing the live world. The normal geographic and artwork gates
remain open; choosing the roster does not accept generated coordinates or models.

## September 8 implementation

The owner instructed implementation after selecting the seven-building roster
and corrected town guidance, then explicitly added the coastal-water issue to
this same pass. The current authored arrangement retains the measured coast,
dock and envelope. It has nine world members (surface, seven interiors and the
bounded dungeon entry), 541 passable surface cells and sixteen directed edges.
The temple/dungeon pair and retained interiors keep their geometry. The independent
geographic record is derived from the explicit arrangement and previously reviewed
interiors before compiler validation; the selected roster does not accept artwork.

The retired shops had only provisional balm stock. The authored bakery/chandler
stock is consolidated at provisions with existing prices and quantities. Deferred
herbalist stock is omitted from a fresh seed. During a persistent cutover, remaining
stock from all retired counters transfers to provisions, including any pawned
items; no existing possession or listing is deleted. No food, candle or herbalist
mechanics are introduced by this consolidation.

The external `town-20260908-r1` packet holds original Blender construction,
source placement, independent geography, the scenic coastal profile and native
captures. Retained lodge artwork remains in use while the other six exteriors
receive purpose-readable silhouette experiments. The forge's defective opaque
prop cluster is removed from its presentation; its authored occupied cells remain
blocked. Water implementation follows the [coastal record](2026-09-07-coastal-water.md).

The live-state audit found all three player characters on retained passable
surface cells. The [offline content cutover](../server-notes.md#offline-content-cutover)
preserves accounts, characters, positions, items, bank/locker contents, logical
time and RNG. A restored-database rehearsal passed two successive hydrations
before the backed-up live cutover. Native images alone do not establish that proof.

### Verification and private activation

The exact isolated snapshot `01404187fbc3a24b70db45d227b4e46b2f7d05c4`
passed the full verification lane in 958.073 seconds, including all seven gated
PostgreSQL certifications, live TLS/WSS authentication and recovery, authoritative
browser capture, and the clean-copy build/test proof. Later documentation and
the live path-control proof are covered separately at closeout; this receipt
does not silently include later edits.

The external packet retains `evidence/geography-audit.json`,
`evidence/expedition-combined.json`, `evidence/native/proof.json`,
`evidence/water/proof.json` and `evidence/full-final.log`. Chromium, Firefox and
WebKit each passed the authoritative service/combat/training/storage loop and
native seven-building circulation. One sequential expedition wrapper was stopped
after its completed Chromium and Firefox reports; WebKit completed separately.
The combined receipt names those three passing reports rather than relabeling
the interrupted wrapper. Final cosmetic water changes have their own native
captures and comparison.

Private release `29a6052f44b5b26c65c9f7208cf189f8c92593a9` was activated after a
fresh backup and stopped-writer checkpoint preparation. Exact state-conservation
checks passed before restart; account/character/world counts remained 2/3/1.
The authority became ready before remote access reopened. Activation and rollback
inputs remain in the private packet's `evidence/activation` directory. No account,
character or world reset was performed. Source work remains uncommitted against
`10a8800869a03ede0441b34a6e054d2fb499cbdb`; no PR or merge was dispatched here.

The final deployed path-control walkthrough passed in Chromium, Firefox and
WebKit: first-click nonmutation, second-click and native double-click commitment,
Escape/right-click cancellation, cooldown locking, reconnect draft clearing,
temple entry/return and sign-out. Each test restored the inspection character's
starting position; credentials were absent from the signed-in URL. The private
`evidence/preview-path/path-controls.json` and captures record those results.
The post-preview audit confirmed the two untouched characters retained their
state apart from normal resource-recovery bookkeeping advancing while the server
ran. Item instances, banks, lockers and account/character/world counts were intact.
The user's Git index was unchanged. The verified source snapshot was archived
before removing the disposable checkout/build and scratch PostgreSQL database.

Artwork acceptance remains open. The six new exterior prototypes need further
facade/material and ground-dressing iteration against the guides; the retained
lodge remains the more detailed model. No central feature has been selected.
Water acceptance and measured channel/night follow-ups stay in issue #48;
the existing restore-count assumption (#49) and temple exit framing (#50)
remain separate findings. The rehearsal used the actual three-character count.

## September 7 bounded treatment

The earlier study rearranged the ten existing service buildings around an open commons, with the
temple at the northern crown and the southern arms opening toward the retained
dock. The bank and lodge form the western civic side; training, provisioning
and other trades occupy the eastern side. Small buildings step toward the
crown rather than forming two mechanically straight rows. The central feature
remains undecided; circulation runs around its reserved space.

Use the existing candidate models to judge placement at the ruled camera.
The first inward-facing rotation trial made side-facing entrances difficult to
read from the straight-on view. The revised study retains readable southern
fronts and varies position and setback. Future architectural detailing must
retain that entrance readability.

The candidate retains the current arrival envelope, water mask and dock exactly.
Ground inside the settlement is edited with the building occupancy and routes;
regrowth along its perimeter remains. Every exterior threshold and return
landing moves with its building. Interiors, their service residents and the
temple-to-dungeon connection retain their current geometry. The
[resident contract](../town-resident-contract.md) continues to own their behavior.

## Candidate and proof

The external packet is `horseshoe-20260907-r1`, based on merged source
`10a8800869a03ede0441b34a6e054d2fb499cbdb`. Its source, modeled assets,
manifest, captures and receipts remain together outside the checkout. The
retained artwork comes from the current temple packet; no asset generation,
download or source import is needed for this arrangement study.

The ordinary Rust candidate path checks the exterior without changing the
promotion receipt or reviewed source anchors:

```bash
cargo run -p tme-authoring -- validate-candidate --land first_expedition \
  --member arrival <candidate-arrival.tmj>
cargo run -p tme-authoring -- project-candidate --land first_expedition \
  --member arrival <candidate-arrival.tmj>
```

The proof must bind those bytes to the presentation floor mask, check the
retained water/dock cells and every entrance/return, and exercise native input
in the shared Chromium, Firefox and WebKit roster. The external feel-scene
walkthrough is a candidate circulation proof, not an authoritative server test.
An overview may use a disclosed inspection zoom; gameplay captures and native
pointer checks use the fixed camera at 768×512.

### Geographic checks

The final candidate master SHA-256 is
`4abe6466d382e65e2e7008a30528609c9aed86c7fa313005ccb83f5dd0d8d01f`.
The final presentation manifest SHA-256 is
`3495497be29759cd2dbf22a59dd0a31999d9535a2dcef3cd27849728c871dca5`.

Both Rust candidate commands passed. The independent comparison found all
972 compiled cell verdicts equal to the presentation floor mask, with 508
connected walkable cells, ten service buildings and their paired exterior
returns. All 186 existing water cells and the dock placement are unchanged.
The other eleven spaces differ only where their exterior return coordinate
moves; the temple/dungeon pair remains exact. These are candidate validation
results, not an owner acceptance receipt.

### Native traversal

Chromium, Firefox and WebKit each passed the continuous dock-to-temple walk,
descent to the dungeon landing and return, all ten service entry/return pairs,
shore refusal and presentation floor-mask checks at 768×512. Hardware rendering
was observed in each engine. No browser errors were recorded. The other nine
service cases declare an exterior start fixture at the relevant return square;
only the temple case claims continuous travel from the dock. The walkthrough
uses native pointer input, with the existing temple-exit edge treatment noted
below. It does not claim server persistence or commerce proof for this candidate.

The external `evidence/native/proof.json` owns those per-engine results;
`evidence/geography-audit.json` binds their manifest to the compiler output.
The overview and play-scale captures are retained in `evidence/views/`.

## Promotion and remaining work

The [authoring gate](../authoring-compiler.md#the-double-anchor) requires the
owner's geographic review before this candidate replaces accepted content.
After that review, migrate authored geography, paired transitions, presentation
binding, reviewed digests, proof targets and deployment content together. Prove
the resulting server world and persistent preview before activation; a local
candidate walkthrough does not establish that result.

Existing architecture remains provisional. The temple still needs exterior
identity beyond its sign (`overview.png` and `temple-approach.png`); the commons
also needs a deliberate short-grass and worn-ground treatment after the layout
review (`commons.png`). These are retained visual findings for the subsequent
exterior art pass. Facade/material work continues under the
[presentation direction](../presentation-direction.md#building-purpose-from-the-street).
Coastal rendering remains in the [water follow-up](2026-09-07-coastal-water.md).
The existing temple exit framing finding (#50) remains separate from exterior
placement; its visible northern tile edge is used for candidate return proof.
The dock capture also exposes an opaque black silhouette in the retained forge
prop cluster. The external workshop-model material/export owns that finding;
its next art pass must verify readable material response and ground contact
from the dock and forge doorway at native play size. It does not change service
occupancy or this candidate's route verdicts. `dock.png` records the defect.
No centrepiece is selected by this study.

The September 7 documentation/boundary lane passed for that earlier study.
That study changed only planning records and owner/index pointers; it made no
preview deployment. The September 8 implementation and proof above supersede
its dispatch status.

## Verification environment finding

The first complete-lane attempt used an isolated `GIT_INDEX_FILE` to include
uncommitted new files without touching the user's index. A capture test creates
a scratch Git repository and inherited that variable, replacing the disposable
index with fixture entries; the next source-tree status command failed. The
user's real index was unaffected. Complete proof must instead run in a separate
checkout of the exact staged snapshot with Git-local environment overrides
removed. The temporary index remains appropriate only for the bounded release
staging operation, which does not create nested Git repositories.

The isolated run also exposed an EV child-process fixture that cleared its
boundary-terms environment and discarded startup errors. The certification now
passes the explicit terms file to the child and retains diagnostics. The focused
EV certification passed against a fresh database after that correction.
