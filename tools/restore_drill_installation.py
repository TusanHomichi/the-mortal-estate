"""Own the isolated PostgreSQL installation used by the restore-drill proof.

Provisioning, resource identity and confirmed cluster shutdown live here. The
launcher owns proof scenarios and the lifetime of the enclosing temporary root.
"""

from __future__ import annotations

import getpass
import json
import secrets
import shutil
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import quote

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPOSITORY_ROOT / "deploy/development"))

from boundary_common import private_terms_path  # noqa: E402
from common import REPO, Installation, digest, document, run, write  # noqa: E402
from provision import bootstrap as stage_bootstrap, validate_settings  # noqa: E402
from live_server_harness import ProofError, ServedInstallation, reserve_port  # noqa: E402

# The deployment helpers address this fixed name only inside our owned cluster.
SCRATCH_DATABASE = "tme"

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

#: `pg_ctl status` distinguishes "running" from "not running" by exit status. Only those
#: two answers are evidence; every other outcome — another status, a signal, an
#: executable that cannot be run, a check that does not finish — is an unresolved
#: inspection, and a check that failed cannot authorize deleting the resource it failed
#: to inspect.
SERVER_RUNNING = 0
SERVER_NOT_RUNNING = 3
STATUS_TIMEOUT_SECONDS = 30.0


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
        source_tree = run(["git", "-C", REPO, "write-tree"])
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
            "schema_version": 1, "source_tree": source_tree, "base_commit": revision,
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

    def server_state(self):
        """`pg_ctl`'s answer about this cluster: running, stopped, or unconfirmed.

        Asked of `pg_ctl status` rather than of SQL, because cleanup has to work for a
        server that is still coming up, or one that never finished accepting
        connections. The tool's own two answers are the only evidence: exit 0 means a
        postmaster holds the data directory, exit 3 means none does. Anything else is
        unconfirmed and says so, with the status and whatever the tool printed, so a
        failed inspection can never be read as a stopped server.
        """
        try:
            completed = subprocess.run(
                [str(self.pg_bin / "pg_ctl"), "-D", str(self.site.data), "status"],
                capture_output=True, text=True, check=False, timeout=STATUS_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired as error:
            return "unconfirmed", (f"the status check did not finish within "
                                   f"{STATUS_TIMEOUT_SECONDS:g}s: {error}")
        except OSError as error:
            return "unconfirmed", f"the status check could not run: {error}"
        if completed.returncode == SERVER_RUNNING:
            return "running", ""
        if completed.returncode == SERVER_NOT_RUNNING:
            return "stopped", ""
        detail = (completed.stdout or "").strip() or (completed.stderr or "").strip()
        return "unconfirmed", (f"the status check answered {completed.returncode}"
                               + (f": {detail[:200]}" if detail else ""))

    def close(self) -> list[str]:
        """Stop the cluster this proof started, and report what would not stop.

        The obligation is cleared by a *confirmed* shutdown, never by attempting one,
        and it survives a repeated call: while a server may still hold the data
        directory -- or while that cannot be established -- every caller is told so and
        every call tries again. Nothing to drop and no ownership to track: the cluster,
        its databases, its roles and every credential in them live under the temporary
        root. But the caller must keep that root until this returns no problems, because
        the files are what somebody needs to finish or diagnose the job.
        """
        if self.cluster_state != LAUNCH_ATTEMPTED:
            return []
        attempt = None
        try:
            run([self.pg_bin / "pg_ctl", "-D", self.site.data, "-m", "immediate",
                 "-w", "-t", "60", "stop"], timeout=120)
        except Exception as error:  # noqa: BLE001 - every cleanup failure is reportable
            attempt = error
        state, detail = self.server_state()
        if state == "stopped":
            # Nothing holds the cluster any more, whatever the stop command reported.
            self.cluster_state = STOPPED
            return []
        if state == "running":
            suffix = f": {attempt}" if attempt is not None else ""
            return [f"the scratch cluster in {self.site.data} is still running{suffix}"]
        return [f"the scratch cluster in {self.site.data} could not be confirmed stopped: "
                f"{detail}"]
