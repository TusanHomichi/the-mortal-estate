#!/usr/bin/env python3
"""Prove the deployment's own backup and restore drill on a scratch installation.

What this proves, and with what
-------------------------------
`deploy/development/operations.py` owns backup and the fenced restore drill, and the
private preview runs both through `manage.py`. This tool drives **those functions**
against an installation it provisions and owns outright, and then checks their claims
independently. It is not a copy of them and it replaces nothing.

1. **Preservation with a character created at runtime.** The installation is built the
   way `deploy/development/provision.py` builds one — its own `initdb` cluster on a
   reserved port with its socket under the root, the production roles, a `tme` database
   owned by `tme_owner`, migrations run as that owner, the production grants, two
   generated accounts and the real bootstrap manifest — and the real server is started
   against it by `tools/live_server_harness.py`. One more character is created through
   the control API, which is the runtime flow rather than SQL. Its saved position is
   read from the authoritative frame, and the real `backup()` then `restore_drill()`
   must preserve it.
2. **An independent look at the restored copy.** The same dump is restored and fenced
   again with the product's own commands, served by the real server, and the created
   character must appear where the live wire said it stood. That is the semantic form
   of a claim the drill's byte-level comparison cannot make by itself.
3. **A commit that lands during a backup, in the window the claim depends on.** The
   writer is held back until the backup has exported its snapshot and started its first
   pinned read, so the commit happens *between* the dump's instant and the read that
   describes it — enforced by a synchronization-only hook, not raced. The character
   must be live, absent from the receipt and the dump, and irrelevant to the drill.
4. **Rejection, isolation and a cleanup that fails.** A dump whose character count is
   unchanged but whose identities and durable state differ must be refused by the real
   drill, with the differing rows named; a receipt whose fence expectation is moved
   must be refused after the fence has run; and when the drill's own scratch database
   cannot be dropped — held by a real prepared transaction this proof places — the
   drill must still report the preservation failure, the cleanup failure and the
   database's exact name. The proof then recovers that database and leaves none behind.

Everything is scratch and exclusively owned. The cluster, its databases and its roles
are created under a temporary root on a reserved port; no other cluster is contacted,
no existing role is altered, and `tme` exists only here. The root is removed on the way
out, so the cluster's databases and roles die with it.

Usage:

    tools/run_restore_drill_proof.py [--postgres-bin <dir>] [--keep]
"""

from __future__ import annotations

import argparse
import getpass
import json
import secrets
import select
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from contextlib import contextmanager
from pathlib import Path
from urllib.parse import quote, urlsplit

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPOSITORY_ROOT / "deploy/development"))
sys.path.insert(0, str(REPOSITORY_ROOT / "tools"))

from boundary_common import private_terms_path  # noqa: E402
from common import REPO, Installation, digest, document, run, write  # noqa: E402
from operations import backup, release_scratch, restore_drill  # noqa: E402
from provision import bootstrap as stage_bootstrap  # noqa: E402
from provision import validate_settings  # noqa: E402

from live_server_harness import (  # noqa: E402
    LiveServer,
    ProofError,
    ServedInstallation,
    World,
    build_server,
    reserve_port,
)
from run_production_smoke import CONTROL_API_VERSION, PublicClient, SmokeError  # noqa: E402

SUCCESS_SENTINEL = "TME_RESTORE_DRILL_PROOF_OK"

#: The database name the deployment helpers address by name. It exists only in this
#: proof's own cluster, which is what makes a fixed name safe here.
SCRATCH_DATABASE = "tme"

#: The observer contract the frames below are read at.
OBSERVER_CONTRACT_VERSION = 8

#: Scratch databases this tool creates outside the drill's own naming.
ALTERED_PREFIX = "tme_altered_"
ORACLE_PREFIX = "tme_oracle_"
HELD_TRANSACTION = "tme_drill_cleanup_hold"

#: What this proof still owes the cluster it launched.
#:
#: A launch is remembered *before* it is attempted, because `pg_ctl` documents that a
#: timed-out start can continue in the background and succeed: a failed start command
#: is not evidence that no server exists. Only a confirmed shutdown clears the
#: obligation, and a caller that cannot confirm one must keep the root, because those
#: files are what somebody needs to finish or diagnose the job.
UNLAUNCHED = "unlaunched"
LAUNCH_ATTEMPTED = "launch-attempted"
STOPPED = "stopped"


def postgres_binaries() -> Path:
    """The directory holding the PostgreSQL installation this proof runs."""
    if shutil.which("pg_config") is not None:
        return Path(run(["pg_config", "--bindir"])).resolve()
    found = shutil.which("initdb")
    if found is None:
        raise ProofError("initdb is not on PATH; a PostgreSQL server installation is required")
    return Path(found).resolve().parent


class ScratchInstallation:
    """A private development installation whose cluster this proof owns outright.

    It follows `deploy/development/provision.py` for everything the deployment helpers
    read: an `initdb` cluster under the root with its socket beside it, the production
    roles, a `tme` database owned by `tme_owner`, migrations run as that owner, the
    production grants, generated accounts and the real bootstrap manifest. The one
    substitution is the service manager: `provision.install` runs the cluster under a
    systemd user unit, and this proof starts the same cluster with `pg_ctl` because it
    installs no host services.

    Because the cluster is created here, `tme`, `tme_owner` and the other production
    names belong to this run alone. Nothing else can be reached through it, and no role
    that exists elsewhere is read, renamed or given a password.
    """

    def __init__(self, root: Path, binary: Path, world_document: str, pg_bin: Path):
        self.root = Path(root).resolve()
        self.binary = Path(binary)
        self.world_document = world_document
        self.pg_bin = Path(pg_bin)
        self.administrator = getpass.getuser()
        self.site = Installation(self.root)
        self.cluster_state = UNLAUNCHED
        self.release: Path | None = None
        self.accounts: list[dict] = []
        self.credentials: dict[str, str] = {}

    # -- ownership ---------------------------------------------------------

    def assert_ownership(self):
        """Refuse to touch a cluster this proof did not create under its own root.

        The check is asked of the running server, not of this object: whatever answers
        on this installation's socket and port must report the data directory inside
        the temporary root, so a misconfigured or substituted cluster is refused before
        anything destructive runs against it.
        """
        expected = self.site.data.resolve()
        if not expected.is_relative_to(self.root):
            raise ProofError(f"the scratch cluster would live in {expected}, outside {self.root}")
        reported = Path(self.site.sql("SHOW data_directory", "postgres").strip()).resolve()
        if reported != expected:
            raise ProofError(
                f"the cluster answering on {self.site.socket} keeps its data in {reported}, "
                f"not {expected}; this proof only touches the cluster it created")

    # -- provisioning ------------------------------------------------------

    def declared_settings(self) -> dict:
        """The declaration `validate_settings` accepts, then the installed shape."""
        https = reserve_port()
        declared = {
            "schema_version": 2,
            "world_document": self.world_document,
            "ports": {"postgres": reserve_port(), "server": reserve_port(),
                      "operations": reserve_port(), "https": https},
            "public_origin": f"https://localhost:{https}",
            "presentation_assets": None,
        }
        validate_settings(declared)
        return {**declared, "administrator": self.administrator,
                "postgres_bin": str(self.pg_bin)}

    def start_cluster(self):
        """Create and start the cluster, in the order the installer uses.

        `max_prepared_transactions` is raised because the cleanup-failure stage needs a
        real prepared transaction: `DROP DATABASE ... WITH (FORCE)` cannot terminate
        one, which is what makes a drill's cleanup genuinely fail rather than appear to.
        """
        self.site.root.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.site.socket.mkdir(mode=0o700, exist_ok=True)
        run([self.pg_bin / "initdb", "-D", self.site.data, "--auth-local=peer",
             "--auth-host=scram-sha-256", "--encoding=UTF8", "--locale=C.UTF-8",
             "-U", self.administrator], timeout=300)
        with (self.site.data / "postgresql.conf").open("a") as output:
            output.write(f"\nport={self.site.ports['postgres']}\nlisten_addresses='127.0.0.1'\n"
                         f"unix_socket_directories='{self.site.socket}'\nunix_socket_permissions=0700\n"
                         "max_connections=40\nshared_buffers='128MB'\nwork_mem='4MB'\n"
                         "maintenance_work_mem='64MB'\nmax_prepared_transactions=5\n")
        write(self.site.data / "pg_hba.conf",
              f"local all {self.administrator} peer\nlocal all all scram-sha-256\n"
              "host all all 127.0.0.1/32 scram-sha-256\n")
        # The obligation is recorded before the command runs, not after it succeeds:
        # a `pg_ctl` start that times out can still be coming up, and this proof may
        # not treat a failed command as proof that nothing is running.
        self.cluster_state = LAUNCH_ATTEMPTED
        run([self.pg_bin / "pg_ctl", "-D", self.site.data, "-l", self.site.root / "postgres.log",
             "-w", "-t", "60", "start"], timeout=120)
        for attempt in range(100):
            try:
                self.site.sql("SELECT 1", "postgres")
                return
            except RuntimeError:
                if attempt == 99:
                    raise
                time.sleep(0.1)

    def stage_scratch_release(self):
        """A release carrying the real binary and the carried content it serves.

        The receipt is the real shape: the binary's own `contract versions` output and
        a digest for every file, so a storage-contract mismatch cannot hide in the
        proof's scaffolding. The release is assembled here rather than by
        `provision.stage_release`, which builds the browser bundle and refuses a
        working tree with uncommitted changes; nothing under test reads the bundle.
        """
        revision = run(["git", "-C", REPO, "rev-parse", "HEAD"])
        release = self.site.root / "releases" / "restore-drill-proof"
        (release / "bin").mkdir(parents=True)
        shutil.copy2(self.binary, release / "bin/tme-server")
        for name in run(["git", "-C", REPO, "ls-files", "--", "content"]).splitlines():
            source = REPO / name
            if not source.is_file() or source.is_symlink():
                raise ProofError(f"release content must be regular carried files: {name}")
            copied = release / name
            copied.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, copied)
        document(release / "release.json", {
            "schema_version": 1, "source_tree": revision, "base_commit": revision,
            "contracts": json.loads(run([release / "bin/tme-server", "contract", "versions"])),
            "files": {str(path.relative_to(release)): digest(path)
                      for path in release.rglob("*") if path.is_file()}})
        self.release = release

    def enroll(self):
        """Two generated accounts, exactly as the private installer creates them."""
        blocklist = self.site.config / "synthetic-compromised-passwords.txt"
        write(blocklist, "".join(f"synthetic-compromised-{index:06d}\n" for index in range(10_000)))
        for number in (1, 2):
            username, password = f"restore_drill_{number}", secrets.token_urlsafe(32)
            account_id = self.site.operator(
                "account", "create", "--username", username,
                "--display-name", f"Restore Drill {number}",
                "--compromised-passwords", blocklist,
                input=f"{password}\n{password}\n").splitlines()[-1].strip()
            self.accounts.append({"username": username, "password": password,
                                  "account_id": account_id})

    def database_credentials(self):
        """The two roles the served process reads, wired as the installer wires them."""
        for role, name in (("tme_runtime", "database"), ("tme_auth", "auth")):
            password = secrets.token_hex(32)
            self.site.sql(f"ALTER ROLE {role} PASSWORD '{password}'", SCRATCH_DATABASE)
            self.credentials[name] = (
                f"postgresql://{role}:{password}@localhost:{self.site.ports['postgres']}"
                f"/{SCRATCH_DATABASE}?host={quote(str(self.site.socket))}")

    def url_for(self, database: str, credential: str) -> str:
        """One role's URL re-pointed at another database, for a restored copy."""
        base = self.credentials[credential]
        prefix, separator, suffix = base.partition(f"/{SCRATCH_DATABASE}?")
        if not separator:
            raise ProofError("a scratch credential did not name the installation database")
        return f"{prefix}/{database}?{suffix}"

    def served(self, character_id: str, database: str = SCRATCH_DATABASE,
               account: int = 0) -> ServedInstallation:
        """The harness's view of one database of this installation.

        A restored copy carries the same accounts, manifest and credentials; only the
        database name and the URLs that point at it change.
        """
        return ServedInstallation(
            database_name=database,
            database_url=self.url_for(database, "database"),
            auth_database_url=self.url_for(database, "auth"),
            bootstrap_manifest=self.site.config / "bootstrap.json",
            account_id=self.accounts[account]["account_id"],
            character_id=character_id,
            username=self.accounts[account]["username"],
            password=self.accounts[account]["password"])

    def provision(self):
        settings = self.declared_settings()
        # The installation object is told where its cluster is before the cluster
        # exists; the written settings file then carries the same facts.
        self.site.settings = settings
        self.start_cluster()
        self.assert_ownership()
        document(self.site.config / "settings.json", settings)
        denylist = private_terms_path(REPOSITORY_ROOT)
        if not denylist.is_file():
            raise ProofError(
                f"the served process reads the private denylist and {denylist} is absent, "
                "so this proof cannot run")
        shutil.copyfile(denylist, self.site.config / "banned-terms.txt")
        (self.site.config / "banned-terms.txt").chmod(0o600)
        self.stage_scratch_release()
        self.site.current.symlink_to(self.release)

        self.site.sql(f"CREATE DATABASE {SCRATCH_DATABASE}", "postgres")
        self.site.sql((REPO / "deploy/production/postgres/18/roles.sql").read_text(), "postgres")
        self.site.sql(f"ALTER DATABASE {SCRATCH_DATABASE} OWNER TO tme_owner", "postgres")
        self.site.operator("migrate")
        self.site.sql((REPO / "deploy/production/postgres/18/grants.sql").read_text(),
                      SCRATCH_DATABASE)
        self.database_credentials()
        self.enroll()
        stage_bootstrap(self.site, self.release, self.accounts)

    def server_running(self) -> bool:
        """Whether a postmaster still holds this cluster's data directory.

        Asked of `pg_ctl status` rather than of SQL, because cleanup has to work for a
        server that is still coming up, or one that never finished accepting
        connections.
        """
        completed = subprocess.run(
            [str(self.pg_bin / "pg_ctl"), "-D", str(self.site.data), "status"],
            capture_output=True, text=True, check=False)
        return completed.returncode == 0

    def close(self) -> list[str]:
        """Stop the cluster this proof started, and report what would not stop.

        The obligation is cleared by a *confirmed* shutdown, never by attempting one,
        and it survives a repeated call: while a server may still hold the data
        directory, every caller is told so and every call tries again. Nothing to drop
        and no ownership to track — the cluster, its databases, its roles and every
        credential in them live under the temporary root — but the caller must keep
        that root until this returns no problems, because the files are what somebody
        needs to finish or diagnose the job.
        """
        if self.cluster_state != LAUNCH_ATTEMPTED:
            return []
        attempt = None
        try:
            run([self.pg_bin / "pg_ctl", "-D", self.site.data, "-m", "immediate",
                 "-w", "-t", "60", "stop"], timeout=120)
        except Exception as error:  # noqa: BLE001 - every cleanup failure is reportable
            attempt = error
        try:
            running = self.server_running()
        except OSError as error:
            return [f"the scratch cluster in {self.site.data} could not be asked whether "
                    f"it stopped: {error}"]
        if running:
            detail = f": {attempt}" if attempt is not None else ""
            return [f"the scratch cluster in {self.site.data} is still running{detail}"]
        # Nothing holds the cluster any more, whatever the stop command reported.
        self.cluster_state = STOPPED
        return []


class SiteWithBarrier:
    """The real installation, with one synchronization hook in front of its SQL.

    `backup()` reads its expectations through this object, so the hook sees the exact
    statement the real `at_snapshot` sends. Everything else is delegated untouched and
    every statement is passed to the real `sql`: the hook decides *when* the
    coordinated writer commits, never what the backup reads.
    """

    def __init__(self, site, barrier):
        self._site = site
        self._barrier = barrier

    def sql(self, text, database="tme", quiet=False):
        self._barrier(text)
        return self._site.sql(text, database=database, quiet=quiet)

    def __getattr__(self, name):
        return getattr(self._site, name)


class PinnedReadBarrier:
    """Runs the coordinated writer inside the backup's own pinned-read window.

    A poll for "some exporting session" cannot enforce the ordering the concurrency
    claim needs: the backup may finish before the writer commits, and every later
    assertion would still hold. This hook fires on the first expectation read that
    imports the exported snapshot — after `pg_export_snapshot()` and before any read
    or dump — and holds the backup there until the writer's character is committed.
    """

    MARKER = "SET TRANSACTION SNAPSHOT"

    def __init__(self, site, writer):
        self._site = site
        self._writer = writer
        self.created = None
        self.exporters = []

    def __call__(self, statement):
        if self.created is not None or self.MARKER not in statement:
            return
        # The snapshot is exported and the reads have not happened: exactly one
        # exporter can be open in a cluster this proof owns outright.
        self.exporters.append(exporting_sessions(self._site))
        self.created = self._writer()
        self.exporters.append(exporting_sessions(self._site))

    def check(self):
        if self.created is None:
            raise ProofError("the backup read its expectations without a snapshot to wait for")
        if self.exporters != [1, 1]:
            raise ProofError(
                "the coordinated commit did not happen inside an exported snapshot: "
                f"exporting transactions were {self.exporters}")


# ---------------------------------------------------------------------------
# Wire and database helpers
# ---------------------------------------------------------------------------


@contextmanager
def control_session(server: LiveServer):
    """A real login, logged out on the way out however the stage ended."""
    endpoint = urlsplit(server.origin)
    client = PublicClient(endpoint.hostname, endpoint.port, 30.0)
    client.context.load_verify_locations(cafile=str(server.authority))
    session = client.login(server.username, server.password)
    try:
        yield session
    finally:
        try:
            session.logout()
        except (SmokeError, OSError):
            pass


def creation_draft(session):
    """One creation option the server itself offered, with its suggested allocation."""
    value, _ = session.public.request("POST", "/v4/characters/creation", token=session.token,
                                      csrf=session.csrf, body={})
    options = None if value is None else value.get("options")
    if not options:
        raise ProofError("the server offered no character creation profile")
    option = options[0]
    return option["profile_id"], option["suggested"]


def create_runtime_character(session, display_name: str):
    """Create one character through the control API, as a client does."""
    profile_id, attributes = creation_draft(session)
    value, _ = session.public.request(
        "POST", "/v4/characters/create", token=session.token, csrf=session.csrf,
        body={"csrf_token": session.csrf, "request_id": str(uuid.uuid4()),
              "draft": {"profile_id": profile_id, "display_name": display_name,
                        "attributes": attributes}})
    if value is None or value.get("control_api_version") != CONTROL_API_VERSION:
        raise ProofError("character creation returned no usable response")
    return value["character"]


DIRECTIONS = {"north": (0, -1), "south": (0, 1), "east": (1, 0), "west": (-1, 0)}


def position_of(frame, character_id: str):
    actors = [row for row in frame["actors"] if row.get("character_id") == character_id]
    if len(actors) != 1:
        raise ProofError(f"the frame names character {character_id} {len(actors)} times")
    return actors[0]["position"]


def frame_naming(gameplay, character_id: str, timeout: float, elsewhere=None):
    """Read authoritative frames until one places the character as asked."""
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ProofError(f"no authoritative frame named {character_id}")
        gameplay.socket.settimeout(remaining)
        try:
            gameplay.receive_json()
        except TimeoutError as error:
            raise ProofError(f"the frame never arrived: {error}") from error
        frame = gameplay.latest_state.get("frame")
        if not isinstance(frame, dict) or frame.get("contract_version") != OBSERVER_CONTRACT_VERSION:
            continue
        actors = [row for row in frame["actors"] if row.get("character_id") == character_id]
        if len(actors) > 1:
            raise ProofError(f"the frame names character {character_id} more than once")
        if not actors:
            continue
        where = (actors[0]["position"]["position"]["x"], actors[0]["position"]["position"]["y"])
        if elsewhere is not None and where == elsewhere:
            continue
        return frame


def take_one_step(gameplay, frame, character_id: str, timeout: float):
    """Move one square, onto a passable neighbour the server's own frame reports.

    The arrival square belongs to the world document; a square a character walked to
    belongs only to the durable world state. That is what makes the oracle below say
    something the document could not have said for it.
    """
    here = position_of(frame, character_id)
    open_squares = {(tile["position"]["x"], tile["position"]["y"])
                    for tile in frame["tiles"] if tile.get("passable")}
    x, y = here["position"]["x"], here["position"]["y"]
    for name, (dx, dy) in DIRECTIONS.items():
        if (x + dx, y + dy) in open_squares:
            break
    else:
        raise ProofError(f"no passable square adjoins the character at ({x},{y})")
    result, _ = gameplay.command({"kind": "move_path", "path": [name]})
    if result.get("disposition") != {"kind": "accepted"}:
        raise ProofError(f"the server refused a step to the {name}: {result.get('disposition')}")
    print(f"runtime character stepped {name} from ({x},{y})")
    return frame_naming(gameplay, character_id, timeout, elsewhere=(x, y))


def observe_position(session, character_id: str, *, walk: bool = False, timeout: float = 60.0):
    """Read one character's own authoritative frame, and return where it stands.

    This is the independent oracle: the position comes from the running server's wire
    rather than from the database, the receipt, or the drill.
    """
    session.session()
    character = next((row for row in session.bootstrap["characters"]
                      if row.get("character_id") == character_id), None)
    if character is None:
        raise ProofError("the character is absent from the session bootstrap")
    session.select(character["slot"])
    gameplay = session.connect()
    try:
        frame = frame_naming(gameplay, character_id, timeout)
        if walk:
            frame = take_one_step(gameplay, frame, character_id, timeout)
        return position_of(frame, character_id)
    finally:
        gameplay.close()


def exporting_sessions(site: Installation) -> int:
    """How many transactions are holding an exported snapshot open."""
    query = ("SELECT count(*) FROM pg_stat_activity WHERE state = 'idle in transaction' "
             "AND query LIKE '%pg_export_snapshot%'")
    return int(site.sql(query, SCRATCH_DATABASE).strip() or "0")


def scratch_databases(site: Installation) -> list[str]:
    """Every drill scratch database the cluster currently holds."""
    rows = site.sql("SELECT datname FROM pg_database WHERE datname LIKE 'tme\\_restore\\_%' "
                    "ORDER BY datname", "postgres")
    return [row.strip() for row in rows.splitlines() if row.strip()]


def live_characters(site: Installation) -> list[str]:
    rows = site.sql("SELECT character_id FROM tme.characters", SCRATCH_DATABASE)
    return [row.strip() for row in rows.splitlines() if row.strip()]


def live_facet_digest(site: Installation) -> str:
    return site.sql("SELECT encode(checkpoint_sha256,'hex') FROM tme.facets",
                    SCRATCH_DATABASE).strip()


def expect_failure(action, *arguments) -> str:
    """Run an action that must refuse, and return what it said."""
    try:
        action(*arguments)
    except RuntimeError as error:
        return str(error)
    raise ProofError(f"{getattr(action, '__name__', action)} accepted evidence it must refuse")


# ---------------------------------------------------------------------------
# The stages
# ---------------------------------------------------------------------------


def prove_preservation(site: Installation, server: LiveServer, report: dict) -> dict:
    """A runtime-created character survives the real backup and the real drill."""
    with control_session(server) as session:
        created = create_runtime_character(session, "Drill Created")
        print(f"runtime character: {created['character_id']} slot {created['slot']}")
        position = observe_position(session, created["character_id"], walk=True)
    print(f"observed position: {position['realm']}/{position['level']} "
          f"({position['position']['x']},{position['position']['y']})")

    saved = backup(site)
    receipt = json.loads((saved / "backup.json").read_text())
    recorded = {row["character_id"] for row in receipt["snapshot"]["characters"]}
    if created["character_id"] not in recorded:
        raise ProofError("the backup receipt does not record the character created at runtime")
    # The receipt must describe this database's instant, read here without the helper:
    # the digest the drill compares is this one.
    if receipt["snapshot"]["facets"][0]["checkpoint_sha256"] != live_facet_digest(site):
        raise ProofError("the receipt's checkpoint digest is not the live database's")
    restored = restore_drill(site, saved)
    if sorted(restored["restored_characters"]) != sorted(recorded):
        raise ProofError("the drill report does not name the characters the receipt recorded")
    if scratch_databases(site):
        raise ProofError("the drill left a scratch database behind")
    print(f"backup {saved.name}: {len(recorded)} characters recorded, drill preserved "
          f"{len(restored['restored_characters'])}, scratch database dropped")
    report["preserved"] = {"characters": sorted(recorded), "position": position,
                           "backup": saved.name}
    return {"saved": saved, "receipt": receipt, "created": created, "position": position}


def prove_restored_position(server_identity: dict, installation: ScratchInstallation,
                            preserved: dict, report: dict) -> None:
    """Serve a restored copy and read the created character's position from it.

    The drill's comparison is byte-level. This stage restores the same dump with the
    product's own commands, serves it with the real server, and asks the wire where the
    character stands — the semantic form of the same claim.
    """
    site, saved = installation.site, preserved["saved"]
    created = preserved["created"]
    database = ORACLE_PREFIX + secrets.token_hex(6)
    site.sql(f"CREATE DATABASE {database} OWNER tme_owner", "postgres")
    observed = None
    failure = None
    try:
        site.pg("pg_restore", "--exit-on-error", saved / "database.dump", database=database)
        site.operator("store", "restore-fence", "--confirm-restored-database", database=database)
        site.operator("store", "verify", database=database)
        with LiveServer(server_identity["admin_url"], server_identity["world"],
                        binary_path=server_identity["binary"],
                        installation=installation.served(created["character_id"],
                                                         database=database)) as server:
            with control_session(server) as session:
                observed = observe_position(session, created["character_id"])
    except BaseException as error:
        failure = error
    problems = release_scratch(site, database)
    if problems and failure is not None:
        raise ProofError(f"{failure}; releasing the oracle database also failed: "
                         f"{'; '.join(problems)}") from failure
    if problems:
        raise ProofError("the oracle database could not be released: " + "; ".join(problems))
    if failure is not None:
        raise failure
    if observed != preserved["position"]:
        raise ProofError(f"the restored copy reports {observed}, not {preserved['position']}")
    print(f"restored copy served: {created['character_id']} still at "
          f"{observed['realm']}/{observed['level']} "
          f"({observed['position']['x']},{observed['position']['y']})")
    report["restored_position"] = observed


def prove_concurrent_commit(site: Installation, server: LiveServer, report: dict) -> None:
    """A commit landing during a backup stays out of the dump and its receipt."""
    box: dict = {}

    def write():
        """Create the character the backup must not have seen, and commit it."""
        with control_session(server) as session:
            created = create_runtime_character(session, "Committed During Backup")
        if created["character_id"] not in live_characters(site):
            raise ProofError("the coordinated character is not visible to the live database")
        box["created"] = created
        return created

    barrier = PinnedReadBarrier(site, write)
    saved = backup(SiteWithBarrier(site, barrier))
    barrier.check()
    created = barrier.created
    receipt = json.loads((saved / "backup.json").read_text())
    recorded = {row["character_id"] for row in receipt["snapshot"]["characters"]}
    if created["character_id"] in recorded:
        raise ProofError("a character committed after the snapshot was exported entered the receipt")
    if receipt["snapshot"]["facets"][0]["checkpoint_sha256"] == live_facet_digest(site):
        raise ProofError("the live world is unchanged, so the concurrent commit proved nothing")
    restored = restore_drill(site, saved)
    if sorted(restored["restored_characters"]) != sorted(recorded):
        raise ProofError("the drill did not reproduce the receipt's characters")
    if scratch_databases(site):
        raise ProofError("the drill left a scratch database behind")
    print(f"coordinated commit: {created['character_id']} committed inside the exported "
          f"snapshot and is absent from {saved.name}, whose drill still preserved "
          f"{len(recorded)} characters")
    report["concurrent_commit"] = {"absent_from_backup": created["character_id"],
                                   "characters_preserved": sorted(recorded),
                                   "exporting_transactions": barrier.exporters}


def prove_rejection(site: Installation, preserved: dict, report: dict):
    """Altered evidence is refused, and a failing drill still drops its database."""
    altered, victim, replacement = substitute_identity(site, preserved)
    message = expect_failure(restore_drill, site, altered)
    if "did not retain its backup" not in message:
        raise ProofError(f"the drill refused an altered backup for the wrong reason: {message}")
    for expected in (victim["character_id"], replacement["character_id"], "facets"):
        if expected not in message:
            raise ProofError(f"the drill's difference report never named {expected}: {message}")
    if scratch_databases(site):
        raise ProofError("a failing comparison left its scratch database behind")
    print(f"altered backup refused by name: {victim['character_id']} lost, "
          f"{replacement['character_id']} gained, scratch database dropped")

    # The same dump again, with only the receipt's fence expectation moved: the state
    # comparison must pass and the fence comparison must refuse, which is a failure
    # that happens after the restore and the fence have both run.
    misfenced = site.root / "backups" / ("misfenced-" + secrets.token_hex(4))
    shutil.copytree(preserved["saved"], misfenced)
    receipt = json.loads((misfenced / "backup.json").read_text())
    receipt["fence"]["fence_epoch"][0]["restore_fence_epoch"] += 5
    document(misfenced / "backup.json", receipt)
    message = expect_failure(restore_drill, site, misfenced)
    if "restore fence did not behave as recorded" not in message:
        raise ProofError(f"the drill refused a misfenced receipt for the wrong reason: {message}")
    if scratch_databases(site):
        raise ProofError("a fence failure left its scratch database behind")
    print("misfenced receipt refused after the fence ran, scratch database dropped")
    report["rejected"] = {"substituted": victim["character_id"],
                          "replacement": replacement["character_id"],
                          "misfenced": misfenced.name}
    return altered


def hold_the_drill_database(site: Installation, box: dict, timeout: float = 120.0) -> None:
    """Make the drill's own cleanup fail, for a real reason and without a race.

    `DROP DATABASE ... WITH (FORCE)` cannot terminate a prepared transaction, so a
    prepared transaction in the drill's scratch database makes its cleanup fail for
    real. The hold is put in place while the drill is blocked on the exclusive lock its
    own fence takes — a lock this holder acquires first — so by the time the drill can
    reach its drop the hold already exists. The ordering is enforced, not raced.
    """
    deadline = time.monotonic() + timeout
    database = None
    while time.monotonic() < deadline and database is None:
        found = scratch_databases(site)
        database = found[0] if found else None
        if database is None:
            time.sleep(0.01)
    if database is None:
        raise ProofError("the drill never created a scratch database to hold")
    box["database"] = database

    def locker_session():
        return subprocess.Popen(
            list(map(str, [site.pg_bin / "psql", "-XqAt", "-v", "ON_ERROR_STOP=1",
                           "-h", site.socket, "-p", site.ports["postgres"],
                           "-U", site.settings["administrator"], "-d", database])),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def release(holder):
        """End one lock-holder client, without a broken pipe at collection time.

        End of input is what ends `psql`'s loop, so the transaction and its lock are
        released by the client itself; the pipes are closed only once it is gone.
        """
        if holder is None:
            return
        try:
            holder.stdin.close()
        except (OSError, ValueError):
            pass
        try:
            holder.wait(timeout=10)
        except subprocess.TimeoutExpired:
            holder.kill()
            try:
                holder.wait(timeout=10)
            except (OSError, subprocess.SubprocessError):
                pass
        except (OSError, subprocess.SubprocessError):
            pass
        for stream in (holder.stdout, holder.stderr):
            try:
                stream.close()
            except (OSError, ValueError):
                pass

    locker = None
    try:
        while time.monotonic() < deadline:
            if locker is None or locker.poll() is not None:
                # The restored schema may still be arriving: a client that found no
                # table has exited, and the next attempt arrives after it exists.
                release(locker)
                locker = locker_session()
            try:
                locker.stdin.write("BEGIN; LOCK TABLE tme.store_state IN ACCESS EXCLUSIVE "
                                   "MODE; SELECT 'held';\n")
                locker.stdin.flush()
            except (BrokenPipeError, ValueError, OSError):
                release(locker)
                locker = None
                continue
            ready, _, _ = select.select([locker.stdout], [], [], 1.0)
            if ready and locker.stdout.readline().strip() == "held":
                # The drill cannot pass its fence now, so the hold is in place before
                # its drop can run.
                site.sql(f"BEGIN; PREPARE TRANSACTION '{HELD_TRANSACTION}';", database)
                box["held"] = True
                return
        raise ProofError("the drill's fence could not be held for the cleanup stage")
    finally:
        release(locker)


def prove_cleanup_failure(site: Installation, altered: Path, report: dict) -> None:
    """A drill that cannot drop its scratch database reports that as well.

    The preservation failure is the finding; a leaked database is an operational
    problem somebody has to clean up, so the drill must name it without losing the
    first failure. This stage then recovers the database with the deployment's own
    helper and proves the cluster holds no scratch database again.
    """
    box: dict = {}

    def hold():
        try:
            hold_the_drill_database(site, box)
        except BaseException as error:  # re-raised on the thread that can report it
            box["error"] = error

    holder = threading.Thread(target=hold, daemon=True)
    holder.start()
    try:
        message = expect_failure(restore_drill, site, altered)
    finally:
        holder.join(timeout=120)
    if holder.is_alive():
        raise ProofError("the cleanup holder never finished")
    if "error" in box:
        raise box["error"]
    if not box.get("held"):
        raise ProofError("the drill's cleanup was never made to fail")
    database = box["database"]
    if "did not retain its backup" not in message:
        raise ProofError(f"a failing cleanup replaced the preservation failure: {message}")
    if "releasing the drill database also failed" not in message or database not in message:
        raise ProofError(f"the drill did not report the surviving database by name: {message}")
    if "prepared transactions" not in message:
        raise ProofError(f"the drill did not report why the database survived: {message}")
    print(f"cleanup failure reported with its cause: {database} survived a prepared "
          "transaction and the drill kept the preservation failure")

    # Recovery: the hold is released, and the deployment's own helper removes the leak.
    site.sql(f"ROLLBACK PREPARED '{HELD_TRANSACTION}'", database)
    problems = release_scratch(site, database)
    if problems:
        raise ProofError("the leaked drill database could not be recovered: " + "; ".join(problems))
    if scratch_databases(site):
        raise ProofError("a scratch database survived the cleanup stage")
    print(f"recovered: {database} released and dropped")
    report["cleanup_failure"] = {"database": database, "recovered": True}


def substitute_identity(site: Installation, preserved: dict):
    """A backup of the same dump with one identity swapped and durable state moved.

    The character count is deliberately unchanged: that is the substitution the fixed
    count this drill replaced could not see.
    """
    saved, victim_id = preserved["saved"], preserved["created"]["character_id"]
    directory = site.root / "backups" / ("altered-" + secrets.token_hex(4))
    directory.mkdir(parents=True, mode=0o700)
    scratch = ALTERED_PREFIX + secrets.token_hex(6)
    site.sql(f"CREATE DATABASE {scratch} OWNER tme_owner", "postgres")
    failure = None
    try:
        site.pg("pg_restore", "--exit-on-error", saved / "database.dump", database=scratch)
        row = site.sql("SELECT account_id,slot,display_name FROM tme.characters "
                       f"WHERE character_id = '{victim_id}'", scratch).split("|")
        if len(row) != 3:
            raise ProofError(f"the dump does not hold exactly one character {victim_id}")
        account_id, slot, display_name = row
        victim = {"character_id": victim_id, "account_id": account_id, "slot": int(slot),
                  "display_name": display_name}
        replacement = {"character_id": str(uuid.uuid4()), "account_id": account_id,
                       "slot": int(slot), "display_name": "Substituted Identity",
                       "actor_id": "substituted_identity"}
        site.sql("UPDATE tme.sessions SET selected_character_id = NULL "
                 f"WHERE selected_character_id = '{victim_id}'", scratch)
        site.sql(f"DELETE FROM tme.characters WHERE character_id = '{victim_id}'", scratch)
        site.sql("INSERT INTO tme.characters (character_id,account_id,slot,display_name,actor_id) "
                 f"VALUES ('{replacement['character_id']}','{account_id}',{slot},"
                 f"'{replacement['display_name']}','{replacement['actor_id']}')", scratch)
        site.sql("UPDATE tme.facets SET facet_revision = facet_revision + 1", scratch)
        site.pg("pg_dump", "--format=custom", "--file", directory / "database.dump",
                database=scratch)
    except BaseException as error:
        failure = error
    problems = release_scratch(site, scratch)
    if problems and failure is not None:
        raise ProofError(f"{failure}; releasing the altered copy also failed: "
                         f"{'; '.join(problems)}") from failure
    if problems:
        raise ProofError("the altered copy could not be released: " + "; ".join(problems))
    if failure is not None:
        raise failure
    receipt = json.loads((saved / "backup.json").read_text())
    receipt["sha256"] = digest(directory / "database.dump")
    document(directory / "backup.json", receipt)
    return directory, victim, replacement


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def parse_args(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--postgres-bin",
                        help="PostgreSQL bin directory; derived from pg_config when omitted")
    parser.add_argument("--world-document", default="content/lands/first-expedition/world.json",
                        help="carried served-world document whose land supports character creation")
    parser.add_argument("--keep", action="store_true",
                        help="keep the scratch root and print where it is")
    return parser.parse_args(argv)


def proof(arguments) -> int:
    pg_bin = Path(arguments.postgres_bin) if arguments.postgres_bin else postgres_binaries()
    world = World.declared(arguments.world_document, key="restore-drill-proof")
    binary = build_server()
    root = Path(tempfile.mkdtemp(prefix="tme-restore-drill-"))
    installation = ScratchInstallation(root, binary, arguments.world_document, pg_bin)
    report: dict = {"database": SCRATCH_DATABASE, "world": world.world_template,
                    "postgres_bin": str(pg_bin)}
    failure = None
    try:
        installation.provision()
        server_identity = {"admin_url": installation_url(installation), "world": world,
                           "binary": binary}
        print(f"scratch installation: {root} (database {SCRATCH_DATABASE}, "
              f"port {installation.site.ports['postgres']}, socket {installation.site.socket})")
        manifest = json.loads((installation.site.config / "bootstrap.json").read_text())
        seeded = manifest["characters"][0]["character_id"]
        with LiveServer(server_identity["admin_url"], world, binary_path=binary,
                        installation=installation.served(seeded)) as server:
            preserved = prove_preservation(installation.site, server, report)
            prove_concurrent_commit(installation.site, server, report)
        prove_restored_position(server_identity, installation, preserved, report)
        altered = prove_rejection(installation.site, preserved, report)
        prove_cleanup_failure(installation.site, altered, report)
        if scratch_databases(installation.site):
            raise ProofError("scratch databases survived the proof")
    except BaseException as error:
        failure = error
    # Teardown never replaces a failure of its own: the proof's real outcome is the one
    # worth reporting, and a cleanup problem is an extra fact, not a substitute. The
    # root is removed only once no server holds it; otherwise it stays, because a
    # cluster that would not stop is exactly what somebody has to come back to.
    problems = installation.close()
    if problems:
        retained = (f"the scratch root {root} was retained because the cluster it holds "
                    f"was not stopped: {'; '.join(problems)}")
        if failure is not None:
            raise ProofError(f"{failure}; {retained}") from failure
        raise ProofError(retained)
    # Nothing holds the root, so the proof's own outcome can be reported after it is
    # gone — or, with --keep, alongside it.
    if arguments.keep:
        print(f"kept: {root}")
    else:
        shutil.rmtree(root, ignore_errors=True)
    if failure is not None:
        raise failure
    print(json.dumps(report, indent=2, sort_keys=True))
    print(SUCCESS_SENTINEL)
    return 0


def installation_url(installation: ScratchInstallation) -> str:
    """The scratch cluster's own superuser URL, for the harness's unused argument.

    The harness never connects to it in installation mode; it is the URL of the
    cluster this proof owns, so a future caller that did use it would reach nothing
    else.
    """
    port = installation.site.ports["postgres"]
    return (f"postgresql://{installation.administrator}@localhost:{port}/postgres"
            f"?host={quote(str(installation.site.socket))}")


def main(argv=None) -> int:
    arguments = parse_args(argv)
    try:
        return proof(arguments)
    except (ProofError, RuntimeError, SmokeError, OSError,
            subprocess.SubprocessError) as error:
        print(f"restore drill proof failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
