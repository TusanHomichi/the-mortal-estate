# Backup, restore, RPO, and RTO

<!-- tme-fact-owner:runbook:backup-restore -->

Production uses encrypted off-host pgBackRest storage, continuous WAL archive,
daily incremental, weekly differential, and monthly full schedules. Retain at
least six monthly fulls, four weekly differentials, seven daily recovery
points, and the WAL required for 35 days of point-in-time recovery. The goals
are RPO at most 15 minutes and RTO at most two hours.

Use `bin/backup full|diff|incr` for a bounded verified backup. Before migration,
require both a pgBackRest backup and a custom-format logical dump with a local
SHA-256 receipt. Test repository decryption from the escrowed recovery copy
without printing the key.

For an isolated drill, run `/opt/tme/operations/bin/restore-drill` as the
PostgreSQL service user. It takes the same
`/var/lib/tme/operations.lock` used by deploy, rollback, and destructive
recovery, so those state-changing operations cannot overlap.
It accepts only the canonical `/var/lib/tme/restore-drill` root, starts a
Unix-socket-only cluster, validates migration history and every rules-owned
checkpoint, applies the restore fence, validates again, and records a
structured receipt containing the selected backup label, PostgreSQL system and
database identities, migration-set hash, pre/post checkpoint-set hashes,
pre/post store verification, and the exact one-step restore-fence epoch.
Success removes the exact drill data/socket children before replacing the
receipt. A failure or HUP/INT/TERM at any phase quarantines the newly created
exact data child and leaves the prior receipt unchanged. The production
database stays online and unchanged. Each phase update fsyncs its file and
directory. Commit fsyncs the complete receipt, atomically renames it across the
drill and operations directories, fsyncs both directories, and only then clears
the failure traps; post-rename signal handling accepts only the complete exact
receipt schema and evidence as committed.

For native pre-fence and post-fence failure cleanup proof, run the unmodified
`bin/restore-drill` as its service user and capture its exact shell PID. From a
second root shell, require exactly one direct child whose full command is the
current release's `tme-server store verify` (pre-fence case) or `tme-server
store restore-fence --confirm-restored-database` (post-fence case). Send
`SIGSTOP` to the parent shell, never the database child. Let that one child
finish. For the post-fence case, query only the isolated drill socket and prove
the fence epoch advanced exactly once. Queue `SIGTERM` to the stopped parent,
then `SIGCONT`; this guarantees the shell handles termination before it can
launch the next phase. Require exit 143, a byte-identical prior receipt, one
new exact quarantine child, no drill `pgdata`, and unchanged live PostgreSQL
and The Mortal Estate status. If parent/child identity, direct ancestry, command line, or
isolated socket cannot be proved exactly, skip the injection rather than signal
an ambiguous process.

For destructive recovery:

1. Capture the failure time and exact target recovery timestamp in UTC. Review
   the retained-host impact before proceeding; destructive recovery is never a
   routine staging certification.
2. Set `TME_DESTRUCTIVE_RESTORE_CONFIRM=destroy-disposable-tme-database`
   and run `/opt/tme/operations/bin/recover-production
   latest|UTC-timestamp` as root. It accepts only the literal
   `/var/lib/postgresql/18/main` target, exact
   `postgresql@18-main.service`, `/var/run/postgresql` socket, and port `5432`.
   It closes/proves the admission gate, preserves the failed data directory
   under its canonical parent, and restores into a new exact live directory.
3. The orchestrator starts PostgreSQL privately, verifies the store and grants,
   applies `store restore-fence --confirm-restored-database`, verifies again,
   starts The Mortal Estate, and requires readiness plus fresh authenticated play. The
   fence revokes sessions/tickets and advances character and restore epochs.
4. Compare schema/content/facet identities, logical time, RNG state, receipts,
   ownership, active/historical marks, schedules, and sample checkpoint hashes.
5. Confirm the atomic recovery receipt names the active release and recovery
   target. Admission opens only when that receipt replaces the Caddy marker.

Before the destructive restore fence, the orchestrator journals the exact
release, target, failed-data quarantine path, PostgreSQL system/database
identity, prior receipt digest, and pre-fence epoch in the admission marker.
Its durable early phases cover pre-move, move-pending, failed-data quarantined,
files restored, and PostgreSQL ready. On any failure before a possible fence
attempt, it stops and asserts the exact PostgreSQL unit, quarantines the
restored `18/main` directory read-only, and records a resumable
`pre_fence_quarantined` phase. A retry with the same target restores again.
Once `fence_pending` is durable, failure handling stops the exact unit but
retains the data for epoch reconciliation rather than risking a repeated
fence. After a process or host interruption, the same command resumes only a
matching journal. An observed unchanged epoch permits one fence attempt; an
observed epoch of exactly `before + 1` proves the fence already committed and
skips it. Every identity check also proves the connected server reports the
literal `18/main` data directory, port 5432, and PostgreSQL 18. Every other
identity or epoch fails closed. Verification, readiness, and a fresh
authenticated smoke are rerun before the marker becomes the success receipt.
If the RPO/RTO target is missed, treat it as an incident; never edit mark or
checkpoint rows manually to make a restore appear healthy.
