---
last_updated: 2026-09-05
revision: 3
status: Merged in PR 41 as 9f3f284; dated cutover receipt. Current state is in the genesis checkpoint; visual acceptance remains separate.
public_safe: true
summary: Replace shared gameplay pulses with server-authoritative individual deadlines, alongside the current browser scene pass.
---

# Individual action cooldowns

## Owner direction and scope

The owner explicitly superseded the shared-pulse design on September 5: the
server retains authority, but each action's cooldown starts from that
character's accepted action time. No gameplay process is managed by a shared
pulse. Housekeeping may wake the server; it does not define gameplay deadlines.
The browser move experiment gives each committed move a full three-second
interval, locks out replacement movement during it, and starts a fresh interval
for the next move. The earlier suggestion to defer a move to a later shared
beat was superseded before implementation.

This slice also enlarges the scene's trees and replaces outdoor wall runs with
complete buildings. The separate private test-server deployment is tracked in
[issue #40](https://github.com/TusanHomichi/the-mortal-estate/issues/40).
The browser remains a local feel experiment; it does not acquire authentication
or authoritative wire integration in this slice.

## Required cutover

- Give deterministic rules time sub-action precision. Keep authored duration
  costs, independent actor readiness, and one authoritative simulation clock.
- Advance to individual due deadlines: actor opportunities, spell preparation,
  effects, recovery, and ecology. No periodic global boundary may shorten an
  independently started duration.
- Inject elapsed time from one server clock into the serialized world owner.
  Persist fractional deadlines and preserve remaining cooldowns across recovery;
  disconnecting or duplicate commands cannot reset them.
- Carry precise time and readiness through the wire and retained client. Retire
  shared-pulse presentation and old ambiguous timing shapes atomically.
- Decompose touched files above the repository's size threshold as part of the
  cutover; do not retain duplicate readiness or a second mutable timer ledger.

## Proof required

Two characters acting at different offsets must each receive their own complete
cooldown. Cover action-specific costs, rejection during cooldown, free reads,
replay, reconnect/recovery, same-time deterministic ordering, and delayed
housekeeping. Verify effect and recovery timers without shared-phase clipping.
Compare late versus early browser clicks and the former preview behavior.
Run the relevant verification lanes and live client/server proof. A passing
local browser capture does not prove server behavior.

## Ownership and closeout

`docs/boundary-map.md` owns the replacement timing ruling; server and client
notes own implementation details. This plan records the work and proof, not a
second timing specification. Work starts from main at
`74abe7631ad1e3af23a4c216d5a13504cdf75775`, with the prior scene-fit changes
already uncommitted. After local closeout, the owner authorized committing,
opening a pull request, merging after required checks, and cleaning up the task
branch on September 5.

## Implementation and findings

- Rules time is explicit milliseconds. Current contracts are checkpoint 5,
  snapshot 31, observed snapshot 30, event 41, action context 32, and observer
  projection/frame 8. Scalar time and observer frame 7 are refused.
- Deadline selection derives from owning state. Automatic actions, recovery,
  effects, summons (including defeated summons), ecology, and group expiry run
  when independently due. Restart preserves an already-started absence interval.
- The server checks deadlines every 25 ms and advances through them in order.
  This operational wakeup never defines an action's start or duration.
- The immutable SQL migration set gains a deadline-audit migration. Historical
  `facet_tick` audit rows remain readable; current runtime writes use
  `facet_deadlines`. Missing this database label was caught by the live proof,
  fixed in the owning schema, and rerun successfully.
- The test-corpus Blue Gate fixture now lasts two action units so it remains
  usable after the caster's complete cooldown. Damage-effect tick budgets were
  not extended to preserve old pulse timing. Golden traces were regenerated
  against the current rules.
- Oversized checkpoint, protocol, facet, PostgreSQL, model, and simulation
  rendering owners were separated by responsibility. Domain test fixtures move
  with their owners. The renderer retains an exhaustive event dispatcher.
- Presentation recording now excludes run-local logical timestamps from its
  semantic comparison and records a digest of working source files. A current
  frame and receipt were captured through the real server and shipped client;
  they explicitly disclose uncommitted source, with the existing candidate
  disposition preserved.

## Observed proof

- Rules library: 102 tests pass, including offset players, fractional checkpoint
  recovery, delayed housekeeping, defeated summon expiry, independent recovery,
  and restart presence preservation. The broader rules suite also passed before
  the final proof additions.
- Real TLS server/client proof: three actions at distinct offsets each report
  exactly 3,000 ms from acceptance to readiness, with server-authoritative lock
  release. Capture shows the same action deadline at three increasing fills,
  including an unrelated world update between samples.
- Workspace formatting, build, clippy with warnings denied, and every Rust test
  target pass. Documentation and Python lanes pass. Clean-copy build and tests
  also pass. The private denylist is intentionally absent in that copy; the
  real private-boundary scan passes in the main checkout.
- The deterministic simulation/facet trace repeats exactly with current SHA-256
  `7f8eaf2b76d5bf20771d7b476a4055f694e1ab3b006d1529f15187cf5d25efcb`.
- Browser scene pass 03: 147 unit tests, typecheck, production build, and movement
  proof in Chromium and Firefox pass. Eight final captures cover four views in
  both engines. The existing private preview uses the verified scene 03 release;
  scene 02 is preserved for rollback. The anonymous HTTPS request still receives
  401. Authenticated public-network viewing was not part of the browser proof.

The first-land tree-variety direction belongs to presentation direction. The
private test-server setup belongs to issue 40. Visual acceptance, real land
creation, and browser wire integration remain outside this slice.

### Durable precision and proof selector findings

The PostgreSQL gameplay smoke test caught a surviving whole-unit conversion in
player-kill persistence. The owning columns are now explicitly
`assessed_logical_millis`, with historical values converted once by an immutable
migration. Assessment, absent-killer consequences, and forgiveness use exact
millisecond values throughout. The durable smoke test passed after the cutover.
The integrated town trace now starts restoration from 20 HP rather than 21 HP,
because recovery no longer gains an early shared-boundary tick.

Moving the database fault tests changed their exact Rust filter. The gated
runner now uses the current module path and refuses a selector that discovers
zero tests, preventing an empty run from becoming false evidence.

### Final server verification

All six exact PostgreSQL cases pass on a quiet-machine run: durable gameplay,
fenced restore, deferred absent-killer consequences, assessment replay,
end-to-end restart certification, and database fault certification. Every exact
filter was checked to select a real test. The loaded-machine retry failed on a
five-second HTTP read timeout; the successful rerun kept that timeout unchanged.
The final retained-client lane also passes, including the rule that a completed
cooldown cannot constrain presentation cues for a later action.

## Closeout

Implementation and local proof are complete. The verification runner's Rust,
docs, Python, browser, retained-client, Workbench, boundary, PostgreSQL, and
clean-copy surfaces passed across the final focused runs. The clean-copy lane
proved build/test independence using its declared synthetic denylist; it does
not claim the private scan, which separately passed in the main checkout.

Both capture routes and their shared frame were freshly recorded for observer
contract 8; all 57 capture tests pass on that updated set. The earlier clean-copy
run retained an already-corrected MP recovery expectation; the refreshed copy
passes. No test timeout was widened to obtain a passing result.

The scene 03 preview is live, with scene 02 preserved for rollback. The temporary
proof database was stopped after confirming it had no other clients. No new
long-running game server was deployed. Issue 40 tracks that separate setup.
First-land tree placement follows presentation direction's variety rule.

Delivery: the owner authorized the completed slice for a pull request and merge
after both required checks pass. GitHub records the resulting commit, review,
and merge evidence. No implementation blocker remains. The next product review
is the existing private preview, followed by first-land authoring when the owner
starts that slice.
