"""Observed health, fenced restore drills, and atomic private release activation."""
from __future__ import annotations

import json
import os
import secrets
import ssl
import time
import urllib.error
import urllib.request
from pathlib import Path

from common import UNITS, digest, document

#: The durable state a restore must reproduce, read from whichever database it is
#: taken from. Rows are emitted as JSON so a separator appearing inside a display
#: name cannot be mistaken for a column boundary.
#:
#: `control_epoch` is deliberately NOT part of this projection. The restore fence
#: increments it on purpose, so it belongs to the fence expectations below rather
#: than to the state that preservation is measured against.
SNAPSHOT_QUERIES = {
    "accounts": "SELECT row_to_json(t) FROM (SELECT account_id::text AS account_id, username, "
                "display_name, status FROM tme.accounts ORDER BY account_id) t",
    "characters": "SELECT row_to_json(t) FROM (SELECT character_id::text AS character_id, "
                  "account_id::text AS account_id, slot, display_name, actor_id "
                  "FROM tme.characters ORDER BY character_id) t",
    "facets": "SELECT row_to_json(t) FROM (SELECT facet_id::text AS facet_id, facet_key, catalog_id, "
              "profile_id, template_id, encode(content_digest,'hex') AS content_digest, "
              "checkpoint_schema, facet_revision, last_server_sequence, "
              "encode(checkpoint_sha256,'hex') AS checkpoint_sha256 FROM tme.facets ORDER BY facet_id) t",
}

#: Pre-fence values the fence is expected to move, kept apart from the projection
#: above so an intended change and a lost fact cannot be confused for each other.
FENCE_SNAPSHOT_QUERIES = {
    "control_epochs": "SELECT row_to_json(t) FROM (SELECT character_id::text AS character_id, "
                      "control_epoch FROM tme.characters ORDER BY character_id) t",
    "fence_epoch": "SELECT row_to_json(t) FROM (SELECT restore_fence_epoch FROM tme.store_state "
                   "WHERE singleton) t",
}

#: The fence revokes every session and removes every unconsumed ticket, so a restored
#: database must hold none of either once it has run.
FENCE_CLEARED_QUERIES = {
    "unrevoked_sessions": "SELECT count(*) FROM tme.sessions WHERE revoked_at IS NULL",
    "unconsumed_tickets": "SELECT count(*) FROM tme.socket_tickets WHERE consumed_at IS NULL",
}

#: Backups written before snapshots existed carry no expectations to check against.
SNAPSHOT_SCHEMA_VERSION = 2


def snapshot(site, database="tme"):
    """Read the identities and durable state a restore of `database` must reproduce."""
    return {name: [json.loads(line) for line in site.sql(query, database).splitlines() if line.strip()]
            for name, query in SNAPSHOT_QUERIES.items()}


def fence_snapshot(site, database="tme"):
    """Read the values the restore fence is expected to change."""
    return {name: [json.loads(line) for line in site.sql(query, database).splitlines() if line.strip()]
            for name, query in FENCE_SNAPSHOT_QUERIES.items()}


def state_differences(expected, actual):
    """Name every way a restored database failed to reproduce the backup.

    Reported per row rather than as a count: the check this replaces could not see a
    substitution that kept the count, and a count mismatch could not say what moved.
    """
    differences = []
    for name in sorted(SNAPSHOT_QUERIES):
        want = {json.dumps(row, sort_keys=True) for row in expected.get(name) or []}
        got = {json.dumps(row, sort_keys=True) for row in actual.get(name) or []}
        differences += [f"{name} lost {row}" for row in sorted(want - got)]
        differences += [f"{name} gained {row}" for row in sorted(got - want)]
    return differences


def fence_differences(expected, site, database):
    """Name every way the fence failed to do exactly what it is supposed to do."""
    differences = []
    before = {row["character_id"]: row["control_epoch"] for row in expected.get("control_epochs") or []}
    after = {row["character_id"]: row["control_epoch"]
             for row in fence_snapshot(site, database)["control_epochs"]}
    for character_id, epoch in sorted(before.items()):
        if character_id not in after:
            differences.append(f"character {character_id} disappeared across the fence")
        elif after[character_id] != epoch + 1:
            differences.append(
                f"character {character_id} control_epoch is {after[character_id]}, expected {epoch + 1}")
    for character_id in sorted(set(after) - set(before)):
        differences.append(f"character {character_id} appeared across the fence")
    recorded = (expected.get("fence_epoch") or [{}])[0].get("restore_fence_epoch")
    if recorded is not None:
        now = fence_snapshot(site, database)["fence_epoch"][0]["restore_fence_epoch"]
        if now != recorded + 1:
            differences.append(f"restore_fence_epoch is {now}, expected {recorded + 1}")
    for name, query in FENCE_CLEARED_QUERIES.items():
        remaining = site.sql(query, database).strip()
        if remaining != "0":
            differences.append(f"{name} is {remaining}, expected 0")
    return differences


def health(site, timeout=30):
    site.check_release()
    context = ssl.create_default_context(cafile=str(site.config / "tls/ca.pem"))
    deadline = time.monotonic() + timeout
    while True:
        try:
            with urllib.request.urlopen(site.local_origin + "/health/ready", context=context, timeout=3) as response:
                public = json.load(response)
            with urllib.request.urlopen(f"http://127.0.0.1:{site.ports['operations']}/internal/status", timeout=3) as response:
                status = json.load(response)
            if not public["gameplay_ready"] or not status["gameplay_ready"]:
                raise RuntimeError("gameplay readiness is false")
            return status
        except (OSError, urllib.error.URLError, RuntimeError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(.1)


def backup(site):
    release = site.check_release()
    directory = site.root / "backups" / (time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + secrets.token_hex(3))
    directory.mkdir(parents=True, mode=0o700)
    path = directory / "database.dump"
    try:
        site.pg("pg_dump", "--format=custom", "--file", path)
        path.chmod(0o600)
        # Recorded in the same breath as the dump, from the database the dump came
        # from, so the drill later compares a restore against this snapshot rather
        # than against a live world that has moved on since.
        document(directory / "backup.json", {
            "schema_version": SNAPSHOT_SCHEMA_VERSION, "sha256": digest(path),
            "release": str(site.current.resolve()), "source_tree": release["source_tree"],
            "storage": release["contracts"]["storage"],
            "snapshot": snapshot(site), "fence": fence_snapshot(site)})
    except BaseException:
        path.unlink(missing_ok=True)
        raise
    return directory


def verify_backup(site, directory):
    directory = directory.resolve()
    if not directory.is_relative_to(site.root / "backups"):
        raise RuntimeError("backup must belong to this installation")
    receipt = json.loads((directory / "backup.json").read_text())
    # Version 1 predates recorded snapshots. It stays restorable but cannot be
    # drilled for preservation, which `restore_drill` refuses explicitly.
    if receipt["schema_version"] not in (1, SNAPSHOT_SCHEMA_VERSION) \
            or digest(directory / "database.dump") != receipt["sha256"]:
        raise RuntimeError("backup digest differs from its receipt")
    if receipt["storage"] != site.check_release()["contracts"]["storage"]:
        raise RuntimeError("backup storage contract differs from this release")
    return directory


def restore_drill(site, directory):
    directory = verify_backup(site, directory)
    receipt = json.loads((directory / "backup.json").read_text())
    expected = receipt.get("snapshot")
    if not expected:
        # Fails closed rather than falling back to a count: a pre-snapshot backup
        # cannot support a preservation claim, and guessing would restore the defect.
        raise RuntimeError(
            "backup records no snapshot of its identities; it cannot be checked for preserved state")
    database = "tme_restore_" + secrets.token_hex(6)
    site.sql(f"CREATE DATABASE {database} OWNER tme_owner", "postgres")
    try:
        site.pg("pg_restore", "--exit-on-error", directory / "database.dump", database=database)
        site.operator("store", "restore-fence", "--confirm-restored-database", database=database)
        site.operator("store", "verify", database=database)
        restored = snapshot(site, database)
        differences = state_differences(expected, restored)
        if differences:
            raise RuntimeError("restored database did not retain its backup: " + "; ".join(differences))
        fenced = fence_differences(receipt.get("fence") or {}, site, database)
        if fenced:
            raise RuntimeError("restore fence did not behave as recorded: " + "; ".join(fenced))
        return {"restored_characters": [row["character_id"] for row in restored["characters"]],
                "restored_accounts": [row["account_id"] for row in restored["accounts"]],
                "restored_worlds": [row["facet_id"] for row in restored["facets"]],
                "fenced_and_verified": True}
    finally:
        site.sql(f"DROP DATABASE {database} WITH (FORCE)", "postgres")


def restore(site, directory):
    directory = verify_backup(site, directory)
    safety = backup(site)
    site.service("stop", UNITS[2], UNITS[1])

    def replace_from(source):
        site.sql("DROP DATABASE IF EXISTS tme WITH (FORCE)", "postgres")
        site.sql("CREATE DATABASE tme OWNER tme_owner", "postgres")
        site.pg("pg_restore", "--exit-on-error", source / "database.dump")
        site.operator("store", "restore-fence", "--confirm-restored-database")
        site.operator("store", "verify")

    try:
        replace_from(directory)
        site.service("start", UNITS[1], UNITS[2])
        result = health(site)
    except BaseException:
        site.service("stop", UNITS[2], UNITS[1])
        replace_from(safety)
        site.service("start", UNITS[1], UNITS[2])
        health(site)
        raise
    return {"restored": str(directory), "safety_backup": str(safety), "status": result}


def activate(site, destination: Path):
    destination = destination.resolve()
    if destination.parent != site.root / "releases":
        raise RuntimeError("release must be an immediate child of this installation's releases")
    previous = site.current.resolve()
    before, after = site.check_release(previous), site.check_release(destination)
    if before["contracts"]["storage"] != after["contracts"]["storage"]:
        raise RuntimeError("storage-changing activation needs an explicit migration slice")
    sources = json.loads((site.config / "seed-sources.json").read_text())
    if any(digest(destination / name) != value for name, value in sources.items()):
        raise RuntimeError("served content changed; a world migration must own that activation")
    backup(site)
    site.service("stop", UNITS[2], UNITS[1])

    def point(path):
        temporary = site.root / ".current-next"
        temporary.unlink(missing_ok=True)
        temporary.symlink_to(path)
        temporary.replace(site.current)

    try:
        point(destination)
        site.service("start", UNITS[1], UNITS[2])
        status = health(site)
    except BaseException:
        site.service("stop", UNITS[2], UNITS[1])
        point(previous)
        site.service("start", UNITS[1], UNITS[2])
        health(site)
        raise
    document(site.config / "activation.json", {"previous": str(previous), "current": str(destination), "status": status})
    return status
