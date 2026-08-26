# Deploy and rollback

<!-- tme-fact-owner:runbook:deploy-rollback -->

Build the release from the reviewed source revision. The release manifest must
hash the server binary, embedded migrations, clean content, deployment bundle,
production smoke runner, `contract-versions.json`, and source-revision receipt.
`stage-release` rejects links and an existing release ID; release directories
are never edited in place.
At operation time, deploy and rollback re-verify every manifest row, require
the manifest to cover every release file exactly once, validate the exact
one-line source revision and contract receipt, and do the same for the active
prior release. When a prior release exists, its structured success receipt
must match its release ID, source, contract digest, and manifest digest before
preflight can lead to any host mutation.

For deployment:

1. Run `/opt/tme/operations/bin/preflight` and verify a recent successful
   restore drill.
2. Run `/opt/tme/operations/bin/deploy <release-id>` as root. It takes the
   shared `/var/lib/tme/operations.lock`, proves
   the root-owned Caddy marker returns public HTTP 503, records the prior
   symlink, makes and verifies a full pgBackRest backup, writes a hashed
   full-database custom-format logical dump including
   `public._sqlx_migrations`, runs the offline migration through a scoped
   deploy-owner-only mode-`0700` credential projection whose files are mode
   `0400`, reapplies and verifies least-privilege grants, and
   atomically switches `current`.
3. The script starts the candidate privately, requires the exact readiness
   contract, rechecks public 503 closure, and runs the bounded two-account
   authenticated command/replay/reconnect/isolation smoke through public TLS
   routed over loopback. The final marker-to-receipt rename is the only success
   transition that opens admission. Its JSON records the measured contract,
   migration-set hash, release-manifest digest, route, readiness, and digest
   of the bounded smoke result. The admission marker is also the durable
   phase journal until that atomic commit.
4. Run the broader PvP certification profile separately when the release
   packet calls for its one-shot state mutation. Inspect bounded journal output
   and prove no request body, cookie, credential, account, character, item,
   mark, message, or IP value entered logs or metric labels.

For rollback, run `/opt/tme/operations/bin/rollback <release-id>`. The script
takes the same lock as deploy, destructive recovery, and restore drill, closes
and proves the gate before changing the active release, and switches only when
the target `contract-versions.json` receipt agrees with the contract versions
owned in code by `crates/tme-rules/src/view/contract_versions.rs` and
`crates/tme-protocol`, and accepted by `../bin/release-operation`. If the requested release fails, the shared failure path
revalidates the current database, returns to the release that was active when
the operation began, and reopens admission only after readiness and the same
authenticated smoke. The prior success receipt stays byte-identical. If
fallback cannot be proven, the service is stopped and admission stays closed.
On a failed first deployment, the candidate service is stopped and a current
link that was switched to the candidate is removed, restoring the original
absence of `current`; the durable closed marker remains for review.
