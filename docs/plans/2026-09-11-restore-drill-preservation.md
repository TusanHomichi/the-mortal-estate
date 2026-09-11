---
last_updated: 2026-09-11
revision: 1
status: Scoped. Implementation and the disposable restore proof are not yet done.
public_safe: true
summary: Replace the restore drill's fixed character count with preservation of the backed-up identities and durable state, and prove it against disposable databases.
---

# Restore drill preservation

Issue [#49](https://github.com/TusanHomichi/the-mortal-estate/issues/49) owns the
requirement. This is a Planning document and execution record, not a second owner of
restore, backup or fence behaviour.

## The defect

`deploy/development/operations.py::restore_drill` asserts a fixed shape:

```python
counts = site.sql("SELECT (SELECT count(*) FROM tme.characters),(SELECT count(*) FROM tme.facets)", database)
if counts != "2|1":
    raise RuntimeError("restored database did not retain both characters and one world")
```

The preview supports durable character creation, so a valid backup carrying an
additional player-created character fails this. The check also returns hardcoded
`{"restored_characters": 2, ...}`, so the report cannot describe what was actually
restored.

A count is the wrong question in both directions: it cannot see a substitution that
keeps the count, and it cannot see changed durable state at all.

## Scope

Replace the fixed count with a comparison against the identities and durable state
recorded **with the backup**, and prove it.

1. `backup()` records a snapshot of the backed-up database alongside the dump: every
   account and every character (identity, owning account, slot, display name, actor
   id) and the single facet's durable identity and checkpoint digest.
2. `restore_drill()` restores into a fenced scratch database as it does today, then
   compares the restored projection against that recorded snapshot and fails with a
   precise difference rather than a count mismatch.
3. The drill reports the character identities it preserved, not a fixed number.

## Binding expectations to the right snapshot

Expectations come from the backup receipt written at dump time, never from the live
database. Comparing an older backup against today's world would report drift that the
backup cannot be blamed for, and would hide the case the issue is about: a character
created after the backup must not be expected in it.

## What the fence changes, and what must not

`restore_fence` (`crates/tme-server/src/operator.rs`) intentionally mutates: every
session revoked, unconsumed socket tickets deleted, every character's `control_epoch`
incremented, `store_state` rewritten (cluster identity, database oid, fence epoch,
fence timestamp), and one audit event appended.

The comparison therefore covers character and account identity, slot, display name,
actor id, and the facet's durable identity and checkpoint digest — and deliberately
excludes `control_epoch`, sessions, tickets, `store_state` and audit rows. Fence
changes are asserted separately so an unexpected difference there fails too.

## Proof

- Deterministic coverage with no database: the snapshot projection and the
  comparison's accept/reject behaviour, including a same-count substitution and
  altered durable state.
- Failure isolation: a failing comparison still drops its scratch database.
- A disposable PostgreSQL restore proof: a scratch database migrated and seeded,
  plus at least one character created through the normal flow, backed up, drilled,
  and compared. Restoration stays confined to disposable databases.

## Non-goals

No change to the fence, the backup format's dump, the live preview, its database or
its saved characters. No live restore, migration, activation, character reset or
content cutover. `#60` (shared live-server teardown) stays separate; if a demonstrated
dependency blocks the proof, it is recorded rather than absorbed.

## Open questions to settle in the slice

- Whether the snapshot belongs in `backup.json` or a sibling receipt, and whether the
  backup schema version must move.
- Which existing gated capability runs the disposable proof, and whether it extends
  `tools/run_gated_postgres.py` or needs its own tool.
