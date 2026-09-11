---
last_updated: 2026-09-11
revision: 2
status: Implemented and proven. The drill preserves the backed-up identities and durable state, and the gated scratch-installation proof exercises the real helpers, including a failure of forced cleanup.
public_safe: true
summary: Replace the restore drill's fixed character count with preservation of the backed-up identities and durable state, and prove it against disposable databases.
---

# Restore drill preservation

Issue [#49](https://github.com/TusanHomichi/the-mortal-estate/issues/49) owns the
requirement. This is a Planning document and execution record, not a second owner of
restore, backup or fence behaviour. `deploy/development/README.md` owns operation of
the drill; the excerpts below record the state this slice changed.

## The defect

`deploy/development/operations.py::restore_drill` asserted a fixed shape:

```python
counts = site.sql("SELECT (SELECT count(*) FROM tme.characters),(SELECT count(*) FROM tme.facets)", database)
if counts != "2|1":
    raise RuntimeError("restored database did not retain both characters and one world")
```

The preview supports durable character creation, so a valid backup carrying an
additional player-created character failed this. The check also returned hardcoded
`{"restored_characters": 2, ...}`, so the report could not describe what was actually
restored.

A count is the wrong question in both directions: it cannot see a substitution that
keeps the count, and it cannot see changed durable state at all. The operational
follow-up this closes was recorded in the
[town buildout record](2026-09-06-town-buildout.md#temple-patterns-of-daily-use).

## What is implemented

1. `backup()` records a snapshot of the backed-up database alongside the dump: every
   account and every character (identity, owning account, slot, display name, actor
   id) and the single facet's durable identity, revision, sequence and checkpoint
   digest.
2. `restore_drill()` restores into a fenced scratch database as it does today, then
   compares the restored projection against that recorded snapshot and fails naming
   each lost and gained row rather than reporting a count mismatch.
3. The drill reports the character, account and world identities it preserved, not a
   fixed number.

Expectations come from the backup receipt written at dump time, never from the live
database. Comparing an older backup against today's world would report drift the
backup cannot be blamed for, and would hide the case the issue is about: a character
created after the backup must not be expected in it.

### The dump and the receipt describe one instant

`SnapshotSession` holds one repeatable-read, read-only transaction open, exports its
snapshot, and every expectation read imports it — so the receipt and `pg_dump` see the
same instant. Reading them through separate transactions let a change landing between
the two be recorded as though the dump had contained it: a perfectly good backup was
then reported as having lost a character.

`SnapshotSession.close()` ends the transaction, closes the client's input so `psql`
exits on its own, and returns its cleanup problems rather than raising them. Forced
termination is reserved for recovering from a hang, is reported as a cleanup problem,
and — this slice's last defect — a forced termination that *fails* (a kill that cannot
be delivered, or a client that still will not be reaped) is reported too. `backup()`
publishes its receipt only after a clean shutdown, and a caller already handling its
own failure keeps that original failure alongside the cleanup problems.

### The fence is asserted, not filtered

`restore_fence` (`crates/tme-server/src/operator.rs`) intentionally mutates: every
session revoked, unconsumed socket tickets deleted, every character's `control_epoch`
incremented, `store_state` rewritten (cluster identity, database oid, fence epoch,
fence timestamp), and one audit event appended.

The comparison therefore covers character and account identity, slot, display name,
actor id, and the facet's durable identity and checkpoint digest — and deliberately
excludes `control_epoch`, sessions, tickets, `store_state` and audit rows. Fence
changes are asserted separately: every `control_epoch` advanced by exactly one,
`restore_fence_epoch` advanced by exactly one, and no unrevoked session or unconsumed
ticket survived. An intended change and a lost fact cannot be confused.

A receipt missing a required section, carrying more or fewer than one world or fence
epoch, carrying a non-integer or out-of-range epoch, or naming different characters in
its preserved and epoch sections is refused **before** any scratch database exists. A
pre-snapshot receipt is refused for drilling — it cannot support a preservation claim
— while remaining restorable through `restore` and verifiable.

## How it is proven

### Portable coverage, no database

`tests/test_development_deploy.py` compares canned projections: an unchanged restore is
accepted; a **same-count substitution** (the case the count could not see), altered
durable state, changed ownership and slot, and a lost character named rather than
counted are all rejected. Receipt validation is covered section by section, including
booleans, duplicates and disagreeing identity sets.

Two cases exist specifically because a green suite would otherwise not distinguish
correct code from plausible code:

- **A coordinated concurrent commit.** A PostgreSQL client double answers from two
  instants: the world as the exported snapshot sees it, and the world after a writer
  committed while the backup was running. Reads carrying `SET TRANSACTION SNAPSHOT`
  get the first; any other read gets the second, exactly as PostgreSQL answers. The
  case asserts that every expectation read was pinned, that the receipt equals the
  dump's instant, and that a receipt read outside the snapshot would have been
  reported as a lost character. It was mutation-checked: reading the expectations
  outside the snapshot (the ordering this replaced) fails it.
- **Forced cleanup that fails.** Real child processes whose `kill()` raises, and whose
  second `wait()` expires, must produce reported problems rather than exceptions — at
  `close()` and again through `backup()`, where the original failure must survive.
  Both were mutation-checked against the previous `close()`: all four cases error.

### The scratch-installation proof

`tools/run_restore_drill_proof.py` is the gated `postgres` step `gated.restore_drill`.
It drives the deployment helpers themselves — `backup()`, `restore_drill()` and
`SnapshotSession` — and replaces none of them. It refuses a cluster that already holds
a `tme` database, so an installed preview is out of reach; everything it creates is
dropped again.

It provisions a scratch installation in the deployment's own order (the production
`roles.sql`, a `tme` database owned by `tme_owner`, migrations run as that owner,
`grants.sql`, two generated accounts, and the real `provision.bootstrap` manifest),
starts the real server on it through `tools/live_server_harness.py`, and then:

1. creates one more character **through the control API** — the runtime flow, not SQL
   — walks it one square onto a passable neighbour the server itself reports, and
   reads that saved position from the authoritative frame;
2. runs the real `backup()` and the real `restore_drill()`, which must preserve the
   runtime-created identity and every recorded row, and must leave no drill database
   behind;
3. restores the same dump again with the product's own `pg_restore`,
   `store restore-fence` and `store verify`, serves it with the real server, and reads
   the created character's position from that restored world — the semantic form of a
   claim the byte-level comparison cannot make by itself;
4. starts a second `backup()` and creates a character on the live server **while it
   runs**, coordinated on the exported snapshot the backup holds: the new character
   must be in the live world, absent from both the receipt and the dump, and the drill
   of that backup must still pass;
5. rewrites one dump so the character count is unchanged but an identity is
   substituted and the facet revision moved, and requires the real drill to refuse it
   naming the lost, gained and facet rows — then moves only the receipt's fence
   expectation and requires the drill to refuse that after the fence has run. In both
   cases no scratch database may survive.

### Evidence observed

`python3 tools/run_restore_drill_proof.py --admin-url-file <file>`:

```text
scratch installation: /tmp/tme-restore-drill-q02s3qjs (database tme, socket <scratch cluster socket>)
database: tme (provisioned by the caller)
server: gameplay_ready=True protocol=1.10
runtime character: 01a09293-6f87-75f0-be6d-05e6652f28ec slot 2
runtime character stepped north from (8,34)
observed position: first_expedition/arrival (8,33)
backup 20260911T222534Z-0a0604: 3 characters recorded, drill preserved 3, scratch database dropped
coordinated commit: 01a09293-94a5-7251-9140-581499a358f8 is live and absent from 20260911T222538Z-82ea2b, whose drill still preserved 3 characters
restored copy served: 01a09293-6f87-75f0-be6d-05e6652f28ec still at first_expedition/arrival (8,33)
altered backup refused by name: 01a09293-6f87-75f0-be6d-05e6652f28ec lost, bd0cbd73-078a-47d2-b8b6-e8a1d594aa22 gained, scratch database dropped
misfenced receipt refused after the fence ran, scratch database dropped
TME_RESTORE_DRILL_PROOF_OK
```

The walk is what makes the position oracle say something: `(8,33)` is a square the
character reached by playing, which the world document does not declare and only the
durable checkpoint can explain.

Coordination in stage 4 is with the exported snapshot rather than with a sleep. The
ordering this replaced exposed no such instant, which is exactly why it was wrong: its
unpinned reads answered from whatever had been committed by the time they ran.

## Open questions settled

- **Where the snapshot lives.** In `backup.json`, beside the dump's checksum, and the
  receipt schema version moved to 2. Version 1 receipts stay readable and verifiable;
  `restore_drill` refuses them explicitly instead of guessing a count.
- **Which capability runs the proof.** The existing gated `postgres` capability, as a
  new step in the table (`gated.restore_drill`) beside `gated.postgres`. It needs no
  new tool input: the cluster comes from `--admin-url-file` and the root is temporary.

## Non-goals, unchanged

No change to the fence, the backup format's dump, the installed preview, its database
or its saved characters. No live restore, migration, activation, character reset or
content cutover. `#60` (shared live-server teardown) stays separate; nothing here
depends on it. The private preview was not restarted or reconfigured by this slice.
