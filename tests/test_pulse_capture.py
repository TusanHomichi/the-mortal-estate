"""Tests for the pulse capture's judgement of what it photographed.

The capture itself needs PostgreSQL, TLS, credentials, a virtual display, and
the pinned Godot binary, so it runs on demand. Its *verdict* needs none of
that, and the verdict is the part that would quietly go soft: a judge that
accepted any three pictures would keep printing green for a meter that had
frozen, or for one that decided readiness for itself. These cases feed it
recorded manifests and check what it refuses.
"""

from __future__ import annotations

import contextlib
import io
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import boundary_test_support  # noqa: F401  (puts tools/ on sys.path)
import run_pulse_capture as pulse
from live_server_harness import ProofError


def sample(index: int, fill: float, **overrides: object) -> dict:
    row = {
        "index": index,
        "requested_fill": fill,
        "directory": f"beat-{index}",
        "logical_time": "41",
        "ready_at": "41",
        "can_act": True,
        "remaining_msec": 0,
        "known_duration": True,
        "duration_msec": 3004,
        "fill": fill,
        "segment_fills": [fill],
        "meter_text": f"◆ Ready · beat {int(fill * 100)}% · world T41 · ready T41",
    }
    row.update(overrides)
    return row


def manifest(*samples: dict, span: float = 3004.0) -> dict:
    return {
        "schema_version": 1,
        "kind": "pulse_capture_manifest",
        "measured_duration_msec": span,
        "requested_fills": [0.15, 0.5, 0.85],
        "samples": list(samples),
    }


class PulseCaptureVerdictTests(unittest.TestCase):
    def setUp(self) -> None:
        self._directory = TemporaryDirectory()
        self.output = Path(self._directory.name)
        self.addCleanup(self._directory.cleanup)

    def write_captures(self, *indexes: int) -> None:
        for index in indexes:
            directory = self.output / f"beat-{index}"
            directory.mkdir(parents=True, exist_ok=True)
            for name in pulse.CAPTURE_FILES:
                (directory / name).write_bytes(b"synthetic")

    def judge(self, document: dict) -> str:
        captured = io.StringIO()
        with contextlib.redirect_stdout(captured):
            pulse.check_manifest(document, self.output)
        return captured.getvalue()

    def test_a_meter_that_advanced_across_one_beat_passes(self) -> None:
        self.write_captures(1, 2, 3)
        report = self.judge(manifest(sample(1, 0.16), sample(2, 0.51), sample(3, 0.86)))
        self.assertIn("3 captures show the beat at 3 distinct fills", report)
        self.assertIn("advanced 0.70 of a beat", report)

    def test_a_frozen_meter_is_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(manifest(sample(1, 0.4), sample(2, 0.4), sample(3, 0.4)))
        self.assertIn("did not reach a distinct state", str(refusal.exception))

    def test_samples_straddling_a_beat_boundary_are_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(
                manifest(
                    sample(1, 0.16),
                    sample(2, 0.51),
                    sample(3, 0.86, ready_at="42"),
                )
            )
        self.assertIn("same individual action deadline", str(refusal.exception))

    def test_a_barely_moving_meter_is_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(manifest(sample(1, 0.40), sample(2, 0.46), sample(3, 0.52)))
        self.assertIn("to call it visible", str(refusal.exception))

    def test_a_measured_beat_that_is_not_the_ruled_cadence_is_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(
                manifest(sample(1, 0.16), sample(2, 0.51), sample(3, 0.86), span=1000.0)
            )
        self.assertIn("outside 3000 +/- 750 ms", str(refusal.exception))

    def test_a_meter_that_disagrees_with_the_frame_is_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(
                manifest(
                    sample(1, 0.16),
                    sample(2, 0.51),
                    # Readiness decided anywhere but the frame is the exact
                    # defect ruling D5 forbids, so the words and the frame have
                    # to agree in every sample.
                    sample(3, 0.86, can_act=False, remaining_msec=2),
                )
            )
        self.assertIn("did not say so", str(refusal.exception))

    def test_a_fill_drawn_without_a_measured_beat_is_refused(self) -> None:
        self.write_captures(1, 2, 3)
        with self.assertRaises(ProofError) as refusal:
            self.judge(
                manifest(sample(1, 0.16), sample(2, 0.51), sample(3, 0.86, known_duration=False))
            )
        self.assertIn("without having measured a beat", str(refusal.exception))

    def test_a_sample_without_its_three_files_is_refused(self) -> None:
        self.write_captures(1, 2)
        with self.assertRaises(ProofError) as refusal:
            self.judge(manifest(sample(1, 0.16), sample(2, 0.51), sample(3, 0.86)))
        self.assertIn("is missing capture.png", str(refusal.exception))

    def test_too_few_samples_are_refused_rather_than_read_as_success(self) -> None:
        self.write_captures(1, 2)
        for document in (manifest(), manifest(sample(1, 0.2), sample(2, 0.8))):
            with self.subTest(samples=len(document["samples"])):
                with self.assertRaises(ProofError) as refusal:
                    self.judge(document)
                self.assertIn("at least three inside one", str(refusal.exception))

    def test_the_client_is_judged_against_the_ruled_pulse_it_was_never_told(self) -> None:
        # The client measures its own beat from the frames that arrive; nothing
        # on the wire carries the cadence. The judgement here is therefore an
        # agreement between two independent statements, which is only worth
        # anything while the tolerance stays narrow.
        self.assertEqual(3000.0, pulse.STANDARD_ACTION_MSEC)
        self.assertLess(pulse.COOLDOWN_TOLERANCE_MSEC, pulse.STANDARD_ACTION_MSEC - 1000.0)

    def test_a_manifest_with_an_absolute_directory_is_read_where_it_says(self) -> None:
        self.write_captures(1, 2, 3)
        document = manifest(sample(1, 0.16), sample(2, 0.51), sample(3, 0.86))
        for row in document["samples"]:
            row["directory"] = str(self.output / row["directory"])
        report = self.judge(document)
        self.assertIn("distinct fills", report)


class PulseCaptureManifestShapeTests(unittest.TestCase):
    def test_the_driver_and_the_client_agree_on_what_they_exchange(self) -> None:
        # Two processes and one contract: the file name, the sentinel, and the
        # script path. Each is stated on both sides, so each is checked.
        root = Path(__file__).resolve().parents[1]
        client = root / "client/tests/pulse_capture.gd"
        source = client.read_text(encoding="utf-8")
        self.assertIn(f'MANIFEST_NAME: String = "{pulse.MANIFEST_NAME}"', source)
        self.assertIn(f'SUCCESS_SENTINEL: String = "{pulse.SUCCESS_SENTINEL}"', source)
        self.assertEqual("res://tests/pulse_capture.gd", pulse.CLIENT_SCRIPT)
        self.assertTrue(client.is_file(), "the driver names a client script that exists")

    def test_the_sample_fills_the_client_asks_for_span_the_spread_required(self) -> None:
        client = Path(__file__).resolve().parents[1] / "client/tests/pulse_capture.gd"
        source = client.read_text(encoding="utf-8")
        line = next(
            row for row in source.splitlines() if row.startswith("const SAMPLE_FILLS")
        )
        fills = json.loads(line.split("=", 1)[1].strip())
        self.assertGreaterEqual(
            fills[-1] - fills[0],
            pulse.MINIMUM_FILL_SPREAD,
            "the points sampled must be able to clear the spread the judge demands",
        )


if __name__ == "__main__":
    unittest.main()
