---
last_updated: 2026-09-05
revision: 16
status: PR 41 merged; scene 06 source and Godot retirement verified for delivery. Private preview remains scene 06; recorded gates remain open.
public_safe: true
summary: Concise resume checkpoint, current-owner pointers, dated genesis evidence, and open gates.
---

# The genesis ledger

The genesis plan that created this repository was authored and tracked in the
private predecessor repository. On 2026-08-21 the owner froze that repository
as reference only; from that date the ledger lives here, and the predecessor's
copy ends with a pointer to this file.

Commit, issue, and pull-request identifiers below the 2026-08-27 public-source
cut refer to the owner-held private development archive. They are historical
receipts, not Git objects or collaboration records in this public repository.
The tracked contracts and owner rulings here are the public authority.

The plan's phases, in order: 0 charter adoption and inventory; 1 preservation;
2 repository genesis and the license transition; 3 public-boundary machinery;
4 the rules spine and its content; 4W Workbench V0a; 5 server, persistence, and
the deployment reference; 6 client; 6W Workbench V0b; 7 documentation and the
boundary map; 8 tooling and the verification lane split; 9 Workbench V1;
10 the first identity proof; 11 the first external public release. Gates are the
owner's and are numbered by the phase they guard.

## Current checkpoint (2026-09-05)

- **Server:** individual-deadline cutover merged in [PR #41](https://github.com/TusanHomichi/the-mortal-estate/pull/41)
  as `9f3f284` after both CI checks passed. [D5](../boundary-map.md#21-authoritative-individual-deadlines-d5)
  owns timing; the [cutover receipt](2026-09-05-individual-action-cooldowns.md) owns its proof.
- **Private preview:** scene 06 is deployed: walking clips, full movement locks,
  cursor-only feedback, and obstacle detours. Scene 05 is retained for rollback.
  Scene 06 passed 152 browser tests and real movement proof in Chromium and Firefox.
- **Source:** scene 06 routing/cursor changes, the GPU proof launcher, and the
  Godot retirement are carried here. The [code/context audit](2026-09-05-code-and-context-audit.md)
  records cleanup and verification; delivery does not update the private preview.
  Python wire proof retains server coverage. Follow-ups: disconnected logout
  [#42](https://github.com/TusanHomichi/the-mortal-estate/issues/42) and browser
  Workbench capture [#43](https://github.com/TusanHomichi/the-mortal-estate/issues/43).
- **Start browser work:** [browser client](../browser-client.md) maps source,
  current behavior, limitations, and proof. Production wire integration,
  proportional chrome, and distance-matched animation remain unfinished.
  [Presentation direction](../presentation-direction.md) owns the target.
- **Separate stop line:** the presentation-adoption experiment remains before P1;
  remaining identity-proof slices are undispatched. Active browser work does not
  release that pause, accept a production presenter, or close G10/G11.

The phase records below are dated history. Read them only when a task needs
ancestry or an earlier gate decision.

## Phase records

| Phase | Gate | Date | Outcome | What carries it here |
| --- | --- | --- | --- | --- |
| 0 | G0 | 2026-08-19 | **Accepted.** The charter and its seven decision-required rulings (D1–D7) were ruled before any work moved. | The charter's rulings are restated where they bind: `docs/settled-conclusions.md`, `docs/boundary-map.md`, `docs/public-boundary-policy.md`. |
| 1 | G1 | 2026-08-19 | **Confirmed.** A verified, encrypted second copy of the evidence base exists off-host and is restored and compared on a schedule. | Private; nothing in this tree depends on it. |
| 2 | G2 | 2026-08-19 | **Accepted.** Root identity, slug, and the two license texts; fresh history with no predecessor commits attached. | Genesis root `a338a6b`; identity and licenses `7956aba` (`LICENSE` AGPL-3.0, `content/LICENSE` all rights reserved). |
| 3 | G3 | 2026-08-19 | **Accepted.** Four fail-closed boundary checks with mutant-kill receipts, before any payload. | `ab3eae6`; `tools/run_checks.py`, `docs/boundary-checks.md`. |
| 4 | G4, G4X | 2026-08-19 | **Accepted.** The rules spine, the strict protocol, the simulation harness, and the content corpus landed as one unit under the `tme-*` scheme. | `9f6adb5` and the following cutover commits; `crates/tme-rules`, `crates/tme-protocol`, `crates/tme-sim`, `content/test-corpus`. |
| 4W | — | 2026-08-19 | **Shipped.** Workbench V0a: the logical selection bridge over the compiler's projection. | `1f1317b`; `docs/workbench-v0.md`. |
| 5 | G5 | 2026-08-20 | **Complete.** One bounded PostgreSQL-backed authoritative server, one process = one world, a deployment reference and a rehearsed drill. | `29e47d0`, `c8e9eca`, the drill and the karma rulings that followed; `docs/server-notes.md`, `docs/deploy-drill-2026-08-20.md`. |
| 6 | G6 | 2026-08-20 | **Stop point reached.** The client signs in, plays, and signs out against the Phase 5 server from an empty database, with no presentation scaffold. Six conservative product defaults stand pending owner review. | `3373328`, `16d5b1c`; `docs/client-architecture.md`, `docs/client-notes.md`. |
| 6W | — | 2026-08-20 | **Shipped.** Workbench V0b: the capture adapter; the owner points at the logical view or a real client capture and gets the same address. | `45d4870`; `docs/workbench-v0.md`. |
| 7 | G7 | 2026-08-20 | **Stop point reached.** Owners named for death, lineage, succession, aging, ancestry, and the pulse; the authoring workflow has no private-corpus dependency. | `8598f72`; `docs/boundary-map.md`, `docs/agent-workflow.md`, `docs/presentation-direction.md`. |
| 8 | G8 | 2026-08-20 | **Stop point reached.** The verification spine: fast, capture, and full scopes; exit 3 means incomplete; clean-clone proof real; the pulse at the ruled 3.0 s. | `966d8c6`, `e26a371`; `tools/run_verification.py`, `docs/working-root-policy.md`. |
| 9 | G9 | 2026-08-21 | **Stop point reached.** Workbench V1: six truth verbs, deterministic replay, candidate preview, atomic Apply; the promotion ceremony untouched. | `1255e60` (after the CI disk-budget fix `0826a2f`); `docs/workbench-v1.md`. |
| 10 (partial) | G10 open | 2026-08-21; bracketed 2026-08-26; paused 2026-08-29; target selected 2026-08-31 | **Started; S1 and S8 landed; the S3-first remainder is bracketed by one bounded presentation experiment, still paused before P1.** The design packet mapped the charter's ten proof items to ten implementation slices and twelve owner rulings, all ruled (R5 amended to 20 beats / 60 s). Item 9 is split between S9 and S10; item 10's clean-build obligation applies across the slices. **S1:** the served world is authored — the compiler compiles a table of lands with content-driven member sets; the proof settlement (48×32) is compiled, loaded through a required bootstrap manifest that refuses an absent or invalid template by name, and seeded with its living cast; the owner reviewed a render, sent the geography back for a shape pass, accepted v2, and the receipt was re-signed `owner_accepted_at_s1`. **S8:** the pulse made visible from one derivation of the beat — meter, movement across the beat, preparation band, feedback bounded by the beat; the client measures the cadence rather than being told it; the live proof caught a per-frame layout stall before merge. **Current bracket:** P0, the disposable-evidence Nomos point, and the authoritative TME recording are complete. The owner subsequently selected the orthographic 2:1 dimetric target, superseding the P0 packet's camera without resuming the experiment. Fresh current-target evidence and a decision about the recording's absent dead layer are therefore both required before P1. No experiment phase or S2–S9 mechanic slice is dispatched without a new owner direction. | Packet `d2752fd`, rulings `c13a17e`, order ruling PR #20; S8 `5cae537` (PR #21); S1 `8021c9d` merged via PR #19. Each merged implementation commit carries the supervisor's own `--scope full` COMPLETE. The present bracket and exact input identities are recorded in [`2026-08-26-nomos-presentation-adoption-experiment.md`](2026-08-26-nomos-presentation-adoption-experiment.md); the accepted current camera is owned by [presentation direction](../presentation-direction.md#projection-and-surface-ruling). Defects filed: #12, #13, #15, #16, #17, #18; #9 and #10 closed as fixed in Phase 8. |
| Public source cut | — | 2026-08-27 | **Accepted.** An in-place visibility change was refused when independent review found one private-lineage token in reachable commit metadata. The accepted tree was exported without Git or collaboration history into a new parentless public repository; the development repository remains a private read-only archive. | [Public boundary policy](../public-boundary-policy.md); the public root; owner-held local-proof and independent-review cut evidence in the private archive. Verification is rerun from the public root. |

## Current owner pause (2026-08-29; target update 2026-08-31)

The owner paused before presentation-experiment P1 to consider the overall
direction. The
[experiment checkpoint](2026-08-26-nomos-presentation-adoption-experiment.md#current-pre-p1-stop-line-owner-2026-08-29)
owns the exact inputs and current stop line. No experiment phase or
remaining Phase 10 slice resumes without a new owner direction.

This pause is not a final experiment disposition, a Nomos R2 verdict, a TME
adoption decision, or a release of S3. TME's target remains accepted, its
representative micro-scene remains unjudged, and G10 and G11 remain open.

The 2026-08-31 target ruling selects orthographic 2:1 dimetric projection and
ordinary world-up geometry. It does not release P1 or any Phase 10 slice. The
paused experiment's P0 packet carries a superseded camera, so resumption also
requires fresh current-target evidence rather than reinterpretation of that
historical digest.

## Open gates

| Gate | Guards | State |
| --- | --- | --- |
| G10 | Exit of Phase 10 — the first identity proof accepted as met | Open. Slices S2–S7 and S9 remain; S10 remains parked as Phase 10 implementation. The presentation experiment is paused before P1, brackets the S3-first fallback, and cannot satisfy G10. |
| G11 | End of Phase 11 — the first external public release, irreversible | Open. Blocked on G10 and on trademark clearance (charter §15). The completed public-source cut cannot satisfy G11. |

## Standing orders (owner, 2026-08-21; amended 2026-08-31)

1. **Order of the remaining slices:** S3 → S2 → (S4, S5, S7 in parallel) → S6
   → S9. Recorded in the
   [identity-proof packet](2026-08-21-identity-proof-packet.md) §12.
2. **Debt slotted at touch time, before the G11 release:** #2, #4, #6, #11,
   #15, #16, #17, #18.
3. **Bounded resumption before S3:** private-archive issue #24 authorized the target packet and
   [`2026-08-26-nomos-presentation-adoption-experiment.md`](2026-08-26-nomos-presentation-adoption-experiment.md).
   P0 and the authoritative TME recording are complete. The exact Nomos
   decision-0022 point is admitted for disposable evidence only; its rejected
   R2 successor is not admitted, does not displace that baseline, and is not a
   prerequisite this plan added. Browser/WebGL remains the first planned evidence
   candidate and Godot a conditional same-input comparison, but neither has
   begun. S10 remains parked; closing public issue #1 accepted the target, not a
   representative micro-scene or visual implementation.
4. **Order bracket, not replacement:** during the current owner pause, no
   experiment phase or S2–S10 slice is dispatched. The pause is neither a final
   disposition nor an automatic return to S3. Any stop, resumption, or new order
   requires an explicit owner direction.
5. **Dead-layer stop:** the authoritative recording explicitly supplies no dead
   layer, while the accepted production rule requires living/dead
   correspondence. Before P1, the owner must release S2 as an input or narrow
   the candidate. Neither choice has been made, and the lab may not fabricate
   the missing authority.
6. **The predecessor repository is frozen** as private reference. Nothing is
   imported from it; its evidence stays where Phase 1 put it.

## Clean public-source ruling (owner, 2026-08-27)

This ruling supersedes the proposed in-place visibility change recorded one day
earlier. Independent review found a private-lineage token in a reachable commit
message. The owner accepted the review and authorized a clean public-source cut:

- the development repository is preserved as a private read-only archive;
- this repository begins from an allowlisted export of the accepted tree, with
  a parentless root and none of the archive's Git or collaboration objects;
- the frozen predecessor remains private and unchanged; and
- publishing source does not admit outside players or publish a client, service,
  public API, store page, lore/content snapshot, or release artifact, and it does
  not close G10 or G11.
