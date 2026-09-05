"""Acceptance criterion 3 — the identity sidecar is real and matching.

"Real" means the sidecar describes the picture beside it rather than a picture
in general: its viewport is the picture's own dimensions, read out of the PNG
header, and its raster is the same size again. "Matching" means the three pieces
are one fact told three ways — the target list, the raster, and the pointer
resolution the presenter itself performs.

The strongest of those is the raster, and it is proven by reconstruction: fill
every target's rectangle in draw order and the bytes must equal the raster the
presenter wrote. If they do, the raster is not an approximation of the target
list, it is the target list.

These retained fixtures prove correspondence within their recorded files. A
fresh browser producer must separately prove that its identity targets match
its own hit testing; tools/run_browser_capture_proof.py exercises that surface.
"""

from __future__ import annotations

import json
import unittest

from workbench_test_support import (
    FIXTURE_ROUTE,
    BOUND_FILES,
    CAPTURE_FILES,
    StagedTree,
    LIVE_ROUTE,
    REPO_ROOT,
    fixture_route_capture,
    live_route_capture,
    recorded_frame,
)

from workbench import capture as capture_reader
from workbench.projection import WorkbenchError, StaleSelection, digest_bytes


def rebuild_raster(taken) -> bytes:
    """The raster the target list implies: every rectangle, in draw order."""
    width, height = taken.raster.width, taken.raster.height
    samples = bytearray(width * height * 2)
    for record in taken.targets:
        index = int(record["index"])
        shape = record["hit_shape"]
        high, low = index >> 8, index & 0xFF
        for y in range(shape["y"], min(height, shape["y"] + shape["height"])):
            row = y * width
            for x in range(shape["x"], min(width, shape["x"] + shape["width"])):
                offset = (row + x) * 2
                samples[offset] = high
                samples[offset + 1] = low
    return bytes(samples)


class TheSidecarDescribesItsOwnPicture(unittest.TestCase):
    def routes(self):
        return (("fixture", fixture_route_capture()), ("live", live_route_capture()))

    def test_the_viewport_the_sidecar_declares_is_the_pictures_own_size(self) -> None:
        for name, taken in self.routes():
            with self.subTest(route=name):
                width, height = capture_reader.png_size(taken.image)
                self.assertEqual(taken.viewport, {"width": width, "height": height})

    def test_the_raster_is_the_same_resolution_as_the_picture(self) -> None:
        for name, taken in self.routes():
            with self.subTest(route=name):
                self.assertEqual(
                    (taken.raster.width, taken.raster.height),
                    (taken.viewport["width"], taken.viewport["height"]),
                )

    def test_the_sidecar_names_the_bytes_of_the_files_beside_it(self) -> None:
        """Loading already checks this; asserting it here says it is the contract."""
        for name, taken in self.routes():
            with self.subTest(route=name):
                self.assertEqual(taken.image_digest, taken.document["image"]["sha256"])
                self.assertEqual(
                    taken.raster_digest, taken.document["identity_raster"]["sha256"]
                )

    def test_the_frame_generation_and_scene_are_the_recorded_frames(self) -> None:
        document = recorded_frame()
        taken = fixture_route_capture()
        self.assertEqual(taken.frame_generation, document["frame_generation"])
        self.assertEqual(taken.level, document["frame"]["observation_center"]["level"])
        self.assertEqual(
            taken.document["scene"]["observation_center"],
            document["frame"]["observation_center"]["position"],
        )

    def test_the_camera_states_every_framing_constant_in_force(self) -> None:
        for name, taken in self.routes():
            with self.subTest(route=name):
                camera = taken.camera
                self.assertEqual(camera["kind"], "orthographic_square_lattice")
                for field in ("square_pitch_px", "square_origin_px", "square_bounds"):
                    self.assertIn(field, camera)
                # The camera is complete: a square's rectangle follows from the
                # pitch and the origin alone, and the target list agrees.
                pitch = int(camera["square_pitch_px"])
                origin = camera["square_origin_px"]
                for record in taken.targets:
                    if record["kind"] != "tile":
                        continue
                    square = record["coordinate"]
                    self.assertEqual(
                        record["hit_shape"],
                        {
                            "kind": "rect",
                            "x": origin["x"] + int(square["x"]) * pitch,
                            "y": origin["y"] + int(square["y"]) * pitch,
                            "width": pitch,
                            "height": pitch,
                        },
                    )


class TheRasterAndTheTargetListAreOneFact(unittest.TestCase):
    def test_the_raster_is_exactly_the_target_rectangles_in_draw_order(self) -> None:
        for name, taken in (("fixture", fixture_route_capture()), ("live", live_route_capture())):
            with self.subTest(route=name):
                self.assertEqual(rebuild_raster(taken), taken.raster.samples)

    def test_every_targets_anchor_pixel_names_that_target(self) -> None:
        """An anchor a target does not own would be a wrong answer with a
        confident shape, which is the failure mode this whole slice exists to
        avoid."""
        for name, taken in (("fixture", fixture_route_capture()), ("live", live_route_capture())):
            with self.subTest(route=name):
                for record in taken.targets:
                    anchor = record["anchor"]
                    self.assertEqual(
                        taken.raster.index_at(int(anchor["x"]), int(anchor["y"])),
                        int(record["index"]),
                        f"{record['identity']} does not own its own anchor",
                    )

    def test_occupant_markers_own_their_pixels_over_the_square_beneath(self) -> None:
        """Draw order is real: a marker overwrites the square it stands on."""
        taken = fixture_route_capture()
        occupants = [row for row in taken.targets if row["presentation_layer"] == "occupants"]
        self.assertTrue(occupants, "the recorded frame shows at least one occupant")
        for record in occupants:
            shape = record["hit_shape"]
            centre_x = shape["x"] + shape["width"] // 2
            centre_y = shape["y"] + shape["height"] // 2
            self.assertEqual(taken.raster.index_at(centre_x, centre_y), int(record["index"]))
            square = [
                row
                for row in taken.targets
                if row["kind"] == "tile" and row["coordinate"] == record["coordinate"]
            ]
            self.assertEqual(len(square), 1)
            self.assertLess(
                int(square[0]["index"]),
                int(record["index"]),
                "the square is drawn before what stands on it",
            )

    def test_a_pixel_no_target_covers_indexes_nothing(self) -> None:
        taken = fixture_route_capture()
        self.assertEqual(taken.raster.index_at(0, 0), 0)
        self.assertEqual(
            taken.raster.index_at(taken.viewport["width"] - 1, taken.viewport["height"] - 1), 0
        )


class ABrokenCaptureIsRefusedRatherThanRead(unittest.TestCase):
    """Honest unavailability: a capture whose pieces disagree is not a surface."""

    def setUp(self) -> None:
        import shutil
        import tempfile
        from pathlib import Path

        self.staged = Path(tempfile.mkdtemp(prefix="tme-capture-")).resolve()
        self.addCleanup(shutil.rmtree, self.staged, ignore_errors=True)
        self.directory = self.staged / "capture"
        shutil.copytree(REPO_ROOT / FIXTURE_ROUTE, self.directory)

    def load(self):
        return capture_reader.load(self.staged, self.directory)

    def test_a_replaced_picture_is_refused(self) -> None:
        payload = bytearray((self.directory / "capture.png").read_bytes())
        payload[-1] ^= 0x01
        (self.directory / "capture.png").write_bytes(bytes(payload))
        with self.assertRaises(capture_reader.CaptureUnavailable) as refusal:
            self.load()
        self.assertIn("capture.png", str(refusal.exception))

    def test_a_replaced_raster_is_refused(self) -> None:
        payload = bytearray((self.directory / "capture.identity.pgm").read_bytes())
        payload[-1] ^= 0x01
        (self.directory / "capture.identity.pgm").write_bytes(bytes(payload))
        with self.assertRaises(capture_reader.CaptureUnavailable) as refusal:
            self.load()
        self.assertIn("capture.identity.pgm", str(refusal.exception))

    def test_a_sidecar_claiming_the_wrong_viewport_is_refused(self) -> None:
        path = self.directory / "capture.sidecar.json"
        document = json.loads(path.read_text(encoding="utf-8"))
        document["viewport"]["width"] += 1
        path.write_text(json.dumps(document), encoding="utf-8")
        with self.assertRaises(capture_reader.CaptureUnavailable) as refusal:
            self.load()
        self.assertIn("viewport", str(refusal.exception))

    def test_a_missing_piece_is_refused_by_name(self) -> None:
        (self.directory / "capture.identity.pgm").unlink()
        with self.assertRaises(capture_reader.CaptureUnavailable) as refusal:
            self.load()
        self.assertIn("incomplete", str(refusal.exception))

    def test_a_sidecar_from_a_future_schema_is_refused_rather_than_guessed_at(self) -> None:
        path = self.directory / "capture.sidecar.json"
        document = json.loads(path.read_text(encoding="utf-8"))
        document["schema_version"] = 99
        path.write_text(json.dumps(document), encoding="utf-8")
        with self.assertRaises(capture_reader.CaptureUnavailable) as refusal:
            self.load()
        self.assertIn("schema version", str(refusal.exception))

    def test_a_raster_that_is_not_a_raster_is_refused(self) -> None:
        with self.assertRaises(WorkbenchError):
            capture_reader.read_raster(b"P2\n2 2\n255\n0 0 0 0\n")
        with self.assertRaises(WorkbenchError):
            capture_reader.read_raster(b"P5\n2 2\n255\n\x00\x00\x00\x00")

    def test_something_that_is_not_a_png_is_refused(self) -> None:
        with self.assertRaises(WorkbenchError):
            capture_reader.png_size(b"not a picture at all, not even close to one")


class BrowserAuthorityBinding(StagedTree):
    """Digest mutations over synthetic metadata; actual rendering has live proof."""
    staged_files = BOUND_FILES + CAPTURE_FILES

    def setUp(self):
        super().setUp()
        self.projection = self.staged_projection()
        self.directory = self.staged / FIXTURE_ROUTE
        self.sidecar_path = self.directory / "capture.sidecar.json"
        self.sidecar = json.loads(self.sidecar_path.read_bytes())
        frame = recorded_frame()["frame"]
        envelope = {"kind": "server_welcome", "server_sequence": "5", "world_revision": "7", "frame": frame}
        encoded = json.dumps(envelope, separators=(",", ":"), ensure_ascii=False)
        recording = json.dumps({"schema_version": 1, "kind": "browser_observer_recording", "envelopes": [encoded]}).encode()
        (self.directory / "capture.frame.json").write_bytes(recording)
        self.sidecar.update(producer="browser_authoritative_view", frame_generation=1,
            scene={"realm": frame["observation_center"]["realm"], "level": frame["observation_center"]["level"],
                   "logical_time": frame["logical_time"], "observation_center": frame["observation_center"]["position"]},
            authority={"path": "capture.frame.json", "sha256": digest_bytes(recording), "envelope_sha256": digest_bytes(encoded.encode()),
                       "server_sequence": "5", "world_revision": "7", "sources": self.projection.source_records()})
        self.write_sidecar()

    def write_sidecar(self):
        self.sidecar_path.write_text(json.dumps(self.sidecar))

    def test_the_recording_is_a_separately_bound_capture_source(self):
        taken = capture_reader.load(self.staged, self.directory)
        capture_reader.bind(self.projection, taken)
        self.assertEqual(taken.source_records(self.staged)[-1]["role"], "capture_frame")

    def test_changed_recording_and_absent_authority_are_refused(self):
        recording = self.directory / "capture.frame.json"
        recording.write_bytes(recording.read_bytes() + b"\n")
        with self.assertRaisesRegex(WorkbenchError, "recording digest"):
            capture_reader.load(self.staged, self.directory)
        del self.sidecar["authority"]
        self.write_sidecar()
        with self.assertRaises(WorkbenchError):
            capture_reader.load(self.staged, self.directory)

    def test_frame_generation_and_scene_cannot_disagree_with_the_recording(self):
        for mutate in (lambda: self.sidecar.update(frame_generation=2),
                       lambda: self.sidecar["scene"].update(logical_time="0")):
            original = json.loads(json.dumps(self.sidecar))
            mutate(); self.write_sidecar()
            with self.assertRaises(WorkbenchError):
                capture_reader.load(self.staged, self.directory)
            self.sidecar = original

    def test_changed_compiler_source_and_cached_capture_fail_closed(self):
        taken = capture_reader.load(self.staged, self.directory)
        self.corrupt(BOUND_FILES[0])
        with self.assertRaises(StaleSelection):
            capture_reader.load(self.staged, self.directory)
        self.sidecar_path.write_bytes(self.sidecar_path.read_bytes() + b"\n")
        with self.assertRaises(StaleSelection):
            capture_reader.select(self.projection, taken, "click", {"point": taken.targets[0]["anchor"]})


if __name__ == "__main__":
    unittest.main()
