---
last_updated: 2026-09-05
revision: 30
status: Live navigation index including the September 5 timing ruling; detailed rulings remain in their sole owners.
public_safe: true
summary: Short lookup index for settled architecture, workflow, presentation decisions, and previously diagnosed traps.
always: true
---

# Settled conclusions

Read the row, then its owner before changing the fact. This index prevents
re-deriving closed decisions; it is not a second specification. A target listed
here is not necessarily implemented: [client notes](client-notes.md#current-implementation)
owns that distinction for the clients.

Exact gameplay mechanics, names, values, timings, penalties, and routes remain
subject to [D2](boundary-map.md#what-an-authored-seam-does-and-does-not-settle).
Presentation rows route to recorded owner direction; they do not settle
experimental gameplay tuning.

## Closed

| Topic | Lookup cue | Owner |
| --- | --- | --- |
| Implementation stack | Fixed stack; widening needs a decision. | [AGENTS.md](../AGENTS.md#operating-rules) |
| Gameplay authority | One reusable rules boundary. | [boundary map](boundary-map.md#the-boundaries) |
| Client shape and baseline | Browser first; Godot retained and cold. | [client architecture](client-architecture.md#the-web-client) |
| Browser and desktop proof | Two browser engines; Tauri targets need their own packaged-webview proof. | [client architecture](client-architecture.md#the-web-client) |
| Live characters | Live rigs and modular equipment; material treatment remains candidate. | [presentation direction](presentation-direction.md#live-characters) |
| Individual movement cooldown | Every committed preview move gets a full interval and locks competing movement. | [client notes](client-notes.md#the-web-feel-scenes-walk-experiment) |
| World projection and standing geometry | Accepted camera and world-up construction. | [presentation direction](presentation-direction.md#projection-and-surface-ruling) |
| Tile and building assembly | Shared cell ruler, dedicated interiors, exterior roofs. | [presentation direction](presentation-direction.md#tile-assembly-ruling) |
| Spaces and portals | Current browser experiment and its limits. | [client notes](client-notes.md#the-web-feel-scenes-walk-experiment) |
| Structure, cards, and character sources | Construction and source/output split. | [presentation direction](presentation-direction.md#structure-and-cards) |
| Viewport and relative scale | Scale evidence; later display ruling also applies. | [presentation direction](presentation-direction.md#relative-scale-ruling) |
| Interior camera | Space-owned focus indoors. | [presentation direction](presentation-direction.md#interior-camera) |
| Ground-contact anchoring | Visual extent never defines occupancy. | [presentation direction](presentation-direction.md#tile-assembly-ruling) |
| Movement and readiness | Presentation direction; experiment tuning remains reopened. | [presentation direction](presentation-direction.md#movement-and-readiness) |
| Chrome and actions | Current interface target; earlier layouts superseded. | [presentation direction](presentation-direction.md#chrome-and-actions) |
| Chrome layout | Accepted layout; assets remain candidates. | [presentation direction](presentation-direction.md#accepted-chrome-layout) |
| Proportional scaling | Ruled cell count and ultrawide treatment; assets remain open. | [presentation direction](presentation-direction.md#proportional-scaling) |
| One world | D4: no player-selectable divergent histories. | [server notes](server-notes.md#the-world-instance-and-what-it-is-for) |
| Individual authoritative deadlines | September 5 replaces the shared-pulse ruling. | [boundary map](boundary-map.md#21-authoritative-individual-deadlines-d5) |
| Presenting cooldowns | Godot uses the observer’s precise deadline and frame-only readiness. | [client notes](client-notes.md#individual-cooldowns-made-visible) |
| Content contracts | Authored facts have one owner. | [boundary map](boundary-map.md#11-the-authoredruntime-contract-seam) |
| Content validation | Rust owns gameplay-semantic validation. | [boundary map](boundary-map.md#13-rust-is-the-sole-gameplay-semantic-validator-and-it-fails-closed) |
| Authoring and member counts | Compiler owns land semantics; members are content. | [authoring compiler](authoring-compiler.md#what-it-compiles) |
| Served world | Content declaration and deployment bootstrap are separate. | [server notes](server-notes.md#which-world-the-one-process-serves) |
| Wire corpus | One shared fixture corpus. | [client notes](client-notes.md#the-wire-fixture-corpus-is-read-not-copied) |
| Blocking checks | P9 mutant qualification. | [boundary checks](boundary-checks.md#qualification) |
| Private denylist | Provisioning, worktrees, and fail-closed behavior. | [boundary checks](boundary-checks.md#the-private-terms-convention) |
| Public source | Source publication is separate from product release. | [public boundary policy](public-boundary-policy.md#the-clean-public-successor-and-the-two-publication-cuts) |
| Two enforcement points | Repository scan and content-load validation. | [boundary checks](boundary-checks.md#the-second-enforcement-point-the-content-validator) |
| Internal migrations | Atomic cutover; no compatibility adapters. | [agent workflow](agent-workflow.md#no-compatibility-adapters) |
| Proof method | Exercise and identify the real path. | [agent workflow](agent-workflow.md#verification) |
| Retired presentation pipelines | Re-entry requires the accepted evidence gate. | [presentation direction](presentation-direction.md#retired-pipelines-carry-no-authority) |
| Verification source and lanes | Runner owns steps; inspect its resolved plan. | [verification](verification.md#the-four-lanes) |
| Unavailable proof | Missing capability cannot become a pass. | [verification](verification.md#what-the-exit-code-means) |
| Working roots | Disposable files, retention, and promotion. | [working-root policy](working-root-policy.md) |
| Spec starting point | Charter, reopened decisions, owners, proof, design. | [agent workflow](agent-workflow.md#where-authoring-a-gameplay-spec-starts) |

## Traps already paid for

The linked owner carries the diagnosis and proof.

- [Length gates making context checks unreachable](server-notes.md#the-defect-this-policy-was-written-against)
- [Godot relative fixture paths reading the wrong directory](client-notes.md#the-wire-fixture-corpus-is-read-not-copied)
- [Headless Godot producing no viewport image](client-notes.md#captureemitter-the-presenter-obligation-discharged)
- [Gated PostgreSQL tests sharing a database](server-notes.md#one-fresh-migrated-database-per-gated-test)
- [Ignored tests with no runner inventory](server-notes.md#the-inventory-fails-closed)
- [Direct Rust binary launches bypassing cargo configuration](agent-workflow.md#prove-it-in-the-environment-the-product-configures)
- [Verification tables naming nonexistent targets](agent-workflow.md#the-single-source-of-truth-may-not-name-something-that-is-not-there)
- [Dotted identifiers mistaken for hostnames](boundary-checks.md#hostname-tier-3-and-identifier-vocabulary)
- [Background services dying with their launch session](verification.md#process-lifetime)

## Reopening one

A row here is closed, not permanent. Reopening one takes:

1. **New evidence** — a measurement, a failure, or a requirement that did not
   exist when the row was written. "A different approach also exists" is not new
   evidence.
2. **The owner's decision**, where the row cites an owner ruling. Rows citing D4
   or D5 cannot be reopened by an implementer at all.
3. **The migration**, in the same slice. Reopening a conclusion means changing
   every caller, fixture, test, and golden that depends on it — the
   no-half-migration rule applies to decisions as much as to code.
4. **The record.** The row is updated or removed here, with what replaced it. A
   silently stale row is worse than no list, because the next agent trusts it.

## Adding one

Add a row when a slice closes a question that would otherwise be re-argued: give
a short lookup cue and link the exact owning section. Keep values, sequences,
implementation details, and reasoning in that owner. **Do not duplicate them here** — this is an index, and an index that grows its own
explanations becomes a second copy of the documents it points at.
