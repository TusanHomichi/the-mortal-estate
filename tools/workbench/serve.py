#!/usr/bin/env python3
"""The local Workbench application: one owner, one machine, loopback only.

    python3 tools/workbench/serve.py

Serves the logical view and a handful of narrow JSON endpoints from the
repository checkout. This is not a service boundary. It binds loopback, it
serves the person who started it, and no external-compatibility, versioning, or
deployment policy activates because it exists.

**Nothing here builds anything.** Serving and selecting read files and hash
them; they invoke no compiler and no test runner. The logical projection is
produced ahead of time by the authoring compiler (`cargo run -p tme-authoring`);
if it is missing or stale this server refuses to open, and says which command
produces it.

**Authoring operations run the compiler only when asked.** Selection over the
logical projection or an existing capture reads files and starts no program.
Only the explicit capture route invokes the browser producer; no ordinary
selection, image read, comment, or staging operation invokes it.
"""

from __future__ import annotations

import argparse
import json
import sys
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from ipaddress import ip_address
from pathlib import Path
import os
from urllib.parse import parse_qs

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from workbench import VERSION  # noqa: E402
from workbench import apply as apply_module  # noqa: E402
from workbench import bridge  # noqa: E402
from workbench import capture as capture_reader  # noqa: E402
from workbench import capture_producer  # noqa: E402
from workbench import imageops  # noqa: E402
from workbench import operations as operation_log  # noqa: E402
from workbench import replay as replay_module  # noqa: E402
from workbench.identity import resolve  # noqa: E402
from workbench.packet import (  # noqa: E402
    CAPTURE_GEOMETRY_KEYS,
    MASKED_GESTURES,
    build,
    cells_for_gesture,
    geometry_of,
    mask_bytes,
    now,
    resolution_of,
)
from workbench.projection import (  # noqa: E402
    DEFAULT_PROJECTION_PATH,
    ProjectionUnavailable,
    Source,
    StaleSelection,
    WorkbenchError,
    load,
    load_candidate,
    verify,
)
from workbench.session import open_session, repository_revision  # noqa: E402

EXIT_OK = 0
EXIT_UNSERVABLE = 3

DEFAULT_HOST = "127.0.0.1"
#: Exactly the files this tool means to serve, named one at a time. There is no
#: directory route on purpose: a wildcard static handler in a tool that reads a
#: repository checkout is a file-disclosure route, and "it only binds loopback"
#: is not the kind of argument this repository accepts for one. The page is a
#: graph of ES modules, so every module gets a line here; a module that is
#: missing from this map is a blank page, which is why the app never reaches for
#: a file it did not declare.
JAVASCRIPT = "application/javascript; charset=utf-8"
STATIC = {
    "/": ("app/index.html", "text/html; charset=utf-8"),
    "/static/app.css": ("app/app.css", "text/css; charset=utf-8"),
    "/static/api.js": ("app/api.js", JAVASCRIPT),
    "/static/app.js": ("app/app.js", JAVASCRIPT),
    "/static/apply.js": ("app/apply.js", JAVASCRIPT),
    "/static/candidate.js": ("app/candidate.js", JAVASCRIPT),
    "/static/capture.js": ("app/capture.js", JAVASCRIPT),
    "/static/gestures.js": ("app/gestures.js", JAVASCRIPT),
    "/static/identities.js": ("app/identities.js", JAVASCRIPT),
    "/static/logical.js": ("app/logical.js", JAVASCRIPT),
    "/static/parameters.js": ("app/parameters.js", JAVASCRIPT),
    "/static/session.js": ("app/session.js", JAVASCRIPT),
    "/static/staging.js": ("app/staging.js", JAVASCRIPT),
    "/static/state.js": ("app/state.js", JAVASCRIPT),
    "/static/surface.js": ("app/surface.js", JAVASCRIPT),
    "/static/view.js": ("app/view.js", JAVASCRIPT),
    "/static/views.js": ("app/views.js", JAVASCRIPT),
}
LOOPBACK_NAMES = ("localhost", "127.0.0.1", "::1", "[::1]")

#: The asset verbs this slice can actually perform. The other four are declared
#: contracts with nothing registered to serve them, and the interface says so
#: rather than offering a button that refuses.
_SERVED_ASSET_VERBS = ("edit_region",)


class Workbench:
    """Everything one running Workbench holds. No mutable world state."""

    def __init__(self, root: Path, projection_path: str, session_id: str | None,
                 capture_configuration: dict | None = None) -> None:
        self.root = Path(root).resolve()
        self.projection = load(self.root, projection_path)
        self.session = open_session(self.projection, session_id)
        self.captures: dict[str, capture_reader.Capture] = {}
        self.capture_configuration = capture_configuration
        # The candidate the last preview produced, if there is one. Derived
        # state, replaced whole by the next preview and never persisted here:
        # the LOG is what a session keeps, and the candidate is a function of it.
        self.candidate = None
        self._reload_captures()

    def check(self) -> None:
        """Recompute every bound digest. Raises StaleSelection when one moved."""
        verify(self.root, self.projection.sources)

    def _reload_captures(self) -> None:
        """Read the captures this session already holds, refusing broken ones.

        A capture directory whose three files stopped describing each other is
        not offered as a selection surface at all; it is reported so the owner
        can see why it disappeared rather than wondering.
        """
        self.captures = {}
        self.broken_captures: dict[str, str] = {}
        directory = self.session.directory / capture_reader.CAPTURES_DIR
        if not directory.is_dir():
            return
        for child in sorted([*directory.glob("cap-*"), *directory.glob("batch-*/*/live"), *directory.glob("batch-*/*/replay")]):
            identifier = "-".join(child.relative_to(directory).parts)
            try:
                taken = capture_reader.load(self.root, child)
                capture_reader.bind(self.projection, taken)
            except WorkbenchError as error:
                self.broken_captures[identifier] = str(error)
                continue
            self.captures[identifier] = taken

    def capture_id(self, taken: capture_reader.Capture) -> str:
        return "-".join(taken.directory.relative_to(self.session.directory / capture_reader.CAPTURES_DIR).parts)

    def take_capture(self) -> list[dict]:
        if self.capture_configuration is None:
            raise capture_reader.CaptureUnavailable("start Workbench with --capture-world or --capture-replay to configure browser capture")
        paths = capture_producer.produce(self.projection, self.session.directory / capture_reader.CAPTURES_DIR,
                                         **self.capture_configuration)
        self.check()
        self._reload_captures()
        return [self.capture_summary(taken) for taken in self.captures.values() if taken.directory in paths]

    def capture(self, identifier: str) -> capture_reader.Capture:
        try:
            taken = self.captures[identifier]
        except KeyError:
            raise WorkbenchError(
                f"this session holds no capture {identifier!r}"
                + (
                    f" ({self.broken_captures[identifier]})"
                    if identifier in self.broken_captures
                    else ""
                )
            ) from None
        verify(self.root, [Source.from_record(record) for record in taken.source_records(self.root)])
        return taken

    def capture_summary(self, taken: capture_reader.Capture) -> dict:
        return {
            "capture_id": self.capture_id(taken),
            "directory": taken.relative,
            "image": taken.relative_path(self.root, capture_reader.IMAGE_NAME),
            "member": taken.level,
            "realm": taken.realm,
            "frame_generation": taken.frame_generation,
            "viewport": taken.viewport,
            "camera": taken.camera,
            "targets": len(taken.targets),
            "route": taken.document.get("route"),
            "digests": taken.source_records(self.root),
        }

    def state(self) -> dict:
        return {
            "workbench_version": VERSION,
            "view": "logical",
            "view_label": "LOGICAL VIEW — the authoring compiler's own projection",
            "session": self.session.manifest,
            "session_directory": self.session.relative,
            "repository_revision": repository_revision(self.root),
            "selections": self.session.selection_ids(),
            "operations": len(self.session.operations()),
            "staged": operation_log.summary(self.session.staged()),
            "applies": [
                record
                for record in self.session.operations()
                if record.get("kind") == operation_log.APPLY_RECORDED
            ],
            "captures": [
                self.capture_summary(self.captures[identifier])
                for identifier in sorted(self.captures)
            ],
            "broken_captures": dict(sorted(self.broken_captures.items())),
            "capture_available": self.capture_configuration is not None,
        }


class Handler(BaseHTTPRequestHandler):
    server_version = f"workbench/{VERSION}"
    protocol_version = "HTTP/1.1"
    workbench: Workbench

    def log_message(self, format: str, *args) -> None:  # noqa: A002 - base class name
        sys.stderr.write(f"{self.address_string()} {format % args}\n")

    # -- plumbing ---------------------------------------------------------

    def _send(self, status: HTTPStatus, payload: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(payload)

    def _json(self, status: HTTPStatus, value) -> None:
        self._send(status, json.dumps(value, indent=2).encode("utf-8"), "application/json")

    def _error(self, status: HTTPStatus, kind: str, detail: str) -> None:
        self._json(status, {"error": kind, "detail": detail})

    def _loopback_only(self) -> bool:
        """One owner on one machine. A request from anywhere else is refused."""
        try:
            if not ip_address(self.client_address[0]).is_loopback:
                return False
        except ValueError:
            return False
        host = (self.headers.get("Host") or "").rsplit(":", 1)[0]
        return host in LOOPBACK_NAMES or host == ""

    def _body(self) -> dict:
        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0:
            raise WorkbenchError("the request carried no body")
        try:
            value = json.loads(self.rfile.read(length))
        except json.JSONDecodeError as error:
            raise WorkbenchError(f"the request body is not JSON: {error}") from error
        if not isinstance(value, dict):
            raise WorkbenchError("the request body must be an object")
        return value

    # -- routes -----------------------------------------------------------

    def do_GET(self) -> None:  # noqa: N802 - base class name
        if not self._loopback_only():
            self._error(HTTPStatus.FORBIDDEN, "not_loopback", "this Workbench serves loopback only")
            return
        path, _, query = self.path.partition("?")
        if path in STATIC:
            relative, content_type = STATIC[path]
            payload = (Path(__file__).resolve().parent / relative).read_bytes()
            self._send(HTTPStatus.OK, payload, content_type)
            return
        try:
            self.workbench.check()
        except StaleSelection as stale:
            self._error(HTTPStatus.CONFLICT, "stale", str(stale))
            return
        if path == "/api/projection":
            self._json(HTTPStatus.OK, self.workbench.projection.document)
        elif path == "/api/operations":
            self._vocabulary()
        elif path == "/api/state":
            self._json(HTTPStatus.OK, self.workbench.state())
        elif path == "/api/packet":
            self._packet(query)
        elif path == "/api/capture/image":
            self._capture_image(query)
        else:
            self._error(HTTPStatus.NOT_FOUND, "no_route", f"no route {path}")

    def do_POST(self) -> None:  # noqa: N802 - base class name
        if not self._loopback_only():
            self._error(HTTPStatus.FORBIDDEN, "not_loopback", "this Workbench serves loopback only")
            return
        path, _, _ = self.path.partition("?")
        try:
            self.workbench.check()
        except StaleSelection as stale:
            self._error(HTTPStatus.CONFLICT, "stale", str(stale))
            return
        try:
            body = self._body()
            if path == "/api/preview":
                self._json(HTTPStatus.OK, self._resolve_gesture(body))
            elif path == "/api/selection":
                self._json(HTTPStatus.CREATED, self._record(body))
            elif path == "/api/capture/preview":
                self._json(HTTPStatus.OK, self._resolve_capture_gesture(body))
            elif path == "/api/capture/selection":
                self._json(HTTPStatus.CREATED, self._record_capture(body))
            elif path == "/api/capture":
                if body:
                    raise WorkbenchError("capture uses only the producer configured at startup")
                self._json(HTTPStatus.CREATED, {"captures": self.workbench.take_capture()})
            elif path == "/api/stage":
                self._json(HTTPStatus.CREATED, self._stage(body))
            elif path == "/api/retract":
                self._json(HTTPStatus.CREATED, self._retract(body))
            elif path == "/api/candidate":
                self._json(HTTPStatus.OK, self._candidate())
            elif path == "/api/candidate/preview":
                self._json(HTTPStatus.OK, self._resolve_candidate_gesture(body))
            elif path == "/api/apply":
                self._json(HTTPStatus.CREATED, self._apply(body))
            elif path == "/api/accept":
                self._json(HTTPStatus.CREATED, self._accept(body))
            elif path == "/api/comment":
                self._json(
                    HTTPStatus.CREATED,
                    self.workbench.session.write_comment(
                        body.get("selection_id"), str(body["comment"])
                    ),
                )
            else:
                self._error(HTTPStatus.NOT_FOUND, "no_route", f"no route {path}")
        except KeyError as error:
            self._error(HTTPStatus.BAD_REQUEST, "bad_request", f"missing field {error}")
        except capture_reader.CaptureUnavailable as error:
            # Honest unavailability: the capture could not be taken or could not
            # be trusted, and the reason names which.
            self._error(HTTPStatus.SERVICE_UNAVAILABLE, "capture_unavailable", str(error))
        except WorkbenchError as error:
            self._error(HTTPStatus.BAD_REQUEST, "bad_request", str(error))

    def _packet(self, query: str) -> None:
        identifier = parse_qs(query).get("id", [None])[0]
        if not identifier:
            self._error(HTTPStatus.BAD_REQUEST, "bad_request", "no packet id given")
            return
        try:
            packet = self.workbench.session.read_packet(identifier)
            resolution = resolution_of(self.workbench.projection, packet)
        except WorkbenchError as error:
            self._error(HTTPStatus.NOT_FOUND, "no_packet", str(error))
            return
        self._json(HTTPStatus.OK, {"packet": packet, "resolution": resolution})

    def _capture_image(self, query: str) -> None:
        identifier = parse_qs(query).get("id", [None])[0]
        if not identifier:
            self._error(HTTPStatus.BAD_REQUEST, "bad_request", "no capture id given")
            return
        try:
            taken = self.workbench.capture(identifier)
        except WorkbenchError as error:
            self._error(HTTPStatus.NOT_FOUND, "no_capture", str(error))
            return
        self._send(HTTPStatus.OK, taken.image, "image/png")

    def _resolve_gesture(self, body: dict) -> dict:
        member = self.workbench.projection.member(str(body["member"]))
        gesture = str(body["gesture"])
        cells = cells_for_gesture(member, gesture, body)
        return {"member": member.member, "gesture": gesture, **resolve(member, cells)}

    def _capture_gesture(self, body: dict) -> tuple:
        """A gesture over a capture, resolved through the identity raster."""
        taken = self.workbench.capture(str(body["capture_id"]))
        gesture = str(body["gesture"])
        geometry = geometry_of(gesture, body, CAPTURE_GEOMETRY_KEYS)
        return taken, capture_reader.select(
            self.workbench.projection, taken, gesture, geometry
        )

    def _resolve_capture_gesture(self, body: dict) -> dict:
        taken, selection = self._capture_gesture(body)
        member = selection["member"]
        return {
            "member": member.member,
            "gesture": selection["gesture"],
            "capture_id": self.workbench.capture_id(taken),
            "observed": selection["observed"],
            **resolve(member, selection["cells"]),
        }

    def _record_capture(self, body: dict) -> dict:
        taken, selection = self._capture_gesture(body)
        member = selection["member"]
        gesture = selection["gesture"]
        session = self.workbench.session
        packet = build(
            projection=self.workbench.projection,
            member=member,
            gesture=gesture,
            cells=selection["cells"],
            screen_region=capture_reader.canvas_rect(gesture, selection["geometry"]),
            comment=str(body.get("comment", "")),
            selection_id=session.next_selection_id(),
            created_at=now(),
            repository_revision=session.manifest.get("repository_revision"),
            mask_reference=None,
            geometry=selection["geometry"],
            capture=taken.binding(self.workbench.root, selection["observed"]),
        )
        mask = (
            mask_bytes(member, selection["cells"]) if gesture in MASKED_GESTURES else None
        )
        packet = session.write_selection(packet, mask)
        if packet["comment"]:
            session.write_comment(packet["selection_id"], packet["comment"])
        return {"packet": packet, "session_directory": session.relative}

    # -- V1: staging, the candidate, and Apply ----------------------------

    def _vocabulary(self) -> None:
        """The whole operation vocabulary, from the two places that own its halves.

        This route runs the compiler. It is called when the staging panel opens
        and not on the selection path, because the picker needs the real verb
        table and a table the browser carried would be a copy to keep true.
        """
        try:
            truth = bridge.describe_operations(
                self.workbench.root, self.workbench.projection.land_id
            )
        except WorkbenchError as error:
            self._error(HTTPStatus.SERVICE_UNAVAILABLE, "compiler_unavailable", str(error))
            return
        self._json(
            HTTPStatus.OK,
            {
                "truth": truth,
                "dressing": {"verbs": [], "ruling": operation_log.DRESSING_RULING},
                "asset": {
                    "verbs": [
                        {
                            "verb": contract.name,
                            "summary": contract.summary,
                            "required": list(contract.required),
                            "optional": list(contract.optional),
                            "served": contract.name in _SERVED_ASSET_VERBS,
                        }
                        for contract in imageops.CONTRACTS.values()
                    ],
                    "adapters": sorted(imageops.REGISTRY),
                },
            },
        )

    def _stage(self, body: dict) -> dict:
        session = self.workbench.session
        record = operation_log.build(
            record_id=session.next_record_id(),
            recorded_at=now(),
            author=str(body.get("author", "owner")),
            selection_id=str(body["selection_id"]),
            operation_class=str(body.get("class", operation_log.CLASS_TRUTH)),
            member=str(body.get("member", session.candidate_member)),
            editable_member=session.candidate_member,
            verb=str(body["verb"]),
            parameters=dict(body.get("parameters") or {}),
            adapter=body.get("adapter"),
            comment=str(body.get("comment", "")),
        )
        session.read_packet(record["selection_id"])
        session.stage_operation(record)
        return {"record": record, "staged": operation_log.summary(session.staged())}

    def _retract(self, body: dict) -> dict:
        session = self.workbench.session
        record = session.retract_operation(
            str(body["record_id"]),
            str(body.get("reason", "")),
            str(body.get("author", "owner")),
        )
        return {"record": record, "staged": operation_log.summary(session.staged())}

    def _resolve_candidate_gesture(self, body: dict) -> dict:
        """A gesture over the candidate view, resolved in the candidate's own frame.

        The lattice is the same lattice — a candidate is an edit to a member, not
        a different member — so the CELLS a gesture covers are the same either
        way. What differs is what occupies them, and that is the whole reason to
        point at a candidate at all.

        No packet is written from here, and the interface says why: a packet
        binds the exact bytes it was taken against, and a candidate's bytes are
        replaced by the next preview. A packet bound to them would be stale by
        design, which is worse than no packet.
        """
        if self.workbench.candidate is None:
            raise WorkbenchError(
                "there is no candidate to point at; preview the staged set first"
            )
        candidate = self.workbench.candidate
        member = candidate.member(str(body["member"]))
        gesture = str(body["gesture"])
        cells = cells_for_gesture(member, gesture, body)
        return {
            "member": member.member,
            "gesture": gesture,
            "binds": [source.as_record() for source in candidate.sources],
            **resolve(member, cells),
        }

    def _candidate(self) -> dict:
        """Replay the staged set and hand back the candidate's own logical view.

        The view is the compiler's, derived from the candidate document, so the
        browser draws the candidate exactly the way it draws the accepted land —
        one renderer, one projection shape, and nothing approximated. Runs the
        compiler; not on the selection path.
        """
        outcome = replay_module.preview(self.workbench.session)
        document = None
        self.workbench.candidate = None
        if outcome.projection:
            self.workbench.candidate = load_candidate(
                self.workbench.root, outcome.projection["path"]
            )
            document = self.workbench.candidate.document
        return {"outcome": outcome.as_record(), "projection": document}

    def _apply(self, body: dict) -> dict:
        applied = apply_module.apply(
            self.workbench.session, author=str(body.get("author", "owner"))
        )
        return {
            "accepted": applied.accepted,
            "apply_id": applied.apply_id,
            "path": applied.path,
            # Verbatim. The owner reads the receipt or the rejection as it was
            # written, never a summary of it.
            "record": applied.record,
        }

    def _accept(self, body: dict) -> dict:
        """Record the owner's acceptance of a candidate — as intent, nothing more."""
        record = self.workbench.session.record_candidate_acceptance(
            candidate_sha256=str(body["candidate_sha256"]),
            apply_id=str(body["apply_id"]),
            note=str(body.get("note", "")),
            author=str(body.get("author", "owner")),
        )
        return {"record": record}

    def _record(self, body: dict) -> dict:
        session = self.workbench.session
        packet = session.record_logical_selection(self.workbench.projection, body)
        return {"packet": packet, "session_directory": session.relative}


def serve(root: Path, projection_path: str, host: str, port: int, session_id: str | None,
          capture_configuration: dict | None = None) -> int:
    try:
        workbench = Workbench(root, projection_path, session_id, capture_configuration)
    except ProjectionUnavailable as error:
        print(f"UNSERVABLE: {error}", file=sys.stderr)
        return EXIT_UNSERVABLE
    try:
        workbench.check()
    except StaleSelection as stale:
        print(f"UNSERVABLE: {stale}", file=sys.stderr)
        return EXIT_UNSERVABLE

    handler = type("BoundHandler", (Handler,), {"workbench": workbench})
    server = ThreadingHTTPServer((host, port), handler)
    server.daemon_threads = True
    bound_host, bound_port = server.server_address[:2]
    print(f"workbench {VERSION}: logical view of {workbench.projection.land_id}")
    print(f"  serving   http://{bound_host}:{bound_port}/")
    print(f"  session   {workbench.session.relative}")
    print(f"  bound to  {len(workbench.projection.sources)} source digests, fail-closed")
    sys.stdout.flush()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nworkbench: stopped")
    finally:
        server.server_close()
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root (default: this checkout)")
    parser.add_argument("--projection", default=DEFAULT_PROJECTION_PATH)
    parser.add_argument("--host", default=DEFAULT_HOST, help="loopback address to bind")
    parser.add_argument("--port", type=int, default=8730, help="0 picks a free port")
    parser.add_argument("--session", default=None, help="session id (default: a new one)")
    capture_source = parser.add_mutually_exclusive_group()
    capture_source.add_argument("--capture-world", help="served-world document for scratch authoritative browser capture")
    capture_source.add_argument("--capture-replay", type=Path, help="existing browser capture directory to replay")
    parser.add_argument("--admin-url-file", default=os.environ.get("TME_PG_ADMIN_URL_FILE"))
    arguments = parser.parse_args(argv)
    root = Path(arguments.root) if arguments.root else Path(__file__).resolve().parents[2]
    try:
        if not ip_address(arguments.host).is_loopback:
            print("UNSERVABLE: --host must be a loopback address", file=sys.stderr)
            return EXIT_UNSERVABLE
    except ValueError:
        print("UNSERVABLE: --host must be a loopback address literal", file=sys.stderr)
        return EXIT_UNSERVABLE
    configuration = None
    if arguments.capture_world or arguments.capture_replay:
        configuration = {"world_document": arguments.capture_world, "replay_directory": arguments.capture_replay,
                         "admin_url_file": arguments.admin_url_file}
    return serve(root, arguments.projection, arguments.host, arguments.port, arguments.session, configuration)


if __name__ == "__main__":
    sys.exit(main())
