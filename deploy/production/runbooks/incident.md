# Incident response

<!-- tme-fact-owner:runbook:incident -->

Keep the public service closed whenever private readiness is false, a durable
commit fails, a facet task panics, the restore fence is incomplete, or the
active release/schema pair is unproven. Preserve journals, release manifests,
database/backup status, and alert state before restarting anything.

The alert runner uses these runbook keys:

- `incident`: readiness for two minutes, restart loop, mailbox pressure,
  filesystem pressure, Caddy/TLS failure, certificate expiry, facet panic, or
  store commit failure. Stop admission, inspect loopback status/metrics and
  bounded journals, then decide restart, rollback, or restore.
- `backup-restore`: WAL age/failure/backlog, stale/failed backup, or stale
  restore drill. Keep migrations closed, inspect `pg_stat_archiver` and
  `pgbackrest info/check`, repair archive delivery, and prove a new isolated
  restore before clearing the incident.

Alerts contain only service, host role, severity, condition, first/last seen,
and runbook key. Delivery is deduplicated for four hours per active condition.
Do not add IDs, addresses, payloads, credentials, or chat/page text while
diagnosing. Rotate a credential immediately if logs or process inspection show
it, then follow the key-rotation runbook and invalidate affected sessions.

An alert is cleared only after its underlying probe is healthy and the operator
has recorded the active release, schema version, recovery point, and redacted
verification result. A restart alone is not resolution evidence.
