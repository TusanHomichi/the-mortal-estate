"""Explicit browser capture operation; never imported by selection consumers."""
from __future__ import annotations

import json
import os
import signal
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from live_server_harness import LiveServer, ProofError, World, read_admin_url
from live_wire_client import LiveWireClient
from run_production_smoke import SmokeError

from . import capture
from .projection import verify


def _run(root: Path, arguments: list[str], *, document: dict | None = None) -> str:
    process = subprocess.Popen(arguments, cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(None if document is None else json.dumps(document), timeout=300)
    finally:
        # Also covers timeout/cancellation: Node's browser/display children must
        # not survive the operation that created them.
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
    if process.returncode:
        # Configuration can contain an ephemeral ticket; commands and stdin are
        # never included in a diagnostic or written to disk.
        raise capture.CaptureUnavailable(f"browser producer exited {process.returncode}: {stderr[-2000:]}")
    return stdout


def produce(projection, destination: Path, *, world_document: str | None = None,
            replay_directory: Path | None = None, admin_url_file: str | None = None) -> list[Path]:
    """Build the carried codec, capture in both engines, verify, publish together."""
    root = projection.root
    if root != Path(__file__).resolve().parents[2]:
        raise capture.CaptureUnavailable("run fresh capture with the selected checkout's own Workbench tools")
    if (world_document is None) == (replay_directory is None):
        raise capture.CaptureUnavailable("configure exactly one live world or replay capture")
    verify(root, projection.sources)
    destination.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".browser-capture-", dir=destination))
    began = time.monotonic()
    try:
        _run(root, ["node", "web/proof/build-codec.mjs"])
        sources = [source.as_record() for source in projection.sources]
        configurations = []
        if replay_directory is not None:
            replay_directory = (root / replay_directory).resolve()
            if not replay_directory.is_relative_to(root):
                raise capture.CaptureUnavailable("replay capture must be inside the selected working root")
            taken = capture.load(root, replay_directory)
            capture.bind(projection, taken)
            authority = taken.document.get("authority")
            if not authority:
                raise capture.CaptureUnavailable("replay requires a browser authoritative recording")
            recorded = json.loads((replay_directory / "capture.frame.json").read_bytes())
            configurations = [{"envelopes": recorded["envelopes"], "sources": authority["sources"]}]
        else:
            if not admin_url_file:
                raise capture.CaptureUnavailable("live capture requires TME_PG_ADMIN_URL_FILE or --admin-url-file")
            world = World.declared(world_document, key="browser-capture-world")
            runtime = next((source.path for source in projection.sources if source.role == "runtime_projection"), None)
            if runtime != world.world_template:
                raise capture.CaptureUnavailable("live world does not use this Workbench's compiled runtime projection")
            with LiveServer(read_admin_url(admin_url_file), world) as server:
                try:
                    for engine in ("chromium", "firefox"):
                        # Control credentials stay in the Python adapter. The page
                        # receives only a fresh, one-use ticket and uses native WSS.
                        public = LiveWireClient(server).public
                        session = public.login(server.username, server.password)
                        try:
                            selected = next(row for row in session.bootstrap["characters"] if row["character_id"] == server.character_id)
                            session.select(selected["slot"])
                            document = {"origin": server.origin, "ticket": session.ticket(), "sources": sources}
                            _run(root, ["node", "web/proof/authoritative-capture.mjs"], document={**document, "engine": engine, "output": str(staging / engine)})
                        finally:
                            token = session.token
                            session.logout()
                            public.request("POST", "/v4/session", body={}, token=token, expected=(401,))
                finally:
                    shutil.copyfile(server.server_log, staging / "server.log")
        for configuration in configurations:
            for engine in ("chromium", "firefox"):
                _run(root, ["node", "web/proof/authoritative-capture.mjs"], document={**configuration, "engine": engine, "output": str(staging / engine)})
        verify(root, projection.sources)
        directories = sorted(staging.glob("*/live")) + sorted(staging.glob("*/replay"))
        if len(directories) != (4 if world_document else 2):
            raise capture.CaptureUnavailable("both engines did not produce all required captures")
        for directory in directories:
            capture.bind(projection, capture.load(root, directory))
        # One directory rename publishes the completed batch; a failed engine
        # never leaves half of a new capture operation offered in the Workbench.
        batch = destination / ("batch-" + staging.name.removeprefix(".browser-capture-"))
        (staging / "operation.json").write_text(json.dumps({"elapsed_seconds": time.monotonic() - began, "engines": ["chromium", "firefox"]}) + "\n")
        relative = [directory.relative_to(staging) for directory in directories]
        staging.rename(batch)
        return [batch / path for path in relative]
    except (OSError, ValueError, subprocess.SubprocessError, ProofError, SmokeError) as error:
        raise capture.CaptureUnavailable(str(error)) from error
    finally:
        if (staging / "server.log").exists():
            shutil.copyfile(staging / "server.log", destination / "last-server.log")
        if staging.exists():
            shutil.rmtree(staging)
