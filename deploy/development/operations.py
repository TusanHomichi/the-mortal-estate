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


def health(site, timeout=30):
    site.check_release()
    context = ssl.create_default_context(cafile=str(site.config / "tls/ca.pem"))
    deadline = time.monotonic() + timeout
    while True:
        try:
            with urllib.request.urlopen(site.origin + "/health/ready", context=context, timeout=3) as response:
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
        document(directory / "backup.json", {"schema_version": 1, "sha256": digest(path),
                 "release": str(site.current.resolve()), "source_tree": release["source_tree"], "storage": release["contracts"]["storage"]})
    except BaseException:
        path.unlink(missing_ok=True)
        raise
    return directory


def verify_backup(site, directory):
    directory = directory.resolve()
    if not directory.is_relative_to(site.root / "backups"):
        raise RuntimeError("backup must belong to this installation")
    receipt = json.loads((directory / "backup.json").read_text())
    if receipt["schema_version"] != 1 or digest(directory / "database.dump") != receipt["sha256"]:
        raise RuntimeError("backup digest differs from its receipt")
    if receipt["storage"] != site.check_release()["contracts"]["storage"]:
        raise RuntimeError("backup storage contract differs from this release")
    return directory


def restore_drill(site, directory):
    directory = verify_backup(site, directory)
    database = "tme_restore_" + secrets.token_hex(6)
    site.sql(f"CREATE DATABASE {database} OWNER tme_owner", "postgres")
    try:
        site.pg("pg_restore", "--exit-on-error", directory / "database.dump", database=database)
        site.operator("store", "restore-fence", "--confirm-restored-database", database=database)
        site.operator("store", "verify", database=database)
        counts = site.sql("SELECT (SELECT count(*) FROM tme.characters),(SELECT count(*) FROM tme.facets)", database)
        if counts != "2|1":
            raise RuntimeError("restored database did not retain both characters and one world")
        return {"restored_characters": 2, "restored_worlds": 1, "fenced_and_verified": True}
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
