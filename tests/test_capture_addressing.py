"""Acceptance criteria 2, 4, 6 and 8, for selections taken over a capture.

**Criterion 2** is the whole point of the slice: a selection over a real capture
must resolve to the cell and semantic identities the equivalent selection over
the logical view produces. It is proven twice — once against the logical view,
and once between the two capture routes, whose pictures frame the same land at
different scales. Two framings, one address space, or the claim is empty.

**Criterion 4** extends to the three files a capture binds. Each is mutated on
its own, because a fail-closed path exercised through one of its inputs is two
assumptions and one guarantee. The packet's own re-derivable parts — its cells,
its observed identities, its gesture — are edited too, since a capture packet
carries more that a careless hand could change.

**Criterion 6** extends to capture packets: the browser's answer and an agent's,
compared over real HTTP against a real subprocess, for every gesture.

**Criterion 8** extends to capture sessions: a session that selects over a
capture writes only into the ignored session root, and this repository's `git
status` is identical before and after.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import threading
import unittest
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

from workbench_test_support import (
    BOUND_FILES,
    CAPTURE_FILES,
    FIXTURE_ROUTE,
    REPO_ROOT,
    StagedTree,
    accepted_projection,
    capture_packet,
    fixture_route_capture,
    live_route_capture,
    region_rect,
    resolve_json,
    run_resolve,
    square_rect,
)

from workbench import capture as capture_reader
from workbench import capture_harness, serve
from workbench.identity import resolve as resolve_identities
from workbench.packet import cells_for_gesture
from workbench.projection import DEFAULT_PROJECTION_PATH
from workbench.session import open_session

#: A block of squares away from the observation centre, so the comparison is not
#: quietly about one cell that happens to be simple.
REGION = (10, 10, 11, 11)

#: The arrival square: a landmark standing on a route, which is the fixture's
#: genuinely ambiguous cell. If the two views ever disagreed, they would disagree
#: here first.
ARRIVAL = (12, 14)


def logical(member, gesture: str, payload: dict) -> dict:
    """The same answer the logical view gives, through the one resolver."""
    return resolve_identities(member, cells_for_gesture(member, gesture, payload))


def address(answer: dict) -> tuple:
    """The part of an answer that must be identical across views."""
    return (answer["cells"], answer["semantic"], answer["candidates"], answer["ambiguous"])


def identities(records) -> list:
    """Observed records reduced to what does not depend on framing."""
    return sorted(
        (row["identity"], row["kind"], row["coordinate"]["x"], row["coordinate"]["y"])
        for row in records
    )


class ACaptureSelectionResolvesLikeALogicalOne(unittest.TestCase):
    """Criterion 2, against the logical view, gesture by gesture."""

    def setUp(self) -> None:
        self.projection = accepted_projection()
        self.taken = fixture_route_capture()
        self.member = self.projection.member("surface")

    def over_capture(self, gesture: str, geometry: dict) -> dict:
        selection = capture_reader.select(self.projection, self.taken, gesture, geometry)
        return {
            **resolve_identities(selection["member"], selection["cells"]),
            "observed": selection["observed"],
        }

    def test_a_click_on_a_square_resolves_to_that_square(self) -> None:
        anchor = [
            row["anchor"] for row in self.taken.targets
            if row["identity"] == f"tile:{ARRIVAL[0]}:{ARRIVAL[1]}"
        ][0]
        capture = self.over_capture("click", {"point": anchor})
        self.assertEqual(
            address(capture),
            address(logical(self.member, "click", {"cell": {"x": ARRIVAL[0], "y": ARRIVAL[1]}})),
        )
        self.assertTrue(capture["ambiguous"], "the arrival square is genuinely ambiguous")

    def test_a_click_on_an_occupant_resolves_to_the_square_it_stands_on(self) -> None:
        """The marker is a different pixel and the same address.

        This is the case a nearest-anchor scheme gets wrong: clicking a marker
        must name the land under it, and say separately what was standing there.
        """
        occupant = [
            row for row in self.taken.targets if row["presentation_layer"] == "occupants"
        ][0]
        square = occupant["coordinate"]
        capture = self.over_capture("click", {"point": occupant["anchor"]})
        self.assertEqual(
            address(capture),
            address(logical(self.member, "click", {"cell": dict(square)})),
        )
        self.assertEqual(
            [row["identity"] for row in capture["observed"]], [occupant["identity"]]
        )

    def test_a_box_over_a_region_resolves_to_that_region(self) -> None:
        rect = region_rect(self.taken, *REGION)
        capture = self.over_capture("box", {"rect": rect})
        expected = logical(
            self.member,
            "box",
            {
                "rect": {
                    "x": REGION[0],
                    "y": REGION[1],
                    "width": REGION[2] - REGION[0] + 1,
                    "height": REGION[3] - REGION[1] + 1,
                }
            },
        )
        self.assertEqual(address(capture), address(expected))
        self.assertEqual(len(capture["cells"]), 4)

    def test_a_lasso_over_a_region_resolves_to_that_region(self) -> None:
        rect = region_rect(self.taken, *REGION)
        polygon = [
            {"x": rect["x"], "y": rect["y"]},
            {"x": rect["x"] + rect["width"], "y": rect["y"]},
            {"x": rect["x"] + rect["width"], "y": rect["y"] + rect["height"]},
            {"x": rect["x"], "y": rect["y"] + rect["height"]},
        ]
        capture = self.over_capture("lasso", {"polygon": polygon})
        expected = logical(
            self.member,
            "box",
            {
                "rect": {
                    "x": REGION[0],
                    "y": REGION[1],
                    "width": REGION[2] - REGION[0] + 1,
                    "height": REGION[3] - REGION[1] + 1,
                }
            },
        )
        self.assertEqual(address(capture), address(expected))

    def test_a_paint_across_squares_resolves_to_those_squares(self) -> None:
        squares = [(10, 10), (11, 10), (12, 10)]
        points = [
            {"x": square_rect(self.taken, x, y)["x"] + 4, "y": square_rect(self.taken, x, y)["y"] + 4}
            for x, y in squares
        ]
        capture = self.over_capture("paint", {"points": points})
        expected = logical(
            self.member, "paint", {"cells": [{"x": x, "y": y} for x, y in squares]}
        )
        self.assertEqual(address(capture), address(expected))

    def test_a_gesture_that_lands_on_no_target_is_refused_rather_than_rounded(self) -> None:
        from workbench.projection import WorkbenchError

        with self.assertRaises(WorkbenchError) as refusal:
            self.over_capture("click", {"point": {"x": 0, "y": 0}})
        self.assertIn("no addressable pixel", str(refusal.exception))


class TheTwoCaptureRoutesResolveIdentically(unittest.TestCase):
    """Criterion 2 again, between the cheap route and the accuracy reference.

    The two captures show one frame of one land at two framings — the live one
    inside the world shell, inset around the HUD, and the fixture one with the
    window to itself. Every pixel differs. No address may.
    """

    def setUp(self) -> None:
        self.projection = accepted_projection()
        self.fixture = fixture_route_capture()
        self.live = live_route_capture()

    def test_the_two_routes_photographed_different_pictures(self) -> None:
        """Otherwise the equality below would be a tautology."""
        self.assertNotEqual(self.fixture.image_digest, self.live.image_digest)
        self.assertNotEqual(
            self.fixture.camera["square_pitch_px"], self.live.camera["square_pitch_px"]
        )

    def test_the_two_routes_show_the_same_frame_of_the_same_land(self) -> None:
        self.assertEqual(self.fixture.frame_generation, self.live.frame_generation)
        self.assertEqual(self.fixture.level, self.live.level)
        self.assertEqual(self.fixture.realm, self.live.realm)
        self.assertEqual(
            identities(self.fixture.targets), identities(self.live.targets),
            "the two routes address exactly the same things",
        )

    def test_the_same_region_resolves_to_the_same_address_in_both(self) -> None:
        answers = []
        for taken in (self.fixture, self.live):
            selection = capture_reader.select(
                self.projection, taken, "box", {"rect": region_rect(taken, *REGION)}
            )
            answers.append(
                (
                    address(resolve_identities(selection["member"], selection["cells"])),
                    identities(selection["observed"]),
                )
            )
        self.assertEqual(answers[0], answers[1])

    def test_the_same_square_resolves_to_the_same_address_in_both(self) -> None:
        answers = []
        for taken in (self.fixture, self.live):
            anchor = [
                row["anchor"] for row in taken.targets
                if row["identity"] == f"tile:{ARRIVAL[0]}:{ARRIVAL[1]}"
            ][0]
            selection = capture_reader.select(
                self.projection, taken, "click", {"point": anchor}
            )
            answers.append(
                (
                    address(resolve_identities(selection["member"], selection["cells"])),
                    identities(selection["observed"]),
                )
            )
        self.assertEqual(answers[0], answers[1])


class ACapturePacketFailsClosedPerDigest(StagedTree):
    """Criterion 4, extended to everything a capture packet binds."""

    staged_files = BOUND_FILES + CAPTURE_FILES

    def setUp(self) -> None:
        super().setUp()
        self.projection = self.staged_projection()
        self.taken = capture_reader.load(self.staged, self.staged / FIXTURE_ROUTE)
        self.session = open_session(self.projection, "session-capture-staleness")
        anchor = [
            row["anchor"] for row in self.taken.targets
            if row["identity"] == f"tile:{ARRIVAL[0]}:{ARRIVAL[1]}"
        ][0]
        packet = capture_packet(
            self.projection,
            self.taken,
            "click",
            {"point": anchor},
            root=self.staged,
            comment="the capture packet under test",
        )
        self.session.write_selection(packet, None)
        self.packet_path = self.staged / self.session.relative / "selections/sel-0001.json"

    def edit(self, mutate) -> None:
        packet = json.loads(self.packet_path.read_text(encoding="utf-8"))
        mutate(packet)
        self.packet_path.write_text(json.dumps(packet, indent=2), encoding="utf-8")

    def test_the_packet_binds_all_eight_sources(self) -> None:
        packet = json.loads(self.packet_path.read_text(encoding="utf-8"))
        self.assertEqual(
            [record["role"] for record in packet["source"]["digests"]],
            [
                "master",
                "companion",
                "receipt",
                "runtime_projection",
                "logical_projection",
                "capture_image",
                "capture_sidecar",
                "capture_identity_raster",
            ],
        )
        self.assertEqual(packet["view"], "capture")
        self.assertEqual(packet["scene"]["frame_generation"], self.taken.frame_generation)
        self.assertIsNotNone(packet["camera"])
        self.assertIsNotNone(packet["context_image"])

    def test_an_unmutated_tree_resolves(self) -> None:
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("VERIFIED, 8 digests", completed.stdout)
        self.assertIn("capture     ", completed.stdout)

    def test_every_bound_file_kills_the_packet_on_its_own(self) -> None:
        for relative in BOUND_FILES + CAPTURE_FILES:
            with self.subTest(moved=relative):
                self.setUp()  # a fresh staged tree per mutant
                self.corrupt(relative)
                completed = run_resolve(self.packet_path, self.staged)
                self.assertEqual(completed.returncode, 2, completed.stdout)
                self.assertIn("REFUSED", completed.stderr)
                self.assertIn(relative, completed.stderr)
                self.assertNotIn("identities", completed.stdout)

    def test_a_deleted_capture_file_is_a_refusal_and_says_so(self) -> None:
        (self.staged / CAPTURE_FILES[0]).unlink()
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("is missing", completed.stderr)

    def test_an_edited_cell_list_is_a_refusal(self) -> None:
        self.edit(lambda packet: packet["cells"].append({"x": 1, "y": 1}))
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("different cells", completed.stderr)

    def test_an_edited_observed_list_is_a_refusal(self) -> None:
        self.edit(lambda packet: packet["observed"].clear())
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("different observed identities", completed.stderr)

    def test_an_edited_gesture_geometry_is_a_refusal(self) -> None:
        def move_the_gesture(packet: dict) -> None:
            packet["screen_region"]["geometry"]["point"] = {"x": 20, "y": 300}

        self.edit(move_the_gesture)
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("different cells", completed.stderr)

    def test_an_edited_frame_generation_is_a_refusal(self) -> None:
        self.edit(lambda packet: packet["scene"].__setitem__("frame_generation", 99))
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("frame generation", completed.stderr)

    def test_an_edited_camera_is_a_refusal(self) -> None:
        self.edit(lambda packet: packet["camera"].__setitem__("square_pitch_px", 7))
        completed = run_resolve(self.packet_path, self.staged)
        self.assertEqual(completed.returncode, 2)
        self.assertIn("camera identity", completed.stderr)


class AgentParityHoldsForCapturePackets(StagedTree):
    """Criterion 6, over the real server and a real subprocess."""

    staged_files = BOUND_FILES + CAPTURE_FILES

    def setUp(self) -> None:
        super().setUp()
        self.workbench = serve.Workbench(
            self.staged, DEFAULT_PROJECTION_PATH, "session-capture-parity"
        )
        # A session carries its own captures; this one is handed the tracked
        # capture rather than taking a new one, because parity is a claim about
        # resolution, not about the client binary being installed.
        destination = (
            self.staged
            / self.workbench.session.relative
            / capture_reader.CAPTURES_DIR
            / "cap-0001"
        )
        shutil.copytree(self.staged / FIXTURE_ROUTE, destination)
        self.workbench._reload_captures()
        self.assertEqual(list(self.workbench.captures), ["cap-0001"])

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
        self.taken = self.workbench.captures["cap-0001"]

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

    def anchor_of(self, x: int, y: int) -> dict:
        return [
            row["anchor"] for row in self.taken.targets if row["identity"] == f"tile:{x}:{y}"
        ][0]

    def assert_parity(self, body: dict) -> dict:
        preview = self.post("/api/capture/preview", dict(body))
        packet = self.post("/api/capture/selection", dict(body))["packet"]
        selection_id = packet["selection_id"]
        served = self.get(f"/api/packet?id={selection_id}")
        path = (
            self.staged
            / self.workbench.session.relative
            / f"selections/{selection_id}.json"
        )
        agent = resolve_json(path, self.staged)
        self.assertEqual(agent["resolution"], served["resolution"])
        self.assertEqual(packet["semantic"], served["resolution"]["semantic"])
        self.assertEqual(packet["semantic"], preview["semantic"])
        self.assertEqual(packet["cells"], preview["cells"])
        self.assertEqual(agent["observed"], packet["observed"])
        self.assertEqual(agent["view"], "capture")
        return packet

    def test_a_capture_click_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            {"capture_id": "cap-0001", "gesture": "click", "point": self.anchor_of(*ARRIVAL)}
        )

    def test_a_capture_box_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        packet = self.assert_parity(
            {
                "capture_id": "cap-0001",
                "gesture": "box",
                "rect": region_rect(self.taken, *REGION),
            }
        )
        self.assertEqual(len(packet["cells"]), 4)

    def test_a_capture_lasso_carries_a_mask_like_a_logical_one(self) -> None:
        rect = region_rect(self.taken, *REGION)
        packet = self.assert_parity(
            {
                "capture_id": "cap-0001",
                "gesture": "lasso",
                "polygon": [
                    {"x": rect["x"], "y": rect["y"]},
                    {"x": rect["x"] + rect["width"], "y": rect["y"]},
                    {"x": rect["x"] + rect["width"], "y": rect["y"] + rect["height"]},
                    {"x": rect["x"], "y": rect["y"] + rect["height"]},
                ],
            }
        )
        self.assertIsNotNone(packet["screen_region"]["mask"])

    def test_a_capture_paint_resolves_identically_for_the_browser_and_an_agent(self) -> None:
        self.assert_parity(
            {
                "capture_id": "cap-0001",
                "gesture": "paint",
                "points": [self.anchor_of(10, 10), self.anchor_of(11, 10)],
            }
        )

    def test_the_state_endpoint_offers_the_capture_with_its_digests(self) -> None:
        state = self.get("/api/state")
        self.assertEqual([row["capture_id"] for row in state["captures"]], ["cap-0001"])
        summary = state["captures"][0]
        self.assertEqual(summary["member"], "surface")
        self.assertEqual(summary["viewport"], self.taken.viewport)
        self.assertEqual(
            [record["role"] for record in summary["digests"]],
            ["capture_image", "capture_sidecar", "capture_identity_raster"],
        )

    def test_the_picture_is_served_as_the_bytes_the_sidecar_names(self) -> None:
        with urllib.request.urlopen(
            self.base + "/api/capture/image?id=cap-0001", timeout=10
        ) as response:
            payload = response.read()
            self.assertEqual(response.headers["Content-Type"], "image/png")
        from workbench.projection import digest_bytes

        self.assertEqual(digest_bytes(payload), self.taken.image_digest)

    def test_a_broken_capture_is_withheld_with_its_reason(self) -> None:
        directory = (
            self.staged
            / self.workbench.session.relative
            / capture_reader.CAPTURES_DIR
            / "cap-0001"
        )
        payload = bytearray((directory / "capture.png").read_bytes())
        payload[-1] ^= 0x01
        (directory / "capture.png").write_bytes(bytes(payload))
        self.workbench._reload_captures()
        self.assertEqual(self.workbench.captures, {})
        self.assertIn("cap-0001", self.workbench.broken_captures)
        state = self.get("/api/state")
        self.assertEqual(state["captures"], [])
        self.assertIn("cap-0001", state["broken_captures"])


class NothingCanonicalMovesWhenSelectingOverACapture(unittest.TestCase):
    """Criterion 8, for the capture half, in this repository."""

    def test_a_capture_session_leaves_this_tree_as_it_was(self) -> None:
        before = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "status", "--porcelain"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        projection = accepted_projection()
        taken = fixture_route_capture()
        session = open_session(projection, "session-capture-criterion-8")
        directory = REPO_ROOT / session.relative
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)

        arrival = square_rect(taken, *ARRIVAL)
        for index, (gesture, geometry) in enumerate(
            [
                ("click", {"point": {"x": arrival["x"] + 4, "y": arrival["y"] + 4}}),
                ("box", {"rect": region_rect(taken, *REGION)}),
            ],
            start=1,
        ):
            packet = capture_packet(
                projection,
                taken,
                gesture,
                geometry,
                selection_id=f"sel-{index:04d}",
                comment="pointing at a photograph changes nothing",
            )
            session.write_selection(packet, None)

        self.assertEqual(len(session.selection_ids()), 2)
        after = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "status", "--porcelain"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        self.assertEqual(after, before)


def can_take_a_capture() -> bool:
    try:
        capture_harness.preflight(REPO_ROOT)
    except Exception:  # noqa: BLE001 - any refusal means the same thing here
        return False
    return True


@unittest.skipUnless(
    can_take_a_capture(),
    f"a capture needs {capture_harness.GODOT_VARIABLE} and {capture_harness.XVFB}; "
    "the tracked capture fixtures prove everything else",
)
class AFreshCaptureMatchesTheTrackedOne(unittest.TestCase):
    """The one thing tracked fixtures cannot prove: that they are still current.

    Skipped honestly, naming what is missing, on a machine without the client
    binary or a virtual display. It never passes by default.
    """

    def test_taking_a_capture_now_addresses_what_the_tracked_one_addresses(self) -> None:
        projection = accepted_projection()
        session = open_session(projection, "session-capture-currency")
        directory = REPO_ROOT / session.relative
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)
        fresh = capture_harness.request(REPO_ROOT, session.directory)
        tracked = fixture_route_capture()
        self.assertEqual(fresh.viewport, tracked.viewport)
        self.assertEqual(fresh.frame_generation, tracked.frame_generation)
        self.assertEqual(fresh.camera, tracked.camera)
        self.assertEqual(identities(fresh.targets), identities(tracked.targets))
        self.assertEqual(fresh.raster_digest, tracked.raster_digest)

    def test_the_capture_command_is_the_pinned_client_under_a_virtual_display(self) -> None:
        command = capture_harness.harness_command(REPO_ROOT, os.environ[capture_harness.GODOT_VARIABLE])
        self.assertEqual(command[0], capture_harness.XVFB)
        self.assertIn(str(REPO_ROOT / "client"), command)


class ThePreflightNamesWhatIsMissing(unittest.TestCase):
    """A capture that cannot be taken must refuse before anything is launched.

    Every one of these is a reason a *fresh checkout* cannot take a capture. The
    class cache is the one this suite learned the hard way: `client/.gitignore`
    ignores `.godot/`, so a clean clone has no `class_name` registry and the
    capture script fails to parse on the first `GridWorldView` it names — which
    arrived as an error inside a launched engine rather than as an honest
    unavailable. Found by `tools/run_clean_clone_proof.py`.
    """

    def preflight_in(self, missing: str | None) -> str:
        scratch = Path(tempfile.mkdtemp(prefix="tme-preflight-"))
        self.addCleanup(shutil.rmtree, scratch, ignore_errors=True)
        for relative in (capture_harness.FRAME_FIXTURE, capture_harness.CLASS_CACHE):
            if relative == missing:
                continue
            target = scratch / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("present\n", encoding="utf-8")
        with self.assertRaises(capture_harness.CaptureUnavailable) as raised:
            capture_harness.preflight(scratch)
        return str(raised.exception)

    def test_a_missing_class_cache_is_an_honest_unavailable(self) -> None:
        if not os.environ.get(capture_harness.GODOT_VARIABLE, "").strip():
            self.skipTest(f"{capture_harness.GODOT_VARIABLE} is not set")
        if shutil.which(capture_harness.XVFB) is None:
            self.skipTest(f"{capture_harness.XVFB} is not installed")
        reason = self.preflight_in(capture_harness.CLASS_CACHE)
        self.assertIn("class cache is missing", reason)
        self.assertIn("--import", reason)

    def test_a_missing_frame_fixture_is_an_honest_unavailable(self) -> None:
        reason = self.preflight_in(capture_harness.FRAME_FIXTURE)
        self.assertIn("recorded frame fixture is missing", reason)

    def test_an_unset_client_binary_is_an_honest_unavailable(self) -> None:
        previous = os.environ.pop(capture_harness.GODOT_VARIABLE, None)
        if previous is not None:
            self.addCleanup(os.environ.__setitem__, capture_harness.GODOT_VARIABLE, previous)
        reason = self.preflight_in(None)
        self.assertIn(capture_harness.GODOT_VARIABLE, reason)

    def test_the_class_cache_is_ignored_by_git_so_its_absence_is_normal(self) -> None:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "check-ignore", "-q", "--", capture_harness.CLASS_CACHE],
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, "the class cache must stay build output")


if __name__ == "__main__":
    unittest.main()
