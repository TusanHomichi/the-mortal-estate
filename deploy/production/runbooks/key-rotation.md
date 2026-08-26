# Credential and recovery-key rotation

<!-- tme-fact-owner:runbook:key-rotation -->

Maintain independent credentials for the runtime, authentication seam,
migrator, monitor, backup repository, alert webhook, and operator access. Store
reconstructible copies in encrypted escrow outside both server and repository
failure domains. Track only credential names, rotation epochs, and verification
hashes; never commit values or include them in shell arguments or evidence.

For database credentials, create the replacement SCRAM value as PostgreSQL
superuser, write a new root-owned systemd credential file with mode `0400`, and
atomically replace the old file. Restart only the consumer service, prove its
least-privilege queries, then revoke the prior password. Runtime must still be
unable to read `tme.account_credentials`; auth must remain unable to mutate
facets, marks, or content state; monitor must remain read-only.

For pgBackRest repository credentials or cipher passphrase, create a new
repository/rotation generation, verify the new escrow copy can decrypt an
isolated backup, and retain old key material until every retained backup that
needs it has expired. Never rotate by destroying the sole decryptable copy.

For the webhook, replace `webhook-url`, restart the alert timer/service, and
capture one redacted local test delivery before revoking the former endpoint.
For suspected session-token exposure, apply the store restore fence or the
approved global session-revocation operation and require fresh login.

After every rotation, run preflight, service readiness, role-capability checks,
backup check, alert delivery, and the relevant authenticated smoke. Record the
rotation epoch and hashes only.
