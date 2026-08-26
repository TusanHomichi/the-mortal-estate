"""The tracked synthetic session fixture, and the projection loader's refusals.

The fixture is tracked proof that a clean clone can exercise the session shape
and the agent read path without an ignored root existing (the D6 ruling). It is
generated, so the guard it needs is that the tracked bytes are exactly what a
fresh generation writes — the same guard the compiler's projections get, for the
same reason: a hand edit and a stale fixture must fail identically.

The loader's refusals matter for the other half of D6: a missing private
artifact produces an honest unavailable result, never a false pass.
"""

from __future__ import annotations

import filecmp
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from workbench_test_support import (
    FIXTURE_ROOT,
    REPO_ROOT,
    SYNTHETIC_PROJECTION,
    synthetic_projection,
)

from workbench.projection import ProjectionUnavailable, load

GENERATOR = FIXTURE_ROOT / "regenerate.py"

EXPECTED_FILES = {
    "projection/synthetic-logical-projection.json",
    "regenerate.py",
    "session/manifest.json",
    "session/masks/sel-0003.pbm",
    "session/masks/sel-0004.pbm",
    "session/operations.jsonl",
    "session/selections/sel-0001.json",
    "session/selections/sel-0002.json",
    "session/selections/sel-0003.json",
    "session/selections/sel-0004.json",
    "sources/synthetic-companion.json",
    "sources/synthetic-master.json",
    "sources/synthetic-receipt.json",
    "sources/synthetic-runtime-projection.json",
}


class TheTrackedFixtureIsCurrent(unittest.TestCase):
    def test_the_fixture_holds_exactly_the_files_it_should(self) -> None:
        carried = {
            str(path.relative_to(FIXTURE_ROOT))
            for path in FIXTURE_ROOT.rglob("*")
            if path.is_file() and "__pycache__" not in path.parts
        }
        self.assertEqual(carried, EXPECTED_FILES)

    def test_a_fresh_generation_reproduces_the_tracked_bytes(self) -> None:
        scratch = Path(tempfile.mkdtemp(prefix="tme-workbench-fixture-")).resolve()
        self.addCleanup(shutil.rmtree, scratch, ignore_errors=True)
        completed = subprocess.run(
            [sys.executable, str(GENERATOR), "--out", str(scratch)],
            capture_output=True,
            text=True,
            cwd=REPO_ROOT,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        for relative in sorted(EXPECTED_FILES - {"regenerate.py"}):
            with self.subTest(file=relative):
                self.assertTrue((scratch / relative).is_file(), "the generator skipped it")
                self.assertTrue(
                    filecmp.cmp(FIXTURE_ROOT / relative, scratch / relative, shallow=False),
                    f"{relative} is stale or hand-edited: rerun {GENERATOR.name}",
                )

    def test_every_packet_binds_the_files_beside_it(self) -> None:
        projection = synthetic_projection()
        for path in sorted((FIXTURE_ROOT / "session/selections").glob("*.json")):
            with self.subTest(packet=path.name):
                packet = json.loads(path.read_text())
                self.assertEqual(packet["source"]["digests"], projection.source_records())

    def test_the_fixture_covers_all_four_gestures(self) -> None:
        gestures = {
            json.loads(path.read_text())["screen_region"]["gesture"]
            for path in (FIXTURE_ROOT / "session/selections").glob("*.json")
        }
        self.assertEqual(gestures, {"click", "box", "lasso", "paint"})

    def test_only_the_mask_bearing_gestures_carry_a_mask(self) -> None:
        for path in sorted((FIXTURE_ROOT / "session/selections").glob("*.json")):
            packet = json.loads(path.read_text())
            gesture = packet["screen_region"]["gesture"]
            with self.subTest(gesture=gesture):
                mask = packet["screen_region"]["mask"]
                if gesture in ("lasso", "paint"):
                    self.assertIsNotNone(mask)
                    self.assertTrue((FIXTURE_ROOT / mask["path"]).is_file())
                else:
                    self.assertIsNone(mask)

    def test_the_fixture_carries_no_capture_or_image_fields(self) -> None:
        """V0a addresses the logical view only. Those fields exist, and stay null."""
        for path in sorted((FIXTURE_ROOT / "session/selections").glob("*.json")):
            packet = json.loads(path.read_text())
            with self.subTest(packet=path.name):
                self.assertEqual(packet["view"], "logical")
                self.assertIsNone(packet["camera"])
                self.assertIsNone(packet["scene"]["frame_generation"])
                self.assertIsNone(packet["context_image"])
                self.assertIsNone(packet["commit_mask"])


class AnUnavailableProjectionIsHonest(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = Path(tempfile.mkdtemp(prefix="tme-workbench-missing-")).resolve()
        self.addCleanup(shutil.rmtree, self.scratch, ignore_errors=True)

    def test_a_missing_projection_names_the_command_that_produces_it(self) -> None:
        with self.assertRaises(ProjectionUnavailable) as refusal:
            load(self.scratch)
        self.assertIn("unavailable", str(refusal.exception))
        self.assertIn("cargo run -p tme-authoring", str(refusal.exception))

    def test_a_projection_of_the_wrong_kind_is_refused(self) -> None:
        target = self.scratch / SYNTHETIC_PROJECTION
        target.parent.mkdir(parents=True)
        target.write_text(json.dumps({"schema_version": 1, "kind": "something_else"}))
        with self.assertRaises(ProjectionUnavailable) as refusal:
            load(self.scratch, SYNTHETIC_PROJECTION)
        self.assertIn("declares kind", str(refusal.exception))

    def test_a_projection_of_a_future_schema_is_refused(self) -> None:
        document = json.loads((FIXTURE_ROOT / SYNTHETIC_PROJECTION).read_text())
        document["schema_version"] = 99
        target = self.scratch / SYNTHETIC_PROJECTION
        target.parent.mkdir(parents=True)
        target.write_text(json.dumps(document))
        with self.assertRaises(ProjectionUnavailable) as refusal:
            load(self.scratch, SYNTHETIC_PROJECTION)
        self.assertIn("schema version", str(refusal.exception))

    def test_a_truncated_projection_is_refused(self) -> None:
        target = self.scratch / SYNTHETIC_PROJECTION
        target.parent.mkdir(parents=True)
        target.write_text('{"schema_version": 1, "kind": "workbench_logical_projection"}')
        with self.assertRaises(ProjectionUnavailable) as refusal:
            load(self.scratch, SYNTHETIC_PROJECTION)
        self.assertIn("missing required content", str(refusal.exception))


if __name__ == "__main__":
    unittest.main()
