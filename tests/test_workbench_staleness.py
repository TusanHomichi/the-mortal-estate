"""Acceptance criterion 4 — staleness fails closed, provably, per digest.

A selection packet binds five files. Each is mutated **independently** and each
mutant must kill the packet on its own, because a fail-closed path that is only
ever exercised through one of its inputs is four assumptions and one guarantee.

The mutation used is a whitespace-only reformat: the document still parses and
still says the same thing. Only the digest moves. A consumer that survives that
is a consumer that would happily resolve a packet against a world that had been
edited underneath it.

Both consumers are proven: the agent-facing `resolve.py`, run as a program, and
the local server, over real HTTP.
"""

from __future__ import annotations

import json
import threading
import unittest
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer

from workbench_test_support import BOUND_FILES, StagedTree, run_resolve

from workbench import serve
from workbench.packet import build, cells_for_gesture, mask_bytes, now, resolution_of
from workbench.projection import (
    DEFAULT_PROJECTION_PATH,
    StaleSelection,
    WorkbenchError,
    verify,
)
from workbench.session import open_session


class StalenessFailsClosed(StagedTree):
    def setUp(self) -> None:
        super().setUp()
        self.projection = self.staged_projection()
        self.member = self.projection.member("surface")
        self.session = open_session(self.projection, "session-staleness")
        # Two packets, exactly as the running Workbench writes them: a box
        # carries no mask, a lasso does.
        self.packet_path = self.write("sel-0001", "box", {
            "rect": {"x": 8, "y": 6, "width": 2, "height": 2}
        }, masked=False)
        self.masked_path = self.write("sel-0002", "lasso", {
            "polygon": [
                {"x": 8.0, "y": 6.0}, {"x": 10.0, "y": 6.0},
                {"x": 10.0, "y": 8.0}, {"x": 8.0, "y": 8.0},
            ]
        }, masked=True)

    def write(self, selection_id: str, gesture: str, payload: dict, masked: bool):
        cells = cells_for_gesture(self.member, gesture, payload)
        packet = build(
            projection=self.projection,
            member=self.member,
            gesture=gesture,
            cells=cells,
            screen_region=None,
            comment="the packet under test",
            selection_id=selection_id,
            created_at=now(),
            repository_revision=None,
            mask_reference=None,
            geometry=payload,
        )
        self.session.write_selection(packet, mask_bytes(self.member, cells) if masked else None)
        return self.staged / self.session.relative / f"selections/{selection_id}.json"

    def test_every_bound_digest_kills_the_packet_on_its_own(self) -> None:
        for relative in BOUND_FILES:
            with self.subTest(moved=relative):
                self.setUp()  # a fresh staged tree per mutant
                self.corrupt(relative)
                with self.assertRaises(StaleSelection) as refusal:
                    verify(self.staged, self.projection.sources)
                moved = refusal.exception.moved
                self.assertEqual([entry["path"] for entry in moved], [relative])
                self.assertIn(relative, str(refusal.exception))
                self.assertIn("digest moved", str(refusal.exception))

    def test_the_agent_consumer_refuses_each_mutant_by_name(self) -> None:
        for relative in BOUND_FILES:
            with self.subTest(moved=relative):
                self.setUp()
                self.corrupt(relative)
                completed = run_resolve(self.packet_path, self.staged)
                self.assertEqual(completed.returncode, 2, completed.stdout)
                self.assertIn("REFUSED", completed.stderr)
                self.assertIn(relative, completed.stderr)
                self.assertNotIn("identities", completed.stdout)

    def test_a_deleted_source_is_a_refusal_and_says_so(self) -> None:
        (self.staged / BOUND_FILES[0]).unlink()
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("is missing", completed.stderr)

    def test_an_unmutated_tree_resolves(self) -> None:
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("VERIFIED", completed.stdout)

    def test_a_mask_edited_after_the_fact_is_a_refusal(self) -> None:
        """The mask is part of the address, so it is verified like the rest of it."""
        mask = self.staged / self.session.relative / "masks/sel-0002.pbm"
        mask.write_bytes(mask.read_bytes().replace(b"1", b"0", 1))
        completed = run_resolve(self.masked_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("mask", completed.stderr)
        self.assertIn("digest moved", completed.stderr)

    def test_a_hand_edited_packet_is_a_refusal(self) -> None:
        """Digests bind the world; re-resolution binds the packet to that world."""
        packet = json.loads(self.packet_path.read_text())
        packet["semantic"][0]["identity"] = "structure:surface:fixture_structure_south"
        self.packet_path.write_text(json.dumps(packet, indent=2))
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("disagree with the current projection", completed.stderr)

    def test_a_packet_naming_a_cell_the_gesture_did_not_cover_is_a_refusal(self) -> None:
        """The gesture is re-derived, so an added cell is caught before anything
        else looks at it — including a cell the member does not carry at all."""
        packet = json.loads(self.packet_path.read_text())
        packet["cells"].append({"x": 99, "y": 99})
        self.packet_path.write_text(json.dumps(packet, indent=2))
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2, completed.stdout)
        self.assertIn("re-deriving the gesture names different cells", completed.stderr)

    def test_the_member_envelope_is_still_a_refusal_in_its_own_right(self) -> None:
        """The second line of defence, exercised directly: the resolver refuses a
        cell its member does not carry, whatever route reached it."""
        packet = json.loads(self.packet_path.read_text())
        packet["cells"].append({"x": 99, "y": 99})
        with self.assertRaises(WorkbenchError) as refusal:
            resolution_of(self.projection, packet)
        self.assertIn("does not carry", str(refusal.exception))


class TheServerRefusesToo(StagedTree):
    """The other consumer, over real HTTP, mutant by mutant."""

    def setUp(self) -> None:
        super().setUp()
        workbench = serve.Workbench(
            self.staged, DEFAULT_PROJECTION_PATH, "session-http"
        )
        handler = type(
            "BoundHandler",
            (serve.Handler,),
            {"workbench": workbench, "log_message": lambda *_args, **_kwargs: None},
        )
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.server.daemon_threads = True
        self.addCleanup(self.server.server_close)
        thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(self.server.shutdown)
        host, port = self.server.server_address[:2]
        self.base = f"http://{host}:{port}"

    def get(self, path: str):
        try:
            with urllib.request.urlopen(self.base + path, timeout=10) as response:
                return response.status, json.loads(response.read())
        except urllib.error.HTTPError as error:
            with error as response:
                return response.code, json.loads(response.read())

    def test_each_moved_digest_makes_the_server_refuse_until_it_is_restored(self) -> None:
        status, _ = self.get("/api/state")
        self.assertEqual(status, 200)
        for relative in BOUND_FILES:
            with self.subTest(moved=relative):
                original = (self.staged / relative).read_bytes()
                self.corrupt(relative)
                status, payload = self.get("/api/state")
                self.assertEqual(status, 409)
                self.assertEqual(payload["error"], "stale")
                self.assertIn(relative, payload["detail"])
                (self.staged / relative).write_bytes(original)
                status, _ = self.get("/api/state")
                self.assertEqual(status, 200, "restoring the bytes must restore service")

    def test_a_stale_tree_refuses_selections_as_well_as_reads(self) -> None:
        self.corrupt(BOUND_FILES[0])
        request = urllib.request.Request(
            self.base + "/api/selection",
            data=json.dumps(
                {"member": "surface", "gesture": "click", "cell": {"x": 8, "y": 6}}
            ).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with self.assertRaises(urllib.error.HTTPError) as refusal:
            urllib.request.urlopen(request, timeout=10)
        with refusal.exception as response:
            self.assertEqual(response.code, 409)
            self.assertEqual(json.loads(response.read())["error"], "stale")


if __name__ == "__main__":
    unittest.main()
