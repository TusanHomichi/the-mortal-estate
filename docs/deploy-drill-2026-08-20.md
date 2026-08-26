---
last_updated: 2026-08-20
revision: 1
status: Historical evidence from the 2026-08-20 deployment drill. Not present authority; front matter added at Phase 8 so the subject router can route to it.
public_safe: true
summary: The 2026-08-20 single-host deployment drill, exactly as observed — what was provisioned, what failed, and what the failures cost. History, not a runbook.
routes:
  - deploy/**
---

# Deployment drill, 2026-08-20

Genesis plan Phase 5 requires "a full bootstrap, deploy, rollback, backup, and
restore drill on the renamed identifiers. A green build is not proof that a data
migration worked." This is the record of that drill: what was run, what it
proved, which deployment defects it found, and what proof remains owed on a real
host.

The drill ran in a local disposable container. It never touched the retired
predecessor host, which carries the migration's evidence backups.

## Environment

| Element | Value |
| --- | --- |
| Drill host image | `debian:13` (`ID=debian`, `DEBIAN_VERSION_FULL=13.6`, amd64) plus `systemd`, `systemd-sysv`, `dbus`, `ca-certificates` |
| Init | systemd 257 (257.13-1~deb13u1) as PID 1 |
| Container runtime | rootless podman 5.8.4 on a cgroup v2 host |
| Backup domain | a second container running MinIO with a drill-issued TLS certificate, reachable as `minio.drill.test` |
| Public host name | `tme-drill.test`, resolving to the drill container's own non-loopback address |
| Release source | workspace revision `d228764e7506445a553aa3f825cc69a9c1319936` |

The server binary was built inside a separate `debian:13` container with the
workspace's pinned toolchain, so the release links against Debian 13's glibc
rather than the developer host's.

One caveat on that revision, stated plainly. A second lane was editing the same
working tree while the drill ran. `HEAD` was `d228764` throughout and the drill's
`source-revision` receipt records it, but the tree also carried uncommitted
changes at build time, and several server sources were modified after the build
finished. The binary this drill exercised is therefore not reproducible from that
commit alone. Nothing here depends on the binary's exact contents — the drill
proves the deployment machinery, the migration, and the identifiers — but the
release receipt's revision should not be read as a reproducible build claim, and
a real host proof should be built from a committed, quiet tree.

Drill host invocation:

```sh
podman run -d --name tme-drill-host --network tme-drill \
  --add-host minio.drill.test:<minio-container-address> \
  --systemd=always --privileged --tmpfs /run --tmpfs /run/lock \
  --hostname tme-drill.test -v <scratch>/inbox:/inbox:z \
  localhost/tme-drill-host:13.6
# inside the container, once:
mount --make-shared /run
```

Both container concessions are explained under Named gaps. Every deployment
script ran unmodified from the repository tree except `bin/host-preflight`,
whose single container-impossible assertion was waived in the operations copy
only; the staged release payload carried the pristine tree.

## What the drill ran

Each step used the repository's own entry point. Evidence tails are quoted
verbatim from the run.

### 1. Host preflight and the reviewed package closure

`bin/host-preflight` accepted the container on every assertion it could reach:

```
host-preflight: Debian 13.6 amd64 systemd 257
```

`bin/prepare-packages` verified the PGDG and Caddy signing-key fingerprints
against the live repositories, selected the exact pinned versions, and wrote its
receipts:

```
caddy       2.11.4               amd64
pgbackrest  2.58.0-1.pgdg13+1    amd64
postgresql-18  18.4-1.pgdg13+1   amd64
PACKAGE-VERSIONS: OK
prepare-packages: reviewed package closure written to /var/tmp/tme-packages
```

### 2. Bootstrap

`bin/bootstrap-host` installed the closure with downloads disabled, created the
`tme`, `tme-monitor`, and `tme-deploy` identities, built the owned path tree,
wrote the bootstrap-closed admission marker, installed the operations bundle at
`/opt/tme/operations`, and enabled the units:

```
tme:x:101:103::/var/lib/tme:/usr/sbin/nologin
tme-monitor:x:102:104::/var/lib/tme/alerts:/usr/sbin/nologin
tme-deploy:x:103:105::/nonexistent:/usr/sbin/nologin
-rw-rw----. 1 root postgres /var/lib/tme/operations.lock
{"schema_version":1,"operation":"bootstrap","state":"closed"}
tme-server.service         enabled
tme-restore-drill.timer    enabled
```

### 3. PostgreSQL 18 and the encrypted off-host repository

`bin/configure-postgres` installed the tracked `conf.d` fragment and exact HBA,
created the database, the six least-privilege roles and their SCRAM passwords,
created the encrypted stanza, and proved a real off-host WAL archive:

```
INFO: stanza-create for stanza 'tme' on repo1
INFO: WAL segment 000000010000000000000002 successfully archived to
      '/tme/archive/tme/18-1/0000000100000000/000000010000000000000002-...gz' on repo1
configure-postgres: PostgreSQL roles and encrypted off-host WAL repository ready
```

The re-founded single migration then applied cleanly, followed by the repeatable
owner-level grants:

```
202608190001|t
```

### 4. Release staging and preflight

Two proof accounts were enrolled with `tme-server account create`, the proof
bootstrap was derived from the tracked content, the release bundle was hashed and
staged immutably, and the full preflight passed:

```
stage-release: r1 verified
preflight: ok
```

### 5. Deploy

`bin/deploy r1` closed admission, proved public HTTP 503, took a verified full
pgBackRest backup, wrote a hashed logical dump, ran the migration through a
deploy-owner-only credential projection, reapplied and verified grants, switched
`current`, started the service, proved private readiness, and ran the two-account
authenticated smoke through public TLS routed over loopback:

```
deploy: r1 committed and admission opened
```

The committed receipt:

```json
{
  "schema_version": 2, "operation": "deploy", "state": "committed",
  "release_id": "r1", "previous_release_id": null,
  "source_revision": "d228764e7506445a553aa3f825cc69a9c1319936",
  "database": { "store_verify": true, "grants_verified": true,
                "migration_set_sha256": "c10daf7d673baf6fd5ec490848c6139f4175f8f1943c3d4fc13e5b5e044eb2c3" },
  "admission": { "closed_http_status": 503, "loopback_tls_smoke": true },
  "readiness": { "gameplay_ready": true, "control_api_version": 3, "protocol_minor": 8 },
  "authenticated_smoke": { "profile": "admission", "result": "passed" }
}
```

Route and gate behaviour, observed from a non-loopback source before and after
the commit:

```
before commit: public /health/ready -> 503, public /internal/status -> 404
after  commit: public /health/ready -> 200, public /internal/status -> 404
```

The smoke's own output:

```
production-smoke: trusted public TLS and route boundary passed
production-smoke: authenticated preference toggle, replay, and restore passed
production-smoke: reconnect and two-account session teardown passed
production-smoke: PASS
```

### 6. Backup, mutation, and a point-in-time restore

A verified full backup was taken, the pre-mutation account set recorded, a
timestamp `T0` captured, and a third account created after `T0`:

```
pre-mutation:  proof_one, proof_two
T0 = 2026-08-20T08:34:54Z
post-mutation: drillmutation, proof_one, proof_two
```

`bin/restore` then restored to `T0` into the canonical drill root, and the
isolated cluster was started exactly as `bin/restore-drill` starts it:

```
=== ISOLATED CLUSTER recovered to 2026-08-20T08:34:54Z ===
recovery target reached: t
accounts in restored copy: proof_one, proof_two
drillmutation rows (expect 0): 0
proof accounts (expect 2): 2
```

Production stayed live and unchanged throughout: three accounts, service active,
public readiness 200.

### 7. The fenced restore drill

`bin/restore-drill`, run as the PostgreSQL service user, restored the latest
backup in isolation, verified the store before and after the fence, advanced the
fence epoch exactly once, and committed its receipt:

```json
{
  "schema_version": 2, "operation": "restore-drill", "state": "committed",
  "target": "latest", "backup": { "label": "20260820-083438F" },
  "database": { "system_identifier": "7676025829999374963", "database_oid": "16384",
                "migration_set_sha256": "c10daf7d673baf6fd5ec490848c6139f4175f8f1943c3d4fc13e5b5e044eb2c3" },
  "verification": { "pre_fence_store_verify": true, "post_fence_store_verify": true },
  "fence": { "applied": true, "epoch_before": 0, "epoch_after": 1 }
}
```

The drill's `migration_set_sha256` equals the deploy receipt's, and the live
database's own `restore_fence_epoch` remained `0` — the fence was applied only to
the isolated copy.

### 8. Rollback

A second release was staged and deployed, then rolled back:

```
deploy: r2 committed and admission opened
rollback: r1 committed and admission opened
```

The rollback is proven by the running process, not only by the symlink:

```
/opt/tme/current -> /opt/tme/releases/r1
readlink -f /proc/<MainPID>/exe -> /opt/tme/releases/r1/bin/tme-server
{"operation":"rollback","release_id":"r1","previous_release_id":"r2","state":"committed"}
public /health/ready -> 200
```

### 9. An unplanned failure-path proof

The first `deploy r1` attempt failed at `systemctl start` because of a container
capability limit. The release machinery behaved exactly as designed: it stopped
the candidate, removed the `current` link it had created, left admission closed,
wrote no success receipt, left the credential-projection root empty, and recorded
`"phase": "failure_recovery"` in the durable admission journal. A later deploy
then refused to start until an operator explicitly re-armed the marker. That
refusal, and the recovery it forced, were exercised for real.

## Defects found and fixed

All five were found by running the deployment, not by reading it. Each is fixed
in the tree and rode this drill.

### D1 — the production smoke runner never ported

`tools/run_production_smoke.py` did not come across in the Phase 5 stage-1 port.
It is not optional: `bootstrap-host` fails closed without it
(`operations bundle smoke runner is unavailable`), `stage-release` rejects a
release that lacks it, and `deploy`/`rollback` cannot prove admission without it.
The successor could not complete a bootstrap, let alone a deploy.

Fixed by porting the runner and adapting it to the wire the D4 facet surgery
left behind:

| Predecessor | Successor |
| --- | --- |
| the predecessor's `<name>.v1` subprotocol | `tme.v1` |
| its `__Host-<name>_session` cookie | `__Host-tme_session` |
| its `X-<Name>-Csrf` header | `X-Tme-Csrf` |
| `facet_id`/`facet_revision` in welcome | `world_revision`; no facet on the wire |
| `observed_facet_revision` in commands | `observed_world_revision` |
| facet-directory validation on bootstrap | retired; replaced by an assertion that the retired fields are absent |
| slot-two facet-isolation section | removed — the single-world proof bootstrap has one slot per account |

`tools/validate_production_deploy.py` also did not port. It is not required to
run a deployment and was not reconstructed; see F5.

### D2 — `python3` missing from the reviewed closure

`bin/preflight` calls `require_command python3` and the smoke runner is a Python
program run as `tme-deploy`, but `prepare-packages` never downloaded it. On a
minimal Debian 13 host the deploy lane is unrunnable. Observed directly:

```
--- python3 present? ---
python3 ABSENT
```

Fix, in `bin/prepare-packages`:

```diff
-    jq nftables
+    jq nftables python3
```

### D3 — the offline install demanded packages the closure never carried

`prepare-packages` downloads with `--no-install-recommends`; `bootstrap-host`
installed without it, so apt tried to fetch recommended packages with downloads
disabled and failed:

```
The following additional packages will be installed:
  e2fsprogs ... logrotate ... psmisc sysstat xz-utils
E: Unable to fetch some archives, maybe run apt-get update or try with --fix-missing?
```

This only stays hidden on a host that already carries those packages. The
runbook's own claim is that bootstrap "installs only the reviewed local package
closure", so the two commands must agree. Fix, in `bin/bootstrap-host`:

```diff
-apt-get install -y --no-download "$@"
+apt-get install -y --no-download --no-install-recommends "$@"
```

### D4 — `build-proof-bootstrap` read a content layout that no longer exists

The script hardcoded a `prototypes/` path segment the successor's content tree
does not have:

```
error: proof content input is unavailable:
  /opt/drill/src/content/prototypes/catalogs/prototype_catalog_v6.json
```

The successor keeps those three inputs directly under a content root
(`catalogs/`, `world_templates/`, `simulation_seeds/`). Fixed by dropping the
stale segment from all four paths, so `<source-content>` and `<runtime-content>`
name the directory that directly contains them.

### D5 — the deployment reference never provisions the content boundary denylist

The largest finding. `tme-rules` fails closed when its denylist is absent, by
design, and it is consulted at content load. The workspace's `.cargo/config.toml`
points cargo processes at the tracked synthetic fixture, but a deployed binary
has no cargo environment, and nothing in the deploy tree, the systemd unit, or
the runbooks supplied one:

```
content boundary denylist is missing: /.boundary/banned-terms.txt
  (set TME_BANNED_TERMS_FILE or provide .boundary/banned-terms.txt)
```

Consequence before the fix: every release hash verifies, every migration
applies, and the server still cannot start. No amount of release-integrity
checking catches it.

The denylist is private operator data and must never become a release file, so it
is provisioned the way credentials are — a root-owned file on the host, named by
`TME_BANNED_TERMS_FILE`:

- `config/server.env.example` declares the variable, so `tme-server.service`
  receives it through its existing `EnvironmentFile`.
- `bin/common` adds it to the required-input registry, so every script fails
  closed when it is unset.
- `bin/preflight` validates the file (regular file, one link, root-owned, mode
  `0444` or `0440`, non-empty) and passes it explicitly to `bootstrap verify`.
  The explicit pass is necessary: the shell that sources `server.env` does not
  export it.
- `runbooks/bootstrap.md` documents the input alongside the environment file.

## Findings recorded, not fixed

These are real but are design or content questions rather than mechanical
misses. They are recorded here rather than changed under a drill.

- **F1 — the bootstrap runbook's ordering is unexecutable on a fresh host.**
  Step 10 runs `preflight` before staging a release, but the proof manifest names
  its catalog under `/opt/tme/current`, which does not exist until the first
  deploy commits. The drill proved both halves instead: the first deploy ran with
  the manifest pointing at the staged release path, and the steady-state manifest
  under `/opt/tme/current` was then built, the service restarted on it, and
  `preflight: ok` re-observed. The reference needs one decided answer for the
  first deploy.
- **F2 — `bin/restore` cannot be used standalone against the drill root.** It
  requires `/var/lib/tme/restore-drill/quarantine`, which only `bin/restore-drill`
  creates and `bootstrap-host` does not:
  `error: restore quarantine root is unavailable`.
- **F3 — account enrollment is undocumented.** The runbooks require
  `smoke-username-*` credentials and a bootstrap manifest carrying account UUIDs,
  but never say to run `tme-server account create`, nor that it demands a
  compromised-password list of at least 10,000 unique entries.
- **F4 — bootstrap steps 5 and 6 are in the wrong order.** Step 5 installs the
  pgBackRest secret configuration under `root:postgres`, but the `postgres`
  identity does not exist until step 6 installs PostgreSQL
  (`install: invalid group 'postgres'`).
- **F5 — no tracked release-bundle builder.** `stage-release` requires
  `contract-versions.json`, a one-line `source-revision`, and a complete
  `SHA256SUMS`, and nothing in the successor produces them; the drill composed
  them by hand. `tools/validate_production_deploy.py` did not port either, so
  nothing in the successor validates the deploy bundle.
- **F6 — the deploy/rollback runbook still describes an "isolation" smoke.** The
  D4 facet surgery retired the player-visible facet selector and the two-slot
  proof bootstrap, so that clause no longer describes anything the smoke does.
- **F7 — the deploy tree points at a document the successor does not have.**
  `README.md` and `runbooks/deploy-rollback.md` make rollback conditional on the
  contract versions owned by `docs/internal/current-state.md`, and
  `docs/internal/` does not exist here. The named authority for the rollback gate
  is missing.

## Named gaps

Proof still owed on a real Debian 13 host, with the reason each is impossible or
out of scope in a container.

- **G1 — newest-kernel assertion waived.** A container shares the host kernel and
  has no `/boot` or `/vmlinuz`. Exactly one line of `bin/host-preflight` was
  waived, in the operations copy only:

  ```diff
  -latest_kernel=$(basename "$(readlink -f /vmlinuz)")
  -latest_kernel=${latest_kernel#vmlinuz-}
  +# DRILL WAIVER: a container shares the host kernel and has no /boot or /vmlinuz.
  +latest_kernel=$(uname -r)
  ```

  Every other assertion — Debian identity, the exact 13.6 point release,
  architecture, systemd series, clock synchronisation, free space — ran unmodified
  and passed.
- **G2 — public ACME issuance was not exercised.** A reserved `.test` name cannot
  complete an ACME challenge. The drill used a Caddyfile identical to the tracked
  file except that `tls { issuer acme { profile ... } }` became `tls internal`,
  and installed Caddy's local root into the host trust store. Everything else the
  gate depends on — real TLS, the non-loopback 503 closure, the `/internal/*` 404
  isolation, the log field filter, the reverse proxy, and the loopback-routed
  authenticated smoke — ran against the tracked configuration. Real certificate
  issuance and renewal remain unproven.
- **G3 — the firewall step was skipped entirely.** `bin/install-firewall` was not
  run: nftables default-deny, the operator-CIDR SSH restriction, and the
  second-session proof are host concerns a container cannot represent. This is
  the largest untested surface of the bootstrap runbook.
- **G4 — the off-host failure domain was simulated.** The pgBackRest repository
  was a genuinely separate container with real TLS, real S3 semantics, and real
  `aes-256-cbc` repository encryption, but it shares one physical host with the
  database. Encryption, archiving, retention, restore, and point-in-time recovery
  were proven; failure-domain separation was not.
- **G5 — destructive recovery was not exercised.** `bin/recover-production`, its
  resumable pre-fence/fence journal, and the RPO/RTO objectives are untested here.
  The isolated fenced restore proves the fence mechanism; the destructive path
  proves the production one, and remains owed.
- **G6 — the retained-host upgrade was not exercised.** `bin/upgrade-operations`,
  its Caddy-gate-first sequence, and its refusal to overwrite a different
  installed operations root are untested.
- **G7 — timers and alerting were not exercised.** The backup, alert-check, and
  restore-drill timers were installed and enabled but never fired;
  `bin/alert-check` and the webhook path are untested.
- **G8 — two container concessions were required and are not host behaviour.**
  The image ships Docker's own `/usr/sbin/policy-rc.d`, which `bootstrap-host`
  correctly refuses to work around; it was removed after review, and a real
  Debian host has none. The container also needed rootless `--privileged` plus
  `mount --make-shared /run`: without the former, systemd cannot build the unit's
  mount-namespace sandbox (`Failed to keep CAP_SYS_ADMIN`), and without the
  latter, systemd's `LoadCredential` directory is created in a private namespace
  and silently never appears, so the service cannot read its database credential.
  Both are container artifacts. The unit's full hardening set was verified to
  work once they were applied.

## Conclusion

The successor's server runs on the renamed identifiers, end to end, under its own
deployment reference. Bootstrap, PostgreSQL configuration, the re-founded
migration, release staging, deploy, backup, point-in-time restore, the fenced
restore drill, and rollback all completed against `tme`-named users, groups,
paths, units, schema, roles, database, pgBackRest stanza, wire subprotocol,
session cookie, and CSRF header, with committed receipts at every gate. The
migration was proven by data, not by a build: a restore recovered to a timestamp
before a deliberate mutation returned exactly the pre-mutation account set while
production stayed live, and the isolated fenced restore advanced its epoch once
without touching the live database.

The drill also earned its keep as a defect hunt. Five defects blocked the
deployment outright, and three of them — the missing smoke runner, the missing
`python3`, and the unprovisioned boundary denylist — would each have stopped a
first production bring-up cold while every hash and every test stayed green. The
denylist gap is the one worth remembering: the release-integrity machinery is
thorough enough that it can verify a release perfectly and still hand the
operator a server that cannot start.

Phase 5's stop point — "successor server runs on renamed identifiers with a
completed restore drill" — is met, with the firewall step (G3), destructive
recovery (G5), and real certificate issuance (G2) named as the proof still owed
on a real host.
