---
last_updated: 2026-09-05
revision: 1
status: Private development deployment and operational procedure; installed-host proof recorded separately.
public_safe: true
summary: Isolated user services, immutable releases, private access, operator credentials, lifecycle, backup, recovery and cleanup.
---

# Private development server

This Canonical runbook owns operation of `deploy/development/`. The scripts own
configuration validation and commands; [server notes](../../docs/server-notes.md)
owns server and world semantics. This runs private test identities against a
separate development world under D4. It admits no outside players.

The host needs PostgreSQL 18 binaries, nginx, OpenSSL, Python, the pinned Rust
toolchain, and a working systemd user manager. Enable lingering for the operator
if services must survive logout. The installer creates no system user, edits no
host database or nginx configuration, and changes no firewall or DNS entry.
Its three named user services have memory, CPU, and task limits. All listeners
bind IPv4 loopback. `config.example.json` owns the example port assignments.

## Install and access

From a carried, staged checkout with no unstaged source changes:

```bash
python3 deploy/development/manage.py install \
  --configuration deploy/development/config.example.json \
  --denylist /absolute/path/to/the/private/denylist
python3 deploy/development/manage.py status
```

The default installation root is `~/.local/share/tme-development`; `--root`
before the command selects another external root. It is refused inside the
checkout. An existing root or a conflicting service unit is also refused.
Settings, operator credentials, generated test-account passwords, TLS material,
bootstrap bindings and proof receipts live under its private `config/` directory.
The process receives runtime/auth database secrets through systemd credentials.
Neither identity uses the administrator's database role.

The frontend's address is `https://localhost:<https-port>`. From another machine,
forward that same port through SSH:

```bash
ssh -N -L <https-port>:127.0.0.1:<https-port> <operator>@<development-host>
```

Trust `config/tls/ca.pem` in the browser used for the private client. Transfer
only that public certificate to a tunnel client; retain private keys on the host.
Do not bypass certificate validation. The leaf certificate names localhost and
127.0.0.1; `renew-tls` signs a replacement using the same authority and switches
the complete key/certificate pair together. The leaf lasts 30 days and the local
authority one year; replace the authority explicitly before its expiry.

`config/test-accounts.json` contains the two generated private test credentials.
Enrollment exercises the real password validator with a generated synthetic
blocklist; it is a private test provisioning input, not a production enrollment
policy. Application credentials are separate from these operator-owned inputs.

The declared served-world document supplies catalog, profile, compiled geography,
seed and RNG selection. The development seed retains the declared cast and adds
one copied controlled character in the adjacent square. The real Rust bootstrap
validator judges the composed result before service startup. This derivative
belongs only to private development and promotes no authored master.

## Lifecycle and proof

```bash
python3 deploy/development/manage.py start
python3 deploy/development/manage.py stop
python3 deploy/development/manage.py restart
python3 deploy/development/manage.py logs
python3 deploy/development/manage.py renew-tls
python3 deploy/development/manage.py proof --restart
```

`proof` uses the existing trusted-TLS wire adapter against the installed services:
two characters see each other, act at different times, preserve their individual
deadlines through reconnect, and reject another action during cooldown. The
`--restart` variant stops and starts all three services during an action and
checks that its remaining interval survives. Both sessions are revoked afterward.
It also backs up the database and performs an isolated fenced restore drill.
The sanitized result is `config/deployment-proof.json`.

This installed-host proof is owner-invoked because it restarts the persistent
development service. Ordinary verification never starts, stops, or restores this
deployment. Portable tests cover isolation and integrity refusals; the complete
baseline supplies independent scratch-database and browser evidence.

## Releases and rollback

`stage` rebuilds the server from the staged carried source, copies carried content,
and writes an integrity manifest plus the built binary's `contract versions`
output. The release directory is addressed by its Git source-tree identity;
`base_commit` identifies its ancestry and does not mislabel uncommitted staged
changes as a commit. Release files are checked before lifecycle operations.

```bash
python3 deploy/development/manage.py stage
python3 deploy/development/manage.py activate <absolute-release-directory>
```

Activation backs up first, stops the server/frontend, switches `current`
atomically, and requires private readiness. A failed activation restores the
previous pointer and proves it ready. To roll back deliberately, activate the
previous release named in `config/activation.json`.

Storage contracts (checkpoint version and embedded migration checksums) and the
served content digests must match. A storage or world-content migration needs
its own implementation slice. Wire and browser changes travel together in one
release; they are not compatibility adapters for older clients. Retain releases
referenced by `config/bootstrap.json`: its validated content bindings are
immutable, even after an equal-content binary release is activated.

## Backup and recovery

```bash
python3 deploy/development/manage.py backup
python3 deploy/development/manage.py restore-drill <absolute-backup-directory>
python3 deploy/development/manage.py restore <absolute-backup-directory> \
  --replace-development-world
```

Backups are consistent PostgreSQL custom-format dumps, stored outside Git with
their SHA-256 and storage contract. The drill creates its own database, restores
the dump, applies the server's restore fence, verifies integrity and both retained
characters, then drops only that drill database. It never serves a second live
world. A modified dump or incompatible storage contract is refused.

Actual restore replaces this development database. It takes a safety backup and
stops gameplay first, fences and verifies the restored store, then proves the
services ready. On failure it restores the safety backup before reopening.

Take a backup before manual experiments whose state matters. Activation and
restore take one automatically. There is no automatic retention deletion or
off-host durability claim for these private test backups; remove obsolete backup
directories deliberately after retaining the recovery points you need.

## Cleanup

```bash
python3 deploy/development/manage.py uninstall
```

This stops/disables only the three matching user units, removes their unit files,
and retains the installation data. Add `--purge-private-development-data` only
when that private world, its credentials and backups should all be deleted.
An interrupted installation retains its inputs and diagnostics; inspect them,
then uninstall/purge that owned root before attempting a fresh install.

The host-specific installation and health receipt stays outside Git. The dated
[execution receipt](../../docs/plans/2026-09-05-private-play-loop.md) owns observed
verification; GitHub owns delivery state.
