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

`restore_drill()` holds the same discipline at its own boundary. It creates exactly one
scratch database, drops it through `release_scratch()`, and when a preservation or
fence failure is already propagating, a cleanup that fails is attached to that failure
rather than allowed to replace it. The message carries the scratch database's exact
name, because a database that survived the drill is something a person has to find and
remove. A drill that verified the restore but could not drop its scratch database is
itself a failure. Every cleanup failure counts, including a timeout.

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

### The owned-installation proof

`tools/run_restore_drill_proof.py` is the gated `postgres-server` step
`gated.restore_drill`. It drives the deployment helpers themselves — `backup()`,
`restore_drill()`, `release_scratch()` and `SnapshotSession` — and replaces none of
them. It takes no cluster argument: it runs its own `initdb` cluster under a temporary
root, on a reserved port, with the socket beside it, and provisions the installation
in the deployment's own order (the production `roles.sql`, a `tme` database owned by
`tme_owner`, migrations run as that owner, `grants.sql`, two generated accounts, and
the real `provision.bootstrap` manifest). The one substitution is the service manager:
the installer runs the cluster under a systemd user unit and this proof starts the
same cluster with `pg_ctl`, because it installs no host services.

That ownership is the isolation boundary, and it is asserted rather than assumed. The
fixed names the deployment uses — `tme`, `tme_owner`, `tme_runtime` — exist only inside
this cluster, so nothing outside the root can be read, renamed, given a password or
dropped. Before anything destructive the proof asks the *running server* where its data
directory is and refuses to continue unless it is the directory under its own root.
There is no cross-cluster cleanup to get wrong: the cluster, its databases, its roles
and every credential in them live under the temporary root and die with it.

The temporary root is removed only once nothing holds it. A launch is remembered
*before* `pg_ctl` is asked to start, because PostgreSQL documents that a timed-out
start can continue in the background and succeed — a failed start command is not
evidence that no server exists — and only a *confirmed* shutdown clears the obligation.
Confirmation is `pg_ctl status`, and only its own two answers count: exit 0 means a
postmaster holds the data directory and exit 3 means none does. Every other outcome —
another exit status, a signal, an executable that cannot be run, a status call that does
not finish within its deadline — is an unresolved inspection, reported with the status
and the tool's own diagnostic, because a check that failed cannot authorize deleting the
resource it failed to inspect. A failed stop leaves the obligation in place, so a later
call tries again. An unconfirmed stop retains the root and reports where it is instead of
submitting the data directory, PID file and logs for deletion; a proof that failed for
its own reason keeps that failure, with the retention attached to it. `--keep` retains
the root without a failure.

After starting the real server through `tools/live_server_harness.py`, it:

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
4. runs a second `backup()` whose coordinated writer is **held until the backup has
   exported its snapshot and started its first pinned read**, so the commit lands
   inside the window the claim depends on rather than beside it: the character must be
   live, absent from both the receipt and the dump, the export must have been open
   before and after the commit, and the drill of that backup must still pass;
5. rewrites one dump so the character count is unchanged but an identity is
   substituted and the facet revision moved, and requires the real drill to refuse it
   naming the lost, gained and facet rows — then moves only the receipt's fence
   expectation and requires the drill to refuse that after the fence has run. Both must
   still drop their scratch database;
6. makes a drill's cleanup fail **for a real reason**: a prepared transaction in the
   drill's own scratch database, which `DROP DATABASE ... WITH (FORCE)` cannot
   terminate. The hold is placed while the drill is blocked on the exclusive lock its
   fence takes, so the ordering is enforced rather than raced. The drill must report
   the preservation failure, the cleanup failure and the database's exact name; the
   proof then releases the hold, removes the leaked database with the deployment's own
   helper, and proves none remains.

### Evidence observed

`python3 tools/run_restore_drill_proof.py`:

```text
scratch installation: /tmp/tme-restore-drill-xyooc662 (database tme, port 38797, socket /tmp/tme-restore-drill-xyooc662/socket)
database: tme (provisioned by the caller)
server: gameplay_ready=True protocol=1.10
runtime character: 01a092f4-6c78-7cc3-aa25-1f8125413c54 slot 2
runtime character stepped north from (8,34)
observed position: first_expedition/arrival (8,33)
backup 20260912T001131Z-521167: 3 characters recorded, drill preserved 3, scratch database dropped
coordinated commit: 01a092f4-8c30-7ef0-9a11-538415056f54 committed inside the exported snapshot and is absent from 20260912T001133Z-42ee11, whose drill still preserved 3 characters
restored copy served: 01a092f4-6c78-7cc3-aa25-1f8125413c54 still at first_expedition/arrival (8,33)
altered backup refused by name: 01a092f4-6c78-7cc3-aa25-1f8125413c54 lost, b405ac08-b76d-410d-a286-00d99e89820c gained, scratch database dropped
misfenced receipt refused after the fence ran, scratch database dropped
cleanup failure reported with its cause: tme_restore_2c0911d04eae survived a prepared transaction and the drill kept the preservation failure
recovered: tme_restore_2c0911d04eae released and dropped
TME_RESTORE_DRILL_PROOF_OK
```

The report records `exporting_transactions: [1, 1]` for stage 4: exactly one exported
snapshot was open before and after the commit, which is what makes the ordering a fact
rather than an assumption. The walk is what makes the position oracle say something:
`(8,33)` is a square the character reached by playing, which the world document does
not declare and only the durable checkpoint can explain.

The lifecycle branches a successful run never reaches are driven through the outer
`proof()` with every collaborator stubbed, and the assertions are about what happens to
the root rather than about a returned string: a confirmed shutdown removes it; a
shutdown that could not be confirmed retains it and the failure names both the problem
and the location; a proof failure with an unresolved shutdown keeps the primary failure
*and* the retention; a proof failure with a confirmed shutdown still removes it; a
failure before any launch removes it, because nothing was started to keep it for; and
`--keep` retains it without a failure. A repeated cleanup after a confirmed stop does
nothing, and a repeated cleanup after a failed stop tries again.

The status classification itself is covered with the real `server_state()`, the real
`close()` and the real outer `proof()` connected, and only the process boundary stubbed,
so what is classified is an exit status rather than a boolean handed in: `0` retains the
root and the obligation, `3` clears both and permits removal, and `1`, `4`, a signal, a
missing executable and a status call that does not finish all leave the shutdown
unconfirmed, retain the root and preserve a primary failure. The status call is asserted
to carry a deadline. Both families were mutation-checked against the lifecycle they
replace: the unresolved-shutdown cases request removal of a root whose server survived,
and reading any status but `0` as stopped fails four cases.

### Canonical checks observed

- `python3 tools/run_verification.py --scope full` → **COMPLETE — every selected step
  ran and passed**, 948s total. The steps that matter here all passed:
  `gated: real backup and restore drill on a scratch installation` (33.1s, run on the
  cluster the proof creates and owns), `gated: PostgreSQL suite, one fresh migrated
  database per test`, `server: trusted TLS sign-in, admission, individual cooldowns,
  reconnect, and logout` (which serves the harness's default, provisioned path against
  real PostgreSQL), `browser: authoritative Workbench capture`, and
  `clean clone: builds and tests with no private root`. The clean-clone step reports
  the boundary check degraded onto the tracked synthetic fixture, which is its
  documented behaviour with no private denylist present; the lane's own
  `boundary: banned-terms` step ran against the real denylist and passed.
- `python3 tools/run_verification.py --scope fast --changed-path …` → **COMPLETE**.
- `python3 -m unittest -q tests.test_development_deploy tests.test_live_server_harness
  tests.test_restore_drill_proof` — 48 + 10 + 33 tests, OK.

The tool also left the cluster as it found it: after the final run, no `tme` database
and no `tme%` role remained.

## Open questions settled

- **Where the snapshot lives.** In `backup.json`, beside the dump's checksum, and the
  receipt schema version moved to 2. Version 1 receipts stay readable and verifiable;
  `restore_drill` refuses them explicitly instead of guessing a count.
- **Which capability runs the proof.** A new `postgres-server` capability, as a new
  step in the table (`gated.restore_drill`) beside `gated.postgres`. The proof needs a
  PostgreSQL *server* installation — `initdb`, `pg_ctl`, `psql`, `pg_dump`,
  `pg_restore` — because it creates its own cluster; the existing `postgres` capability
  means a shared cluster's superuser URL, which this proof deliberately does not use.

## Non-goals, unchanged

No change to the fence, the backup format's dump, the installed preview, its database
or its saved characters. No live restore, migration, activation, character reset or
content cutover. `#60` (shared live-server teardown) stays separate; nothing here
depends on it. The private preview was not restarted or reconfigured by this slice.
