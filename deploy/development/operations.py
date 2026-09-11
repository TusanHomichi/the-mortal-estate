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

from common import UNITS, SnapshotSession, digest, document

#: The durable state a restore must reproduce, read from whichever database it is
#: taken from. Each query aggregates to a single JSON line so it can be read through
#: one open snapshot session without guessing how many rows to expect, and so a
#: column value can never be mistaken for a row boundary.
#:
#: `control_epoch` is deliberately NOT part of this projection. The restore fence
#: increments it on purpose, so it belongs to the fence expectations below rather
#: than to the state that preservation is measured against.
SNAPSHOT_QUERIES = {
    "accounts": "SELECT COALESCE(json_agg(row_to_json(t) ORDER BY t.account_id),'[]'::json)::text "
                "FROM (SELECT account_id::text AS account_id, username, display_name, status "
                "FROM tme.accounts) t",
    "characters": "SELECT COALESCE(json_agg(row_to_json(t) ORDER BY t.character_id),'[]'::json)::text "
                  "FROM (SELECT character_id::text AS character_id, account_id::text AS account_id, "
                  "slot, display_name, actor_id FROM tme.characters) t",
    "facets": "SELECT COALESCE(json_agg(row_to_json(t) ORDER BY t.facet_id),'[]'::json)::text "
              "FROM (SELECT facet_id::text AS facet_id, facet_key, catalog_id, profile_id, "
              "template_id, encode(content_digest,'hex') AS content_digest, checkpoint_schema, "
              "facet_revision, last_server_sequence, encode(checkpoint_sha256,'hex') AS checkpoint_sha256 "
              "FROM tme.facets) t",
}

#: Pre-fence values the fence is expected to move, kept apart from the projection
#: above so an intended change and a lost fact cannot be confused for each other.
FENCE_SNAPSHOT_QUERIES = {
    "control_epochs": "SELECT COALESCE(json_agg(row_to_json(t) ORDER BY t.character_id),'[]'::json)::text "
                      "FROM (SELECT character_id::text AS character_id, control_epoch "
                      "FROM tme.characters) t",
    "fence_epoch": "SELECT COALESCE(json_agg(row_to_json(t)),'[]'::json)::text "
                   "FROM (SELECT restore_fence_epoch FROM tme.store_state WHERE singleton) t",
}

#: The fence revokes every session and removes every unconsumed ticket, so a restored
#: database must hold none of either once it has run.
FENCE_CLEARED_QUERIES = {
    "unrevoked_sessions": "SELECT count(*)::text FROM tme.sessions WHERE revoked_at IS NULL",
    "unconsumed_tickets": "SELECT count(*)::text FROM tme.socket_tickets WHERE consumed_at IS NULL",
}

#: Backups written before snapshots existed carry no expectations to check against.
LEGACY_SCHEMA_VERSION = 1
SNAPSHOT_SCHEMA_VERSION = 2

#: Every section a current receipt must carry. A receipt missing any of these is
#: malformed, which is a different refusal from a legacy receipt that predates them.
REQUIRED_SNAPSHOT_SECTIONS = tuple(SNAPSHOT_QUERIES)
REQUIRED_FENCE_SECTIONS = ("control_epochs", "fence_epoch")


def decode_rows(line):
    """Decode one aggregate line into rows, refusing anything that is not a list."""
    try:
        rows = json.loads(line)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"expected a JSON row list, got {line[:120]!r}: {error}") from error
    if not isinstance(rows, list):
        raise RuntimeError(f"expected a JSON row list, got {type(rows).__name__}")
    return rows


def expectations(source):
    """Read both expectation sections through one source of `one(query)`."""
    values = {name: decode_rows(source.one(query))
              for name, query in {**SNAPSHOT_QUERIES, **FENCE_SNAPSHOT_QUERIES}.items()}
    return ({"accounts": values["accounts"], "characters": values["characters"],
             "facets": values["facets"]},
            {"control_epochs": values["control_epochs"], "fence_epoch": values["fence_epoch"]})


def at_snapshot(site, database, snapshot_id, query):
    """Run one query inside an already-exported snapshot.

    Importing the snapshot is what binds these reads to the dump. Each read is its own
    bounded `psql` run rather than a conversation with a long-lived process, and the
    exporting transaction only has to stay open, not answer questions.
    """
    # The query must be terminated before COMMIT follows it, or psql accumulates both
    # into one malformed statement and PostgreSQL reports a syntax error at COMMIT.
    statement = query.strip().rstrip(";")
    script = ("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;\n"
              f"SET TRANSACTION SNAPSHOT '{snapshot_id}';\n"
              f"{statement};\n"
              "COMMIT;")
    # Quiet: without it the BEGIN/COMMIT command tags arrive alongside the JSON.
    return site.sql(script, database, quiet=True)


class SnapshotReads:
    """Read expectations from one instant, either pinned or current."""

    def __init__(self, site, database, snapshot_id=None):
        self.site, self.database, self.snapshot_id = site, database, snapshot_id

    def one(self, query):
        if self.snapshot_id is None:
            return self.site.sql(query, self.database)
        return at_snapshot(self.site, self.database, self.snapshot_id, query)


def snapshot(site, database="tme"):
    """Read the identities and durable state a restore of `database` must reproduce."""
    state, _ = expectations(SnapshotReads(site, database))
    return state


def fence_snapshot(site, database="tme"):
    """Read the values the restore fence is expected to change."""
    _, fence = expectations(SnapshotReads(site, database))
    return fence


#: `bigint` is the widest column any of these epochs is stored in.
MAXIMUM_EPOCH = 2 ** 63 - 1


def is_epoch(value):
    """A genuine non-negative integer the storage can hold.

    `type(value) is int` rather than `isinstance`, because `True` is an `int` and a
    boolean epoch would otherwise pass as one.
    """
    return type(value) is int and 0 <= value <= MAXIMUM_EPOCH


def validate_expectations(state, fence):
    """Refuse a malformed receipt before any scratch database is created.

    A current receipt missing a section is not the same as a legacy receipt that
    predates the sections entirely: the first is malformed and must fail loudly, the
    second is refused later for a different, explicit reason. Neither may quietly
    reduce the number of assertions performed.
    """
    problems = []
    if not isinstance(state, dict) or not isinstance(fence, dict):
        return ["receipt does not carry a snapshot and fence section"]
    sections = {}
    for name in REQUIRED_SNAPSHOT_SECTIONS:
        sections[name] = state.get(name)
    for name in REQUIRED_FENCE_SECTIONS:
        sections[name] = fence.get(name)
    for name, rows in sections.items():
        if not isinstance(rows, list):
            problems.append(f"receipt is missing its {name} section")
        elif any(not isinstance(row, dict) for row in rows):
            problems.append(f"receipt {name} section contains a row that is not an object")
    if problems:
        return problems
    # Identity counts must be real and unique. A set comparison cannot see a repeated
    # record, and a duplicate would silently reduce what the drill compares.
    for name in ("characters", "control_epochs"):
        identities = [row.get("character_id") for row in sections[name]]
        if any(not isinstance(value, str) or not value for value in identities):
            problems.append(f"receipt {name} section has a row without a character identity")
        elif len(set(identities)) != len(identities):
            problems.append(f"receipt {name} section repeats a character identity")
    if problems:
        return problems
    if len(state["facets"]) != 1:
        problems.append(f"receipt records {len(state['facets'])} worlds, expected exactly one")
    # Each singleton is only inspected once it is known to be one, so a missing or
    # repeated record refuses cleanly instead of raising past the validation.
    if len(fence["fence_epoch"]) != 1:
        problems.append(
            f"receipt records {len(fence['fence_epoch'])} fence epochs, expected exactly one")
    elif not is_epoch(fence["fence_epoch"][0].get("restore_fence_epoch")):
        problems.append("receipt fence epoch is not a storable non-negative integer")
    preserved = [row.get("character_id") for row in state["characters"]]
    epochs = [row.get("character_id") for row in fence["control_epochs"]]
    if set(preserved) != set(epochs):
        problems.append(
            "receipt character identities disagree between its preserved and epoch sections")
    for row in fence["control_epochs"]:
        if not is_epoch(row.get("control_epoch")):
            problems.append(
                f"receipt control epoch for {row.get('character_id')} is not a storable "
                "non-negative integer")
            break
    return problems


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
    """Name every way the fence failed to do exactly what it is supposed to do.

    Every check here is unconditional. `validate_expectations` has already refused a
    receipt that lacks the sections this reads, so a missing field cannot silently
    reduce the number of assertions performed.
    """
    differences = []
    before = {row["character_id"]: row["control_epoch"] for row in expected["control_epochs"]}
    observed = fence_snapshot(site, database)
    after = {row["character_id"]: row["control_epoch"] for row in observed["control_epochs"]}
    for character_id, epoch in sorted(before.items()):
        if character_id not in after:
            differences.append(f"character {character_id} disappeared across the fence")
        elif after[character_id] != epoch + 1:
            differences.append(
                f"character {character_id} control_epoch is {after[character_id]}, expected {epoch + 1}")
    for character_id in sorted(set(after) - set(before)):
        differences.append(f"character {character_id} appeared across the fence")
    recorded = expected["fence_epoch"][0]["restore_fence_epoch"]
    now = observed["fence_epoch"][0]["restore_fence_epoch"]
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
    # One exported snapshot is held across both the dump and the expectation reads, so
    # the receipt describes the same instant the dump does. Reading them through
    # separate transactions would let a change land between the two and be recorded as
    # though the dump had contained it -- a false "lost character" at drill time.
    session = None
    try:
        session = SnapshotSession(site, "tme")
        state, fence = expectations(SnapshotReads(site, "tme", session.identifier))
        site.pg("pg_dump", "--format=custom", "--snapshot", session.identifier, "--file", path)
        path.chmod(0o600)
    except BaseException as error:
        # The original failure is what matters; a cleanup problem is attached to it
        # rather than replacing it.
        problems = session.close(failed=True) if session is not None else []
        path.unlink(missing_ok=True)
        if problems:
            raise RuntimeError(f"{error}; releasing the snapshot also failed: "
                               f"{'; '.join(problems)}") from error
        raise
    # The receipt is published only once the exporting snapshot has been released
    # cleanly, so a shutdown problem cannot disappear behind a backup that otherwise
    # looks complete.
    problems = session.close()
    if problems:
        path.unlink(missing_ok=True)
        raise RuntimeError("the backup was not published because the snapshot session did "
                           "not shut down cleanly: " + "; ".join(problems))
    document(directory / "backup.json", {
        "schema_version": SNAPSHOT_SCHEMA_VERSION, "sha256": digest(path),
        "release": str(site.current.resolve()), "source_tree": release["source_tree"],
        "storage": release["contracts"]["storage"],
        "snapshot_id": session.identifier, "snapshot": state, "fence": fence})
    return directory


def verify_backup(site, directory):
    directory = directory.resolve()
    if not directory.is_relative_to(site.root / "backups"):
        raise RuntimeError("backup must belong to this installation")
    receipt = json.loads((directory / "backup.json").read_text())
    # Version 1 predates recorded snapshots. It stays restorable but cannot be
    # drilled for preservation, which `restore_drill` refuses explicitly.
    if receipt["schema_version"] not in (LEGACY_SCHEMA_VERSION, SNAPSHOT_SCHEMA_VERSION) \
            or digest(directory / "database.dump") != receipt["sha256"]:
        raise RuntimeError("backup digest differs from its receipt")
    if receipt["storage"] != site.check_release()["contracts"]["storage"]:
        raise RuntimeError("backup storage contract differs from this release")
    return directory


def restore_drill(site, directory):
    directory = verify_backup(site, directory)
    receipt = json.loads((directory / "backup.json").read_text())
    if receipt.get("schema_version") == LEGACY_SCHEMA_VERSION:
        # A pre-snapshot backup cannot support a preservation claim, and guessing
        # would restore the defect this replaced. Refused before any scratch work.
        raise RuntimeError(
            "backup records no snapshot of its identities; it cannot be checked for preserved state")
    expected, fence_expected = receipt.get("snapshot"), receipt.get("fence")
    problems = validate_expectations(expected, fence_expected)
    if problems:
        # Malformed evidence is refused, never relaxed into the legacy path, and never
        # allowed to skip a comparison it promised to make.
        raise RuntimeError("backup receipt is not usable for a drill: " + "; ".join(problems))
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
        fenced = fence_differences(fence_expected, site, database)
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
