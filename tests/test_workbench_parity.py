"""Acceptance criterion 6 — agent parity holds, demonstrated end to end.

The claim is not "both paths call the same function". It is that an agent, with
nothing but the documented file layout and ordinary file tools, arrives at the
identical answer the browser application is looking at. So this test drives the
real HTTP server for the browser's answer and runs `resolve.py` as a separate
process for the agent's, and requires the two to be equal.

The tracked synthetic session fixture is used as well as a live session,
because an agent handed a session directory from somewhere else — a colleague,
a bug report, a week-old checkout — must be able to read it cold.
"""

from __future__ import annotations

import json
import threading
import unittest
import urllib.request
from http.server import ThreadingHTTPServer

from workbench_test_support import (
    FIXTURE_ROOT,
    SYNTHETIC_PROJECTION,
    StagedTree,
    resolve_json,
    run_resolve,
    synthetic_projection,
)

from workbench import serve
from workbench.packet import resolution_of
from workbench.projection import DEFAULT_PROJECTION_PATH


class BrowserAndAgentAgree(StagedTree):
    def setUp(self) -> None:
        super().setUp()
        self.workbench = serve.Workbench(self.staged, DEFAULT_PROJECTION_PATH, "session-parity")
        handler = type(
            "BoundHandler",
            (serve.Handler,),
            {"workbench": self.workbench, "log_message": lambda *_a, **_k: None},
        )
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.server.daemon_threads = True
        self.addCleanup(self.server.server_close)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.addCleanup(self.server.shutdown)
        host, port = self.server.server_address[:2]
        self.base = f"http://{host}:{port}"

    def post(self, path: str, body: dict) -> dict:
        request = urllib.request.Request(
            self.base + path,
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.loads(response.read())

    def get(self, path: str) -> dict:
        with urllib.request.urlopen(self.base + path, timeout=10) as response:
            return json.loads(response.read())

    def take(self, body: dict) -> dict:
        return self.post("/api/selection", body)["packet"]

    def packet_path(self, selection_id: str):
        return self.staged / self.workbench.session.relative / f"selections/{selection_id}.json"

    def assert_parity(self, packet: dict) -> None:
        selection_id = packet["selection_id"]
        served = self.get(f"/api/packet?id={selection_id}")
        agent = resolve_json(self.packet_path(selection_id), self.staged)
        self.assertEqual(agent["resolution"], served["resolution"])
        # And the packet's own recorded identities are that same answer, so a
        # reader who never runs anything still sees the truth.
        self.assertEqual(packet["semantic"], served["resolution"]["semantic"])
        self.assertEqual(packet["ambiguous"], served["resolution"]["ambiguous"])
        self.assertEqual(packet["candidates"], served["resolution"]["candidates"])

    def test_a_click_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            self.take({"member": "surface", "gesture": "click", "cell": {"x": 8, "y": 6}})
        )

    def test_a_box_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            self.take(
                {
                    "member": "surface",
                    "gesture": "box",
                    "rect": {"x": 8, "y": 6, "width": 7, "height": 3},
                }
            )
        )

    def test_a_lasso_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            self.take(
                {
                    "member": "surface",
                    "gesture": "lasso",
                    "polygon": [
                        {"x": 11.0, "y": 8.0},
                        {"x": 15.0, "y": 8.0},
                        {"x": 15.0, "y": 11.0},
                        {"x": 11.0, "y": 11.0},
                    ],
                }
            )
        )

    def test_a_paint_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            self.take(
                {
                    "member": "surface",
                    "gesture": "paint",
                    "cells": [{"x": 8, "y": 8}, {"x": 9, "y": 8}, {"x": 10, "y": 8}],
                }
            )
        )

    def test_the_preview_and_the_recorded_packet_are_the_same_answer(self) -> None:
        """Previewing must never differ from recording, or the owner records a surprise."""
        body = {"member": "surface", "gesture": "click", "cell": {"x": 12, "y": 14}}
        preview = self.post("/api/preview", dict(body))
        packet = self.take(dict(body))
        self.assertEqual(packet["semantic"], preview["semantic"])
        self.assertEqual(packet["ambiguous"], preview["ambiguous"])
        self.assertEqual(packet["cells"], preview["cells"])

    def test_an_agent_can_list_and_read_a_session_without_the_server(self) -> None:
        for cell in ({"x": 8, "y": 6}, {"x": 12, "y": 14}):
            self.take({"member": "surface", "gesture": "click", "cell": cell})
        directory = self.staged / self.workbench.session.relative
        packets = sorted((directory / "selections").glob("*.json"))
        self.assertEqual([path.stem for path in packets], ["sel-0001", "sel-0002"])
        for path in packets:
            answer = resolve_json(path, self.staged)
            self.assertEqual(answer["selection_id"], path.stem)
            self.assertEqual(len(answer["verified_digests"]), 5)


class AnAgentReadsTheTrackedFixtureCold(unittest.TestCase):
    """No server ever ran here, and no ignored root is involved (the D6 ruling)."""

    def test_every_tracked_packet_resolves_from_files_alone(self) -> None:
        projection = synthetic_projection()
        for path in sorted((FIXTURE_ROOT / "session/selections").glob("*.json")):
            with self.subTest(packet=path.name):
                answer = resolve_json(path, FIXTURE_ROOT)
                packet = json.loads(path.read_text())
                self.assertEqual(answer["resolution"]["semantic"], packet["semantic"])
                self.assertEqual(
                    answer["resolution"], resolution_of(projection, packet)
                )

    def test_the_fixture_binds_the_synthetic_projection_it_names(self) -> None:
        projection = synthetic_projection()
        self.assertEqual(projection.path, SYNTHETIC_PROJECTION)
        packet = json.loads(
            (FIXTURE_ROOT / "session/selections/sel-0001.json").read_text()
        )
        self.assertEqual(packet["source"]["digests"], projection.source_records())

    def test_a_packet_read_from_the_fixture_resolves_without_a_root_argument_failing_safely(
        self,
    ) -> None:
        """Pointed at the wrong tree, the consumer refuses rather than guesses."""
        completed = run_resolve(
            FIXTURE_ROOT / "session/selections/sel-0001.json", FIXTURE_ROOT.parents[1]
        )
        self.assertEqual(completed.returncode, 2, completed.stdout)
        self.assertIn("REFUSED", completed.stderr)


if __name__ == "__main__":
    unittest.main()
