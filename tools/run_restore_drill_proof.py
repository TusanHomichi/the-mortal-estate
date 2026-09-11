#!/usr/bin/env python3
"""Prove the deployment's own backup and restore drill on a scratch installation.

What this proves, and with what
-------------------------------
`deploy/development/operations.py` owns backup and the fenced restore drill, and the
private preview runs both through `manage.py`. This tool drives **those functions**
against a scratch installation on the gated PostgreSQL cluster and then checks their
claims independently. It is not a copy of them and it replaces nothing.

1. **Preservation with a character created at runtime.** A scratch installation is
   provisioned in the deployment's own order — the production roles, a `tme` database
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
3. **A commit that lands during a backup.** A character created while `backup()` runs
   must be absent from both the receipt and the dump, and the drill of that backup must
   still pass although the live world has moved on.
4. **Rejection and isolation.** A dump whose character count is unchanged but whose
   identities and durable state differ must be refused by the real drill, with the
   differing rows named, and the drill's scratch database must be dropped whether the
   comparison fails or the fence expectation does.

Everything is scratch. The cluster comes from `--admin-url-file`, the installation root
is a temporary directory, and every database and role this tool creates is dropped
again. A cluster that already holds a `tme` database is refused outright, which is what
keeps an installed preview out of reach.

Usage:

    tools/run_restore_drill_proof.py --admin-url-file <file> [--keep]
"""

from __future__ import annotations

import argparse
import json
import secrets
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
from operations import backup, restore_drill  # noqa: E402
from provision import bootstrap as stage_bootstrap  # noqa: E402
from provision import validate_settings  # noqa: E402
from run_gated_postgres import Cluster, GatedError  # noqa: E402

from live_server_harness import (  # noqa: E402
    LiveServer,
    ProofError,
    ServedInstallation,
    World,
    build_server,
    read_admin_url,
    reserve_port,
)
from run_production_smoke import CONTROL_API_VERSION, PublicClient, SmokeError  # noqa: E402

SUCCESS_SENTINEL = "TME_RESTORE_DRILL_PROOF_OK"

#: The one database the deployment helpers address by name. A proof that renamed it
#: would stop driving the code the preview runs.
SCRATCH_DATABASE = "tme"

#: The observer contract the frames below are read at.
OBSERVER_CONTRACT_VERSION = 8

#: Scratch databases this tool creates outside the drill's own naming.
ALTERED_PREFIX = "tme_altered_"
ORACLE_PREFIX = "tme_oracle_"


# ---------------------------------------------------------------------------
# The scratch installation
# ---------------------------------------------------------------------------


def postgres_binaries() -> Path:
    """The directory holding the PostgreSQL client the cluster is managed with."""
    if shutil.which("pg_config") is not None:
        return Path(run(["pg_config", "--bindir"])).resolve()
    found = shutil.which("pg_dump")
    if found is None:
        raise GatedError("pg_dump is not on PATH")
    return Path(found).resolve().parent


class ScratchInstallation:
    """An installation root attached to the gated scratch cluster.

    It follows `deploy/development/provision.py` for everything the helpers under test
    actually read: the settings, the release receipt, the roles, the database owner,
    migrations as that owner, the production grants, the generated accounts and the
    bootstrap manifest. It deliberately installs no host services: the proof cluster
    belongs to the caller and the server is started by the live-server harness.
    """

    def __init__(self, cluster: Cluster, root: Path, binary: Path, world_document: str,
                 admin_url: str):
        self.cluster = cluster
        self.root = Path(root)
        self.binary = Path(binary)
        self.world_document = world_document
        parsed = urlsplit(admin_url)
        self.port = parsed.port
        self.administrator = parsed.username
        if self.port is None or not self.administrator:
            raise GatedError("the admin URL must name a host, a port and a role")
        directories = [entry.strip() for entry in
                       cluster.psql("SHOW unix_socket_directories").split(",") if entry.strip()]
        if not directories:
            raise GatedError(
                "the cluster listens on no Unix socket, which the deployment helpers require")
        self.socket_directory = Path(directories[0])
        self.created_roles: set[str] = set()
        self.site: Installation | None = None
        self.release: Path | None = None
        self.accounts: list[dict] = []
        self.credentials: dict[str, str] = {}

    # -- provisioning ------------------------------------------------------

    def declared_settings(self) -> dict:
        """The declaration `validate_settings` accepts, then the installed shape."""
        https = reserve_port()
        declared = {
            "schema_version": 2,
            "world_document": self.world_document,
            "ports": {"postgres": self.port, "server": reserve_port(),
                      "operations": reserve_port(), "https": https},
            "public_origin": f"https://localhost:{https}",
            "presentation_assets": None,
        }
        validate_settings(declared)
        return {**declared, "administrator": self.administrator,
                "postgres_bin": str(postgres_binaries())}

    def stage_scratch_release(self) -> None:
        """A release carrying the real binary and the carried content it serves.

        The receipt is the real shape: the binary's own `contract versions` output and
        a digest for every file, so a storage-contract mismatch cannot hide in the
        proof's scaffolding. The release is assembled here rather than by
        `provision.stage_release`, which builds the browser bundle and refuses a
        working tree with uncommitted changes; nothing under test reads the bundle.
        """
        revision = run(["git", "-C", REPO, "rev-parse", "HEAD"])
        release = self.root / "releases" / "restore-drill-proof"
        (release / "bin").mkdir(parents=True)
        shutil.copy2(self.binary, release / "bin/tme-server")
        for name in run(["git", "-C", REPO, "ls-files", "--", "content"]).splitlines():
            source = REPO / name
            if not source.is_file() or source.is_symlink():
                raise GatedError(f"release content must be regular carried files: {name}")
            copied = release / name
            copied.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, copied)
        document(release / "release.json", {
            "schema_version": 1, "source_tree": revision, "base_commit": revision,
            "contracts": json.loads(run([release / "bin/tme-server", "contract", "versions"])),
            "files": {str(path.relative_to(release)): digest(path)
                      for path in release.rglob("*") if path.is_file()}})
        self.release = release

    def enroll(self) -> None:
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

    def database_credentials(self) -> None:
        """The two roles the served process reads, wired as the installer wires them."""
        for role, name in (("tme_runtime", "database"), ("tme_auth", "auth")):
            password = secrets.token_hex(32)
            self.site.sql(f"ALTER ROLE {role} PASSWORD '{password}'", SCRATCH_DATABASE)
            self.credentials[name] = (
                f"postgresql://{role}:{password}@localhost:{self.port}/{SCRATCH_DATABASE}"
                f"?host={quote(str(self.site.socket))}")

    def url_for(self, database: str, credential: str) -> str:
        """One role's URL re-pointed at another database, for a restored copy."""
        base = self.credentials[credential]
        prefix, separator, suffix = base.partition(f"/{SCRATCH_DATABASE}?")
        if not separator:
            raise GatedError("a scratch credential did not name the installation database")
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

    def provision(self) -> None:
        settings = self.declared_settings()
        self.root.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.site = Installation(self.root)
        document(self.site.config / "settings.json", settings)
        self.site.settings = settings
        # The cluster belongs to the gated runner rather than to this root, so the
        # socket directory is the cluster's own rather than `<root>/socket`. Everything
        # else the helpers read has the installed layout.
        self.site.socket = self.socket_directory
        denylist = private_terms_path(REPOSITORY_ROOT)
        if not denylist.is_file():
            raise GatedError(
                f"the served process reads the private denylist and {denylist} is absent, "
                "so this proof cannot run")
        shutil.copyfile(denylist, self.site.config / "banned-terms.txt")
        (self.site.config / "banned-terms.txt").chmod(0o600)
        self.stage_scratch_release()
        self.site.current.symlink_to(self.release)

        before = set(self.cluster.psql("SELECT rolname FROM pg_roles").split())
        self.cluster.create_database(SCRATCH_DATABASE)
        self.site.sql((REPO / "deploy/production/postgres/18/roles.sql").read_text(), "postgres")
        self.created_roles = set(self.cluster.psql("SELECT rolname FROM pg_roles").split()) - before
        self.site.sql(f"ALTER DATABASE {SCRATCH_DATABASE} OWNER TO tme_owner", "postgres")
        self.site.operator("migrate")
        self.site.sql((REPO / "deploy/production/postgres/18/grants.sql").read_text(),
                      SCRATCH_DATABASE)
        self.database_credentials()
        self.enroll()
        stage_bootstrap(self.site, self.release, self.accounts)

    def close(self) -> list[str]:
        """Drop what this proof created, and report anything it could not.

        The instrument database goes first. The roles then go one at a time with the
        production grants undone: `roles.sql` grants EXECUTE on a system function, and
        a system-catalog grant outlives the database it was made from, so dropping the
        database is not enough to release the role. Problems are returned rather than
        raised, because a proof that already failed must keep that failure.
        """
        problems = []
        try:
            if self.site is not None:
                self.site.sql(f'DROP DATABASE IF EXISTS "{SCRATCH_DATABASE}" WITH (FORCE)',
                              "postgres")
        except (RuntimeError, GatedError, OSError) as error:
            problems.append(f"dropping {SCRATCH_DATABASE} failed: {error}")
        for role in sorted(self.created_roles, reverse=True):
            try:
                if role == "tme_runtime":
                    self.cluster.psql("REVOKE EXECUTE ON FUNCTION "
                                      "pg_catalog.pg_control_system() FROM tme_runtime")
                self.cluster.psql(f'DROP OWNED BY "{role}"')
                self.cluster.psql(f'DROP ROLE IF EXISTS "{role}"')
            except (RuntimeError, GatedError, OSError) as error:
                problems.append(f"dropping role {role} failed: {error}")
        self.created_roles.clear()
        return problems


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
        raise GatedError("the server offered no character creation profile")
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
        raise GatedError("character creation returned no usable response")
    return value["character"]


DIRECTIONS = {"north": (0, -1), "south": (0, 1), "east": (1, 0), "west": (-1, 0)}


def position_of(frame, character_id: str):
    actors = [row for row in frame["actors"] if row.get("character_id") == character_id]
    if len(actors) != 1:
        raise GatedError(f"the frame names character {character_id} {len(actors)} times")
    return actors[0]["position"]


def frame_naming(gameplay, character_id: str, timeout: float, elsewhere=None):
    """Read authoritative frames until one places the character as asked."""
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise GatedError(f"no authoritative frame named {character_id}")
        gameplay.socket.settimeout(remaining)
        try:
            gameplay.receive_json()
        except TimeoutError as error:
            raise GatedError(f"the frame never arrived: {error}") from error
        frame = gameplay.latest_state.get("frame")
        if not isinstance(frame, dict) or frame.get("contract_version") != OBSERVER_CONTRACT_VERSION:
            continue
        actors = [row for row in frame["actors"] if row.get("character_id") == character_id]
        if len(actors) > 1:
            raise GatedError(f"the frame names character {character_id} more than once")
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
        raise GatedError(f"no passable square adjoins the character at ({x},{y})")
    result, _ = gameplay.command({"kind": "move_path", "path": [name]})
    if result.get("disposition") != {"kind": "accepted"}:
        raise GatedError(f"the server refused a step to the {name}: {result.get('disposition')}")
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
        raise GatedError("the character is absent from the session bootstrap")
    session.select(character["slot"])
    gameplay = session.connect()
    try:
        frame = frame_naming(gameplay, character_id, timeout)
        if walk:
            frame = take_one_step(gameplay, frame, character_id, timeout)
        return position_of(frame, character_id)
    finally:
        gameplay.close()


def wait_for_exported_snapshot(site: Installation, timeout: float = 60.0) -> None:
    """Wait until `backup()` has exported the snapshot that binds its reads.

    The exported transaction is what a coordinated commit is aimed at: anything
    committed after it is committed after the dump's instant too. A backup that never
    exports one has nothing to coordinate with, and this proof says so instead of
    racing it.
    """
    query = ("SELECT count(*) FROM pg_stat_activity WHERE state = 'idle in transaction' "
             "AND query LIKE '%pg_export_snapshot%'")
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if site.sql(query, SCRATCH_DATABASE).strip() not in ("", "0"):
            return
        time.sleep(0.02)
    raise GatedError("the backup never exported a snapshot for a commit to be coordinated with")


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
    raise GatedError(f"{getattr(action, '__name__', action)} accepted evidence it must refuse")


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
        raise GatedError("the backup receipt does not record the character created at runtime")
    # The receipt must describe this database's instant, read here without the helper:
    # the digest the drill compares is this one.
    if receipt["snapshot"]["facets"][0]["checkpoint_sha256"] != live_facet_digest(site):
        raise GatedError("the receipt's checkpoint digest is not the live database's")
    restored = restore_drill(site, saved)
    if sorted(restored["restored_characters"]) != sorted(recorded):
        raise GatedError("the drill report does not name the characters the receipt recorded")
    if scratch_databases(site):
        raise GatedError("the drill left a scratch database behind")
    print(f"backup {saved.name}: {len(recorded)} characters recorded, drill preserved "
          f"{len(restored['restored_characters'])}, scratch database dropped")
    report["preserved"] = {"characters": sorted(recorded), "position": position,
                           "backup": saved.name}
    return {"saved": saved, "receipt": receipt, "created": created, "position": position}


def prove_restored_position(admin_url: str, world: World, binary: Path,
                            installation: ScratchInstallation, preserved: dict,
                            report: dict) -> None:
    """Serve a restored copy and read the created character's position from it.

    The drill's comparison is byte-level. This stage restores the same dump with the
    product's own commands, serves it with the real server, and asks the wire where the
    character stands — the semantic form of the same claim.
    """
    site, saved = installation.site, preserved["saved"]
    created = preserved["created"]
    database = ORACLE_PREFIX + secrets.token_hex(6)
    site.sql(f"CREATE DATABASE {database} OWNER tme_owner", "postgres")
    try:
        site.pg("pg_restore", "--exit-on-error", saved / "database.dump", database=database)
        site.operator("store", "restore-fence", "--confirm-restored-database", database=database)
        site.operator("store", "verify", database=database)
        restored = installation.served(created["character_id"], database=database)
        with LiveServer(admin_url, world, binary_path=binary, installation=restored) as server:
            with control_session(server) as session:
                observed = observe_position(session, created["character_id"])
    finally:
        site.sql(f'DROP DATABASE IF EXISTS "{database}" WITH (FORCE)', "postgres")
    if observed != preserved["position"]:
        raise GatedError(f"the restored copy reports {observed}, not {preserved['position']}")
    print(f"restored copy served: {created['character_id']} still at "
          f"{observed['realm']}/{observed['level']} "
          f"({observed['position']['x']},{observed['position']['y']})")
    report["restored_position"] = observed


def prove_concurrent_commit(site: Installation, server: LiveServer, report: dict) -> None:
    """A commit landing during a backup stays out of the dump and its receipt."""
    box: dict = {}

    def take_backup() -> None:
        try:
            box["directory"] = backup(site)
        except BaseException as error:  # re-raised on the thread that can report it
            box["error"] = error

    worker = threading.Thread(target=take_backup, name="backup")
    worker.start()
    try:
        wait_for_exported_snapshot(site)
        with control_session(server) as session:
            created = create_runtime_character(session, "Committed During Backup")
    finally:
        worker.join(timeout=300)
    if worker.is_alive():
        raise GatedError("the backup did not finish")
    if "error" in box:
        raise box["error"]
    saved = box["directory"]
    receipt = json.loads((saved / "backup.json").read_text())
    recorded = {row["character_id"] for row in receipt["snapshot"]["characters"]}
    if created["character_id"] in recorded:
        raise GatedError("a character committed after the snapshot was exported entered the receipt")
    if created["character_id"] not in live_characters(site):
        raise GatedError("the coordinated commit is not in the live database, so nothing was proven")
    if receipt["snapshot"]["facets"][0]["checkpoint_sha256"] == live_facet_digest(site):
        raise GatedError("the live world is unchanged, so the concurrent commit proved nothing")
    restored = restore_drill(site, saved)
    if sorted(restored["restored_characters"]) != sorted(recorded):
        raise GatedError("the drill did not reproduce the receipt's characters")
    if scratch_databases(site):
        raise GatedError("the drill left a scratch database behind")
    print(f"coordinated commit: {created['character_id']} is live and absent from {saved.name}, "
          f"whose drill still preserved {len(recorded)} characters")
    report["concurrent_commit"] = {"absent_from_backup": created["character_id"],
                                   "characters_preserved": sorted(recorded)}


def prove_rejection(site: Installation, preserved: dict, report: dict) -> None:
    """Altered evidence is refused, and a failing drill still drops its database."""
    altered, victim, replacement = substitute_identity(site, preserved)
    message = expect_failure(restore_drill, site, altered)
    if "did not retain its backup" not in message:
        raise GatedError(f"the drill refused an altered backup for the wrong reason: {message}")
    for expected in (victim["character_id"], replacement["character_id"], "facets"):
        if expected not in message:
            raise GatedError(f"the drill's difference report never named {expected}: {message}")
    if scratch_databases(site):
        raise GatedError("a failing comparison left its scratch database behind")
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
        raise GatedError(f"the drill refused a misfenced receipt for the wrong reason: {message}")
    if scratch_databases(site):
        raise GatedError("a fence failure left its scratch database behind")
    print("misfenced receipt refused after the fence ran, scratch database dropped")
    report["rejected"] = {"substituted": victim["character_id"],
                          "replacement": replacement["character_id"],
                          "misfenced": misfenced.name}


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
    try:
        site.pg("pg_restore", "--exit-on-error", saved / "database.dump", database=scratch)
        row = site.sql("SELECT account_id,slot,display_name FROM tme.characters "
                       f"WHERE character_id = '{victim_id}'", scratch).split("|")
        if len(row) != 3:
            raise GatedError(f"the dump does not hold exactly one character {victim_id}")
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
    finally:
        site.sql(f'DROP DATABASE IF EXISTS "{scratch}" WITH (FORCE)', "postgres")
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
    parser.add_argument("--admin-url-file", required=True,
                        help="file holding the scratch cluster's superuser URL")
    parser.add_argument("--world-document", default="content/lands/first-expedition/world.json",
                        help="carried served-world document whose land supports character creation")
    parser.add_argument("--keep", action="store_true",
                        help="keep the scratch root and print where it is")
    return parser.parse_args(argv)


def refuse_an_installed_cluster(cluster: Cluster) -> None:
    """Refuse a cluster that already holds the database this proof creates.

    The proof creates, fences and drops a database named `tme`, which is exactly the
    name an installed preview's world carries. A cluster holding one is not a scratch
    cluster, and the check is what makes that distinction instead of trusting the URL.
    """
    held = cluster.psql(
        f"SELECT count(*) FROM pg_database WHERE datname = '{SCRATCH_DATABASE}'").strip()
    if held != "0":
        raise GatedError(
            f"the cluster already holds a {SCRATCH_DATABASE} database, so it is not a scratch "
            "cluster; this proof creates and drops that database and refuses to touch an "
            "installed world")


def proof(arguments) -> int:
    admin_url = read_admin_url(arguments.admin_url_file)
    cluster = Cluster(admin_url)
    refuse_an_installed_cluster(cluster)
    world = World.declared(arguments.world_document, key="restore-drill-proof")
    binary = build_server()
    root = Path(tempfile.mkdtemp(prefix="tme-restore-drill-"))
    installation = ScratchInstallation(cluster, root, binary, arguments.world_document, admin_url)
    report: dict = {"database": SCRATCH_DATABASE, "world": world.world_template}
    failure = None
    try:
        installation.provision()
        print(f"scratch installation: {root} (database {SCRATCH_DATABASE}, "
              f"socket {installation.socket_directory})")
        manifest = json.loads((installation.site.config / "bootstrap.json").read_text())
        seeded = manifest["characters"][0]["character_id"]
        with LiveServer(admin_url, world, binary_path=binary,
                        installation=installation.served(seeded)) as server:
            preserved = prove_preservation(installation.site, server, report)
            prove_concurrent_commit(installation.site, server, report)
        prove_restored_position(admin_url, world, binary, installation, preserved, report)
        prove_rejection(installation.site, preserved, report)
        if scratch_databases(installation.site):
            raise GatedError("scratch databases survived the proof")
    except BaseException as error:
        failure = error
    # Teardown never replaces a failure of its own: the proof's real outcome is the
    # one worth reporting, and a cleanup problem is an extra fact, not a substitute.
    problems = installation.close()
    cluster.drop_everything()
    if arguments.keep:
        print(f"kept: {root}")
    else:
        shutil.rmtree(root, ignore_errors=True)
    if problems and failure is None:
        raise GatedError("tearing the proof down failed: " + "; ".join(problems))
    if problems:
        print(f"teardown also failed: {'; '.join(problems)}", file=sys.stderr)
    if failure is not None:
        raise failure
    print(json.dumps(report, indent=2, sort_keys=True))
    print(SUCCESS_SENTINEL)
    return 0


def main(argv=None) -> int:
    arguments = parse_args(argv)
    try:
        return proof(arguments)
    except (GatedError, ProofError, SmokeError, OSError, subprocess.SubprocessError) as error:
        print(f"restore drill proof failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
