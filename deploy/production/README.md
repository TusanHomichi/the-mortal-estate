# The Mortal Estate native production reference

<!-- tme-fact-owner:production:overview -->

This is the reviewed single-host Debian 13 reference for The Mortal Estate. Internet
traffic reaches Caddy on ports 80/443, Caddy proxies to `127.0.0.1:8080`, and
the operations listener remains loopback-only on `127.0.0.1:9090`. PostgreSQL
accepts The Mortal Estate identities only over its local Unix socket.

The bundle is deliberately input-driven. On a fully updated host, run
`bin/host-preflight`, then use `bin/prepare-packages` to assemble the reviewed
offline package directory from Debian's signed repositories plus the official
Caddy and PostgreSQL repositories. Copy `config/server.env.example` to a
root-owned deployment input outside Git, provide systemd credentials, and run
`bin/bootstrap-host`. Run the full `bin/preflight` only after PostgreSQL,
pgBackRest, Caddy, and the firewall are configured. For a first release, pass
the candidate server binary to `bin/preflight`; it parses and rules-validates
the bootstrap manifest before `bin/stage-release` or `bin/deploy`. No script
invents the hostname, SSH
operator CIDR, backup failure domain, or alert destination.

Releases are immutable directories below `/opt/tme/releases`; only the
`/opt/tme/current` symlink changes. A release manifest must cover the server
binary, migrations, content, deployment files, strict contract-version
receipt, and one-line full source Git revision receipt.
Every release operation verifies the candidate and active prior manifest,
source revision, and contract again; an active prior release also requires a
matching structured success receipt before admission or service state changes.
Rollback is permitted only when the prior manifest's exact release gate agrees
with the contract versions the built release declares — owned in code by
`crates/tme-rules/src/view/contract_versions.rs` and `crates/tme-protocol`.
The enforcing authority remains `bin/stage-release` plus
`bin/release-operation` and the release's `contract-versions.json` receipt.

The root-owned `/var/lib/tme/admission/closed` marker rejects non-loopback
admission at Caddy. Deploy, rollback, and destructive recovery keep that marker
present through private readiness and a loopback-routed public-TLS authenticated
smoke. The complete success JSON replaces the prior receipt in the same atomic
rename that removes the marker; a successful fallback reopens admission without
changing the prior receipt. Until commit, that marker is an atomic phase and
recovery journal. The receipt includes the immutable manifest and sanitized
smoke-result digests. On an already provisioned host, use
`bin/upgrade-operations`, not the fresh-host bootstrap, to install this gate and
its identities/roots while preserving the current release and player data.

Host operations are installed independently at `/opt/tme/operations`.
Backup, restore-drill, alert, deploy, rollback, and recovery entry points must
come from that root, never from `/opt/tme/current`; retaining an older current
gameplay release therefore cannot route an EV operation through older scripts.
Deploy, rollback, destructive recovery, and the isolated restore drill share
the root-owned `/var/lib/tme/operations.lock` (`root:postgres`, mode `0660`).
The first retained-host upgrade installs the operations root atomically. If a
different operations root already exists, the installer stops for an explicit
reviewed replacement instead of overwriting it.

Recovery objectives are RPO <= 15 minutes and RTO <= 2 hours. Production uses
encrypted off-host pgBackRest storage and continuous WAL archiving. Follow the
tracked runbooks; never admit gameplay after a migration or restore until the
full smoke/restore fence succeeds.
