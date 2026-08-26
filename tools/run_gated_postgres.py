#!/usr/bin/env python3
"""Run every `#[ignore]`-gated PostgreSQL test, each against its own fresh database.

Why this exists
---------------
Six tests in this workspace are `#[ignore]`d because they need a real database.
Until this file existed, nothing in the repository provisioned one: they
compiled on every run and executed on none (successor issue #10). A
certification that is never executed is not a certification.

The contract it implements is `docs/server-notes.md`, "Gated-test runner
contract":

* **One fresh migrated database per gated test.** Two gated tests sharing a
  database fail on each other's state — observed, not hypothetical. Each entry
  below gets a database created, migrated, used once, and dropped.
* **The fenced restore is a real restore.** Its source database is the one the
  durability test just finished with, because later tests truncate and reseed
  different accounts and a copy taken after them lacks `durable_tester`. It is
  `pg_dump`ed into a *new* database — a new oid is what makes the restore real
  — and then fenced with the product's own
  `tme-server store restore-fence --confirm-restored-database`.
* **The EV certification gets its runner-owned identity.** A dedicated role, a
  database named `tme_ev_*` owned by it and stamped
  `COMMENT ... IS 'tme_ev:<sentinel>'`, and an absolute private temp root the
  child-process restart proof can create 0700 directories under. The test
  asserts all four; nothing here can satisfy it by accident.

Usage:

    tools/run_gated_postgres.py --admin-url-file <path> [--only <name>]

The superuser URL is read from a file, never taken from the environment: a URL
with a password in it does not belong in a process listing. Everything created
is dropped on the way out, including after a failure.
"""

from __future__ import annotations

import argparse
import os
import secrets
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

#: Minimum PostgreSQL the EV certification accepts (it asserts 180000..190000).
EV_SERVER_VERSION_RANGE = (180_000, 190_000)


class GatedError(RuntimeError):
    """Provisioning or a gated test could not be completed."""


@dataclass(frozen=True)
class GatedTest:
    """One gated test and how cargo is asked for exactly it."""

    name: str
    #: cargo target selector, e.g. `--test postgres_persistence` or `--lib`.
    target: tuple[str, ...]
    #: The `--exact` filter cargo's harness matches.
    filter: str
    #: How this test's database is prepared: "fresh", "restore", or "ev".
    provisioning: str = "fresh"
    seconds: float = 1800.0


#: The order matters exactly once: `durable` must run immediately before
#: `fenced_restore`, whose source dump is `durable`'s finished database.
GATED_TESTS: tuple[GatedTest, ...] = (
    GatedTest(
        "durable",
        ("--test", "postgres_persistence"),
        "postgres_bootstrap_command_and_restart_are_durable",
    ),
    GatedTest(
        "fenced_restore",
        ("--test", "postgres_persistence"),
        "fenced_restore_hydrates_and_commits_fresh_authenticated_command",
        provisioning="restore",
    ),
    GatedTest(
        "absent_killer_karma",
        ("--test", "postgres_persistence"),
        "absent_killer_karma_is_deferred_and_applied_exactly_once",
    ),
    GatedTest(
        "replayed_kill_assessment",
        ("--test", "postgres_persistence"),
        "replayed_player_kill_assessment_agrees_and_a_contradicting_one_is_refused",
    ),
    GatedTest(
        "ev_certification",
        ("--test", "ev_certification"),
        "ev_postgres_certification",
        provisioning="ev",
        seconds=3600.0,
    ),
    GatedTest(
        "ev_database_fault",
        ("--lib",),
        "postgres::ev_database_fault_certification",
        provisioning="ev",
        seconds=3600.0,
    ),
)


def shell(command: list[str], *, env: dict[str, str] | None = None, seconds: float = 600.0) -> str:
    completed = subprocess.run(
        command, capture_output=True, text=True, env=env, timeout=seconds, check=False
    )
    if completed.returncode != 0:
        raise GatedError(
            f"{command[0]} exited {completed.returncode}\n"
            f"stdout: {completed.stdout.strip()}\nstderr: {completed.stderr.strip()}"
        )
    return completed.stdout


@dataclass
class Cluster:
    """The superuser connection, and everything this run created in it."""

    admin_url: str
    databases: list[str] = field(default_factory=list)
    roles: list[str] = field(default_factory=list)

    @property
    def base(self) -> str:
        return self.admin_url.rsplit("/", 1)[0]

    def url_for(self, database: str, *, role: str | None = None, password: str | None = None) -> str:
        if role is None:
            return f"{self.base}/{database}"
        authority = self.base.split("://", 1)[1]
        host = authority.split("@")[-1]
        return f"postgresql://{role}:{password}@{host}/{database}"

    def psql(self, statement: str, *, database: str | None = None) -> str:
        url = self.admin_url if database is None else self.url_for(database)
        return shell(["psql", url, "-v", "ON_ERROR_STOP=1", "-XAt", "-c", statement])

    def create_database(self, name: str, *, owner: str | None = None) -> str:
        clause = f' OWNER "{owner}"' if owner else ""
        self.psql(f'CREATE DATABASE "{name}"{clause}')
        self.databases.append(name)
        return self.url_for(name)

    def create_role(self, name: str, password: str) -> None:
        self.psql(f"CREATE ROLE \"{name}\" LOGIN PASSWORD '{password}'")
        self.roles.append(name)

    def drop_everything(self) -> None:
        for name in reversed(self.databases):
            subprocess.run(
                ["psql", self.admin_url, "-c", f'DROP DATABASE IF EXISTS "{name}" WITH (FORCE)'],
                capture_output=True,
                text=True,
                check=False,
            )
        for name in reversed(self.roles):
            subprocess.run(
                ["psql", self.admin_url, "-c", f'DROP ROLE IF EXISTS "{name}"'],
                capture_output=True,
                text=True,
                check=False,
            )
        self.databases.clear()
        self.roles.clear()

    def server_version(self) -> int:
        return int(self.psql("SELECT current_setting('server_version_num')").strip())


def build_binaries() -> Path:
    """Build the server binary once. Test binaries build with the test run."""
    shell(["cargo", "build", "--locked", "--bin", "tme-server"], env={**os.environ}, seconds=3600.0)
    binary = ROOT / "target" / "debug" / "tme-server"
    if not binary.is_file():
        raise GatedError(f"the server binary is missing at {binary}")
    return binary


def migrate(binary: Path, url: str) -> None:
    shell([str(binary), "migrate"], env={**os.environ, "DATABASE_URL": url})


def run_cargo_test(test: GatedTest, extra_env: dict[str, str]) -> None:
    command = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "tme-server",
        *test.target,
        "--",
        "--ignored",
        "--exact",
        test.filter,
        "--nocapture",
        "--test-threads=1",
    ]
    environment = {**os.environ, **extra_env}
    print(f"--- {test.name} :: {' '.join(command)}", flush=True)
    completed = subprocess.run(command, cwd=str(ROOT), env=environment, timeout=test.seconds)
    if completed.returncode != 0:
        raise GatedError(f"{test.name} failed with exit {completed.returncode}")
    print(f"--- {test.name} PASS", flush=True)


def prepare_ev(cluster: Cluster, binary: Path, temp_root: Path) -> dict[str, str]:
    """Provision the runner-owned identity the EV certification asserts."""
    version = cluster.server_version()
    low, high = EV_SERVER_VERSION_RANGE
    if not low <= version <= high:
        raise GatedError(
            f"the EV certification requires PostgreSQL {low // 10000}; this cluster reports "
            f"server_version_num={version}"
        )
    token = secrets.token_hex(4)
    role = f"tme_ev_role_{token}"
    database = f"tme_ev_{token}"
    sentinel = secrets.token_hex(8)
    password = secrets.token_urlsafe(24).replace("'", "x")
    cluster.create_role(role, password)
    cluster.create_database(database, owner=role)
    cluster.psql(f"COMMENT ON DATABASE \"{database}\" IS 'tme_ev:{sentinel}'")
    url = cluster.url_for(database, role=role, password=password)
    migrate(binary, url)
    private_root = temp_root / f"ev-{token}"
    private_root.mkdir(mode=0o700, parents=True)
    return {
        "TME_EV_DATABASE_URL": url,
        "TME_EV_DATABASE_NAME": database,
        "TME_EV_DATABASE_SENTINEL": sentinel,
        "TME_EV_DATABASE_ROLE": role,
        "TME_EV_PRIVATE_TEMP_ROOT": str(private_root.resolve()),
    }


def prepare_restore(cluster: Cluster, binary: Path, source_database: str, temp_root: Path) -> dict[str, str]:
    """Dump the durability test's finished database into a genuinely new one.

    A new database has a new oid, which is what makes this a real restore
    rather than a rename: the store refuses to open until the product's own
    restore-fence command acknowledges the change.
    """
    if source_database is None:
        raise GatedError("the fenced restore has no source; the durability test must run first")
    dump = temp_root / "durable.dump"
    shell(
        ["pg_dump", "--format=custom", "--file", str(dump), cluster.url_for(source_database)],
        seconds=1800.0,
    )
    target = f"tme_restore_{secrets.token_hex(4)}"
    url = cluster.create_database(target)
    shell(["pg_restore", "--exit-on-error", "--dbname", url, str(dump)], seconds=1800.0)
    fence = shell(
        [str(binary), "store", "restore-fence", "--confirm-restored-database"],
        env={**os.environ, "DATABASE_URL": url},
    )
    print(fence.strip() or "restore fence applied", flush=True)
    return {"TME_RESTORE_DATABASE_URL": url}


def run(admin_url: str, only: tuple[str, ...]) -> int:
    cluster = Cluster(admin_url)
    temp_root = Path(tempfile.mkdtemp(prefix="tme-gated-"))
    binary = build_binaries()
    selected = [test for test in GATED_TESTS if not only or test.name in only]
    unknown = set(only) - {test.name for test in GATED_TESTS}
    if unknown:
        raise GatedError(f"unknown gated test(s): {sorted(unknown)}")
    last_fresh_database: str | None = None
    try:
        for test in selected:
            if test.provisioning == "fresh":
                database = f"tme_gated_{secrets.token_hex(4)}"
                url = cluster.create_database(database)
                migrate(binary, url)
                last_fresh_database = database
                run_cargo_test(test, {"TME_TEST_DATABASE_URL": url})
            elif test.provisioning == "restore":
                run_cargo_test(
                    test, prepare_restore(cluster, binary, last_fresh_database, temp_root)
                )
            elif test.provisioning == "ev":
                run_cargo_test(test, prepare_ev(cluster, binary, temp_root))
            else:  # pragma: no cover - the dataclass is the only writer
                raise GatedError(f"unknown provisioning {test.provisioning!r}")
    finally:
        cluster.drop_everything()
        shutil.rmtree(temp_root, ignore_errors=True)
    print(f"\nTME_GATED_POSTGRES_OK — {len(selected)} gated test(s), one fresh database each")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--admin-url-file",
        required=True,
        help="file holding a PostgreSQL superuser URL, used only to create and drop databases",
    )
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        help=f"run only this gated test; repeat. One of: {', '.join(t.name for t in GATED_TESTS)}",
    )
    arguments = parser.parse_args(argv)
    try:
        admin_url = Path(arguments.admin_url_file).read_text(encoding="utf-8").strip()
        if not admin_url:
            raise GatedError(f"{arguments.admin_url_file} is empty")
        return run(admin_url, tuple(arguments.only))
    except (GatedError, OSError, subprocess.TimeoutExpired) as error:
        print(f"gated PostgreSQL run failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
