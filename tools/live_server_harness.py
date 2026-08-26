#!/usr/bin/env python3
"""Provision a real server on a scratch database and hand a client the keys.

Two proofs need exactly this: `run_client_live_proof.py`, which walks the
shipped client through sign-in and play, and `run_fixture_land_capture.py`,
which takes a gameplay capture over the authoring fixture's land. They differ
only in which world they serve and which client script they drive, so the
provisioning lives here once rather than twice.

What it provisions, in order: a scratch PostgreSQL database, the schema, one
enrolled account, one bootstrapped character, the real `tme-server` binary, and
a loopback TLS front with a throwaway authority the client is told to trust.

Everything it needs lives in this repository except the PostgreSQL superuser
URL, which is used only to create and drop the scratch database.

Three inputs are **generated for the run** rather than tracked, because each is
a fact about this run rather than about the project: the bootstrap manifest
(which names this run's account and world key), the compromised-password
blocklist the enrolment gate requires, and — for a land whose only tracked seed
belongs to another land — a minimal simulation seed placing the controlled
actor. None of them is content; none of them outlives the run directory.
"""

from __future__ import annotations

import json
import os
import secrets
import shutil
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from contextlib import closing
from dataclasses import dataclass, field
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
GODOT_VERSION = "4.7.2.stable.official.ed1daf0bf"
READY_TIMEOUT_SECONDS = 120.0
PROXY_HOST = "localhost"

#: The one catalog the prototype content corpus carries, and the profile whose
#: terrain vocabulary every authored land compiles against. A land may name its
#: own in its served-world document; these are what a run falls back on when it
#: serves a land that has none, which today is the authoring fixture.
CATALOG = "content/test-corpus/catalogs/prototype_catalog_v6.json"
CATALOG_PROFILE = "profile/first_land_structure"

#: The document kind a land uses to declare which content makes its world.
SERVED_WORLD_KIND = "served_world"


class ProofError(RuntimeError):
    """A step of the provisioning could not be completed."""


@dataclass(frozen=True)
class World:
    """Which world a run serves, and who the player is in it."""

    #: Repository-relative path to the runtime world template.
    world_template: str
    #: Repository-relative path to a tracked simulation seed, or None when the
    #: seed is generated for the run (see `generated_seed`).
    simulation_seed: str | None = None
    #: A complete simulation-seed document to write into the run directory.
    #: Used when no tracked seed describes this land's levels.
    generated_seed: dict | None = None
    controlled_actor: str = "player"
    rng_seed: int = 7
    key: str = "live-proof-world"
    catalog: str = CATALOG
    catalog_profile: str = CATALOG_PROFILE

    @classmethod
    def declared(cls, document: str, *, key: str = "live-proof-world") -> "World":
        """The world a land declares for itself, read from its own document.

        A land's `world.json` is the one tracked statement of which catalog,
        profile, compiled template, and seed make its world. A harness that
        restated any of them would be a second opinion about which land is
        served, and the first time the two disagreed nobody would notice.
        """
        path = REPOSITORY_ROOT / document
        declared = json.loads(path.read_text(encoding="utf-8"))
        if declared.get("schema_version") != 1 or declared.get("kind") != SERVED_WORLD_KIND:
            raise ProofError(f"{document} is not a version 1 {SERVED_WORLD_KIND} document")
        base = path.parent

        def resolve(field: str) -> str:
            named = declared[field]
            resolved = (base / named).resolve()
            if not resolved.is_file():
                raise ProofError(f"{document} names {field} {named}, which is not a file")
            return str(resolved.relative_to(REPOSITORY_ROOT))

        return cls(
            world_template=resolve("world_template"),
            simulation_seed=resolve("simulation_seed"),
            controlled_actor=declared["controlled_actor"],
            rng_seed=declared["rng_seed"],
            key=key,
            catalog=resolve("catalog"),
            catalog_profile=declared["catalog_profile"],
        )

    def seed_path(self, run_directory: Path) -> Path:
        if self.simulation_seed is not None:
            return REPOSITORY_ROOT / self.simulation_seed
        if self.generated_seed is None:
            raise ProofError("a world needs either a tracked seed or a generated one")
        path = run_directory / "simulation-seed.json"
        path.write_text(json.dumps(self.generated_seed, indent=2), encoding="utf-8")
        return path

    def seed_document(self, run_directory: Path) -> dict:
        """The seed this run actually serves, tracked or generated."""
        if self.generated_seed is not None:
            return self.generated_seed
        return json.loads(self.seed_path(run_directory).read_text(encoding="utf-8"))


def reserve_port() -> int:
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def run(command: list[str], *, env: dict[str, str] | None = None, stdin: str | None = None) -> str:
    completed = subprocess.run(
        command,
        input=stdin,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if completed.returncode != 0:
        raise ProofError(
            f"{command[0]} failed with status {completed.returncode}\n"
            f"stdout: {completed.stdout.strip()}\nstderr: {completed.stderr.strip()}"
        )
    return completed.stdout


def validate_godot(binary: Path) -> None:
    output = run([str(binary), "--version"]).strip().splitlines()[-1].strip()
    if output != GODOT_VERSION:
        raise ProofError(f"Godot must be exactly {GODOT_VERSION}; this binary reports {output}")


def build_server() -> Path:
    run(["cargo", "build", "--bin", "tme-server"], env={**os.environ})
    binary = REPOSITORY_ROOT / "target" / "debug" / "tme-server"
    if not binary.is_file():
        raise ProofError(f"the server binary is missing at {binary}")
    return binary


def create_certificates(directory: Path) -> tuple[Path, Path, Path]:
    """Issues a throwaway authority and a `localhost` leaf signed by it."""
    authority_key = directory / "ca.key"
    authority_certificate = directory / "ca.pem"
    leaf_key = directory / "leaf.key"
    leaf_request = directory / "leaf.csr"
    leaf_certificate = directory / "leaf.pem"
    extensions = directory / "leaf.ext"
    extensions.write_text("subjectAltName=DNS:localhost,IP:127.0.0.1\n", encoding="utf-8")
    run([
        "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-sha256",
        "-days", "1", "-subj", "/CN=The Mortal Estate proof authority",
        "-keyout", str(authority_key), "-out", str(authority_certificate),
    ])
    run([
        "openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes", "-sha256",
        "-subj", "/CN=localhost", "-keyout", str(leaf_key), "-out", str(leaf_request),
    ])
    run([
        "openssl", "x509", "-req", "-in", str(leaf_request),
        "-CA", str(authority_certificate), "-CAkey", str(authority_key), "-CAcreateserial",
        "-out", str(leaf_certificate), "-days", "1", "-sha256", "-extfile", str(extensions),
    ])
    run([
        "openssl", "verify", "-CAfile", str(authority_certificate),
        "-verify_hostname", "localhost", str(leaf_certificate),
    ])
    return authority_certificate, leaf_certificate, leaf_key


class TlsProxy:
    """Terminates TLS on a loopback port and forwards bytes to the server.

    Deliberately dumb: it copies bytes in both directions and adds no headers,
    so the `Host` and `Origin` the client sent are exactly what the server
    validates. Anything smarter would be proving the proxy, not the client.
    """

    def __init__(self, listen_port: int, upstream_port: int, certificate: Path, key: Path) -> None:
        self._upstream_port = upstream_port
        self._context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self._context.load_cert_chain(str(certificate), str(key))
        self._listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._listener.bind(("127.0.0.1", listen_port))
        self._listener.listen(16)
        self._running = True
        self._thread = threading.Thread(target=self._accept_loop, daemon=True)
        self._thread.start()

    def _accept_loop(self) -> None:
        while self._running:
            try:
                raw, _ = self._listener.accept()
            except OSError:
                return
            threading.Thread(target=self._serve, args=(raw,), daemon=True).start()

    def _serve(self, raw: socket.socket) -> None:
        try:
            client = self._context.wrap_socket(raw, server_side=True)
        except (ssl.SSLError, OSError):
            raw.close()
            return
        try:
            upstream = socket.create_connection(("127.0.0.1", self._upstream_port))
        except OSError:
            client.close()
            return
        for source, destination in ((client, upstream), (upstream, client)):
            threading.Thread(target=self._pump, args=(source, destination), daemon=True).start()

    @staticmethod
    def _pump(source: socket.socket, destination: socket.socket) -> None:
        """Copies one direction and half-closes when it ends.

        Half-closing rather than tearing both directions down lets a TLS peer
        finish the close it started; a full shutdown here cuts the session
        mid-record and the client logs a spurious TLS error on an otherwise
        clean sign-out.
        """
        try:
            while True:
                chunk = source.recv(65536)
                if not chunk:
                    break
                destination.sendall(chunk)
        except OSError:
            pass
        try:
            destination.shutdown(socket.SHUT_WR)
        except OSError:
            pass

    def close(self) -> None:
        self._running = False
        self._listener.close()


def blocklist_path(directory: Path) -> Path:
    """Enrolment requires a compromised-password list of 10,000-1,000,000 lines.

    These are synthetic and exist only so the real gate runs; the run account's
    own password is generated and is not among them.
    """
    path = directory / "compromised-passwords.txt"
    path.write_text(
        "".join(f"synthetic-compromised-{index:06d}\n" for index in range(10_000)),
        encoding="utf-8",
    )
    return path


def write_credential_directory(directory: Path, database_url: str) -> Path:
    credentials = directory / "credentials"
    credentials.mkdir(mode=0o700, exist_ok=True)
    for name in ("database-url", "auth-database-url"):
        path = credentials / name
        path.write_text(database_url + "\n", encoding="utf-8")
        path.chmod(0o600)
    return credentials


@dataclass
class LiveServer:
    """A provisioned server, its world, and the environment a client needs.

    Used as a context manager. Everything it created is removed on exit unless
    `keep` is set, in which case it prints where it left the pieces.
    """

    admin_url: str
    world: World
    keep: bool = False

    run_directory: Path = field(init=False)
    database_name: str = field(init=False)
    database_url: str = field(init=False)
    server_log: Path = field(init=False)
    account_id: str = field(init=False)
    character_id: str = field(init=False)
    username: str = field(init=False)
    password: str = field(init=False)
    origin: str = field(init=False)
    authority: Path = field(init=False)
    status: dict = field(init=False, default_factory=dict)
    _server: subprocess.Popen | None = field(init=False, default=None)
    _proxy: TlsProxy | None = field(init=False, default=None)

    def __enter__(self) -> "LiveServer":
        self.run_directory = Path(tempfile.mkdtemp(prefix="tme-live-"))
        self.server_log = self.run_directory / "server.log"
        self.database_name = f"tme_live_{secrets.token_hex(4)}"
        self.database_url = self.admin_url.rsplit("/", 1)[0] + "/" + self.database_name
        print(f"run directory: {self.run_directory}")
        try:
            self._provision()
        except BaseException:
            self.close()
            raise
        return self

    def __exit__(self, *_exception) -> None:
        self.close()

    # -- provisioning ------------------------------------------------------

    def _provision(self) -> None:
        server_binary = build_server()
        run([
            "psql", self.admin_url, "-v", "ON_ERROR_STOP=1",
            "-c", f'CREATE DATABASE "{self.database_name}"',
        ])
        print(f"database: {self.database_name}")

        offline = {**os.environ, "DATABASE_URL": self.database_url}
        run([str(server_binary), "migrate"], env=offline)
        print("schema: migrated")

        self.username = "proof_operator"
        self.password = "proof-passphrase-" + secrets.token_hex(8)
        self.account_id = run(
            [
                str(server_binary), "account", "create",
                "--username", self.username,
                "--display-name", "Proof Operator",
                "--compromised-passwords", str(blocklist_path(self.run_directory)),
            ],
            env=offline,
            stdin=f"{self.password}\n{self.password}\n",
        ).strip().splitlines()[-1].strip()
        print(f"account: {self.account_id}")

        self.character_id = str(uuid.uuid4())
        manifest = self._write_bootstrap_manifest()
        run([str(server_binary), "bootstrap", "verify", str(manifest)], env=offline)
        print(f"character: {self.character_id}")

        public_port = reserve_port()
        operations_port = reserve_port()
        tls_port = reserve_port()
        self.authority, certificate, key = create_certificates(self.run_directory)
        self._proxy = TlsProxy(tls_port, public_port, certificate, key)
        self.origin = f"https://{PROXY_HOST}:{tls_port}"
        print(f"origin: {self.origin}")

        environment = {
            **os.environ,
            "CREDENTIALS_DIRECTORY": str(
                write_credential_directory(self.run_directory, self.database_url)
            ),
            "TME_BANNED_TERMS_FILE": str(REPOSITORY_ROOT / ".boundary" / "banned-terms.txt"),
            "TME_PUBLIC_LISTEN": f"127.0.0.1:{public_port}",
            "TME_OPS_LISTEN": f"127.0.0.1:{operations_port}",
            "TME_PUBLIC_HOST": f"{PROXY_HOST}:{tls_port}",
            "TME_PUBLIC_ORIGIN": self.origin,
            "TME_BOOTSTRAP_MANIFEST": str(manifest),
            # The server filters tracing from the environment, so without this
            # the log it writes is empty and any tail of it would say nothing.
            "RUST_LOG": os.environ.get("RUST_LOG", "info"),
        }
        with self.server_log.open("w", encoding="utf-8") as log_handle:
            self._server = subprocess.Popen(
                [str(server_binary), "serve"],
                env=environment,
                stdout=log_handle,
                stderr=subprocess.STDOUT,
                text=True,
            )
        self.status = self._wait_for_ready(operations_port)
        print(
            "server: gameplay_ready=%s protocol=%s.%s"
            % (
                self.status.get("gameplay_ready"),
                self.status.get("protocol_major"),
                self.status.get("protocol_minor"),
            )
        )

    def _write_bootstrap_manifest(self) -> Path:
        manifest = {
            "schema_version": 1,
            "catalog": str(REPOSITORY_ROOT / self.world.catalog),
            "catalog_profile": self.world.catalog_profile,
            "world_template": str(REPOSITORY_ROOT / self.world.world_template),
            "world": {
                "facet_id": str(uuid.uuid4()),
                "key": self.world.key,
                "simulation_seed": str(self.world.seed_path(self.run_directory)),
                "rng_seed": self.world.rng_seed,
            },
            "characters": [{
                "account_id": self.account_id,
                "character_id": self.character_id,
                "slot": 1,
                "display_name": "Wayfarer",
                "actor_id": self.world.controlled_actor,
            }],
        }
        path = self.run_directory / "bootstrap.json"
        path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
        return path

    def _wait_for_ready(self, operations_port: int) -> dict:
        deadline = time.monotonic() + READY_TIMEOUT_SECONDS
        last_error = "no response"
        while time.monotonic() < deadline:
            if self._server is not None and self._server.poll() is not None:
                raise ProofError(
                    f"the server exited with status {self._server.returncode} before becoming ready\n"
                    f"{self.server_log.read_text(encoding='utf-8', errors='replace')[-4000:]}"
                )
            try:
                with urllib.request.urlopen(
                    f"http://127.0.0.1:{operations_port}/internal/status", timeout=2.0
                ) as response:
                    payload = json.loads(response.read().decode("utf-8"))
                if bool(payload.get("gameplay_ready")):
                    return payload
                last_error = f"gameplay_ready is {payload.get('gameplay_ready')}"
            except (urllib.error.URLError, OSError, ValueError) as error:
                last_error = str(error)
            time.sleep(0.5)
        raise ProofError(f"the server never reported gameplay readiness: {last_error}")

    # -- driving a client --------------------------------------------------

    def client_environment(self, extra: dict[str, str] | None = None) -> dict[str, str]:
        environment = {
            **os.environ,
            "TME_EX_HTTPS_BASE_URL": self.origin,
            "TME_EX_WEBSOCKET_URL": f"wss://{PROXY_HOST}:{self.origin.rsplit(':', 1)[1]}/v3/socket",
            "TME_EX_ORIGIN": self.origin,
            "TME_EX_CA_PATH": str(self.authority),
            "TME_EX_USERNAME": self.username,
            "TME_EX_PASSWORD": self.password,
            "TME_EX_CHARACTER_ID": self.character_id,
        }
        environment.update(extra or {})
        return environment

    def run_client(
        self,
        godot: Path,
        script: str,
        *,
        extra_environment: dict[str, str] | None = None,
        timeout: float = 300.0,
        display: list[str] | None = None,
        window: tuple[int, int] | None = None,
    ) -> subprocess.CompletedProcess:
        """Runs one client script against this server.

        `display` prefixes the command with a virtual-display launcher, for a
        run that must actually draw. Without it the client runs headless, which
        is right for every proof that reads client state rather than pixels —
        and is the reason a capture cannot: Godot's headless display driver
        produces no viewport image at all.
        """
        command = list(display or [])
        command += [str(godot), "--path", str(REPOSITORY_ROOT / "client")]
        if not display:
            command.append("--headless")
        if window is not None:
            command += ["--resolution", f"{window[0]}x{window[1]}"]
        command += ["-s", script]
        return subprocess.run(
            command,
            env=self.client_environment(extra_environment),
            capture_output=True,
            text=True,
            check=False,
            timeout=timeout,
        )

    def log_tail(self, lines: int = 12) -> str:
        tail = self.server_log.read_text(encoding="utf-8", errors="replace").splitlines()[-lines:]
        return "\n".join(tail) if tail else "(the server wrote nothing)"

    # -- teardown ----------------------------------------------------------

    def close(self) -> None:
        if self._server is not None and self._server.poll() is None:
            self._server.terminate()
            try:
                self._server.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self._server.kill()
        self._server = None
        if self._proxy is not None:
            self._proxy.close()
            self._proxy = None
        if not hasattr(self, "run_directory"):
            return
        if self.keep:
            print(f"kept: database {self.database_name}, run directory {self.run_directory}")
            return
        subprocess.run(
            ["psql", self.admin_url, "-c",
             f'DROP DATABASE IF EXISTS "{self.database_name}" WITH (FORCE)'],
            capture_output=True,
            text=True,
            check=False,
        )
        shutil.rmtree(self.run_directory, ignore_errors=True)


def resolve_godot(value: str) -> Path:
    if not value:
        raise ProofError("the pinned Godot binary must be named")
    godot = Path(value).resolve()
    validate_godot(godot)
    return godot


def read_admin_url(path: str) -> str:
    return Path(path).read_text(encoding="utf-8").strip()


def emit_client_output(completed: subprocess.CompletedProcess) -> None:
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
