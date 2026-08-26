# Bootstrap the native host

<!-- tme-fact-owner:runbook:bootstrap -->

Use only a fresh disposable Debian 13 amd64 host for the EU proof. Confirm the
hostname, DNS, SSH operator CIDR, and off-host backup failure domain before any
public listener is enabled. Keep gameplay admission closed throughout setup.

1. Apply all current Debian 13 security and point-release updates, reboot into
   the newest installed kernel, and prove that no upgrade remains. Run
   `bin/host-preflight`; it reads the point release from Debian's
   `DEBIAN_VERSION_FULL` field (with `/etc/debian_version` as the native
   fallback), not the major-only `VERSION_ID` field.
2. Run `bin/prepare-packages` as root. It verifies the official Caddy and PGDG
   signing-key fingerprints, selects the exact Caddy 2.11.4, PostgreSQL 18.4,
   and pgBackRest 2.58.0 packages, downloads their not-already-installed
   dependency closure plus required operator packages, and writes a reviewed
   `PACKAGE-VERSIONS` receipt and `SHA256SUMS` under
   `/var/tmp/tme-packages`.
3. Copy `config/server.env.example` to `/etc/tme/server.env`, replace every
   `REQUIRED` value, set mode `0640`, and keep it exactly `root:root`. It must
   be a canonical regular file with one link (mode `0400`, `0600`, or `0640`),
   not a symlink. The file contains
   topology only, never credentials. Keep `TME_ACME_PROFILE=shortlived` when
   the public host is an IP address; current Let's Encrypt IP certificates use
   that roughly six-day profile.
   Install the private content boundary denylist at the path
   `TME_BANNED_TERMS_FILE` names, root-owned, mode `0444` or `0440`, one link,
   never empty. The denylist is operator data and is never a release file: the
   server fails closed at content load without it, so a host that lacks it
   cannot start a release even though every release hash verifies.
4. Create root-owned credential files named `database-url`,
   `auth-database-url`, `migrator-database-url`, `migrator-pgpass`,
   `monitor-pgpass`, `smoke-username-one`, `smoke-password-one`,
   `smoke-username-two`, `smoke-password-two`, and `webhook-url` under
   `/etc/tme/credentials`. Use the exact `0400` or `0600` mode in
   `config/credential-formats.txt`; never pass their values as command arguments. Follow
   `config/credential-formats.txt` exactly: SQLx URLs use the percent-encoded
   Unix-socket host `%2Fvar%2Frun%2Fpostgresql`, while each libpq password file
   uses `localhost` because PostgreSQL matches that value for local Unix-socket
   connections. Each `smoke-username-*` file contains the account's canonical
   3-32 character lowercase username, never its UUID, account ID, character ID,
   display label, or email address.
5. Put the pgBackRest cipher passphrase and repository credentials in the
   native pgBackRest secret configuration. Verify their reconstructible copies
   in encrypted escrow outside both the VPS and repository failure domains.
6. Set `TME_BOOTSTRAP_CONFIRM=fresh-disposable-debian-13` and run
   `bin/bootstrap-host` as root. It re-runs `bin/host-preflight`, verifies the
   package receipts, installs only the reviewed local package closure with
   network downloads disabled, and prevents package maintainer scripts from
   starting public or database services before their tracked configuration is
   installed.
7. Put the five independently generated hexadecimal database-role passwords
   in the exact root-owned, mode-`0600` input described by
   `bin/configure-postgres`. Set
   `TME_POSTGRES_CONFIRM=configure-fresh-postgresql-18` and run that script.
   It installs the tracked Debian PostgreSQL `conf.d` fragment and exact HBA,
   creates the database and least-privilege roles, sets their SCRAM passwords,
   creates the encrypted pgBackRest stanza, and proves an off-host WAL archive.
   Remove the transient password input after its encrypted escrow is verified.
   The migrator URL must use `options=-c role=tme_owner`.
8. Run the first migration, then apply the repeatable owner-level `grants.sql`
   through the migrator's membership in `tme_owner`. Ordinary deploys repeat
   only that final grants phase; they cannot administer roles. Run a full
   pgBackRest backup and verify it before public admission.
9. Install a firewall default-deny policy: permit SSH only from the recorded
   operator CIDR and permit TCP 80/443. Do not expose PostgreSQL, 8080, 9090,
   or Caddy's admin endpoint. Set
   `TME_FIREWALL_CONFIRM=install-default-deny` and run
   `bin/install-firewall`; keep the initiating SSH session open until a second
   key-authenticated connection succeeds through the installed ruleset.
10. Use `bin/build-proof-bootstrap` to derive the two-account single-world proof
   seed and manifest from the clean tracked alignment/social/law content. Run
   the full `bin/preflight <candidate-server-binary>` so the actual server
   parses and rules-validates that manifest, stage a hashed immutable release,
   run `bin/preflight <staged-server-binary>` again, and follow the
   deploy runbook. Confirm Caddy denies `/internal/*` externally while the
   loopback status and metrics endpoints remain available to the operator.

Record package hashes, configuration hashes, firewall output, role capability
queries, and redacted service status. Do not record credential values.

`bootstrap-host` remains fresh-host-only and intentionally retains its exact
`fresh-disposable-debian-13` confirmation. To harden an existing retained host,
stage/review this bundle, provide the smoke credentials, set
`TME_OPERATIONS_UPGRADE_CONFIRM=preserve-existing-tme-state`, and run
`bin/upgrade-operations`. It validates and reloads the new Caddy gate while the
existing service remains intact, proves external 503 closure, then stops
gameplay only while installing units and validating the unchanged database. It
reopens only after readiness and authenticated smoke; a pre-gate failure
restores the prior Caddy file and leaves the service running.

The retained-host upgrade atomically installs this reviewed bundle at
`/opt/tme/operations` and rewrites operational systemd units to that fixed
root; it does not change `/opt/tme/current`. It also creates the shared
`/var/lib/tme/operations.lock` as `root:postgres` mode `0660`. Afterward run
deploy, rollback, restore-drill, and destructive-recovery commands only from
`/opt/tme/operations/bin`. The installer intentionally refuses to overwrite
a different existing `/opt/tme/operations`; replacing an installed
operations bundle requires a separate reviewed procedure and hash comparison.

If a first retained-host upgrade fails after the new Caddy gate activates, do
not delete `/var/lib/tme/admission/closed`, repoint `current`, fabricate a
success receipt, or run an operation from the gameplay release. Keep public
admission closed and prove the selected release, private readiness, database
identity, services, marker bytes, receipt bytes or absence, and empty
credential-projection root before recovery. Freeze every operations timer and
its oneshot service, preserving their prior enabled/active state. If the exact
reviewed bundle was installed before failure, stage its reviewed replacement
as a root-owned sibling of `/opt/tme/operations`; require only regular files
and directories, exact `0755` directories and `bin/*`, `0644` other files,
one link per file, full byte/hash equality with the reviewed source, the same
filesystem as `/opt/tme`, and a successful smoke-runner `--help` probe as
`tme-deploy`. Under both operation locks, rename the old root to a retained
unique backup and the verified sibling into place with a rollback trap and
parent-directory `fsync` after each rename.

Only after the bundle swap and all prior invariants pass may an operator rearm
the still-closed first-upgrade journal: atomically replace, never remove, the
failed marker with the exact root-owned mode-`0644` bootstrap-closed JSON
marker using a temporary on the admission filesystem plus file and directory
`fsync`. Release the locks and rerun the installed
`/opt/tme/operations/bin/upgrade-operations` with the literal confirmation.
It snapshots a pre-existing closed marker and restores it on any failure before
the new gate proves HTTP `503`; later failures retain the owned operation
marker. Restore prior timer state only after the schema-2 success receipt,
selected release, private/public authenticated smoke, route isolation, and
absent marker all validate. On any ambiguity, leave admission closed and stop
for a fresh review.
