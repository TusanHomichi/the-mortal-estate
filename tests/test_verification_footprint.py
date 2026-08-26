"""The disk measurement, proven — because it is what a budget decision rests on.

A footprint number nobody checked is the same kind of evidence as a green run
nobody looked at. Each case here is a way the measurement could be wrong in the
direction that matters: reporting *less* than the disk is actually paying.
"""

from __future__ import annotations

import shutil
import tempfile
import threading
import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

from verification import footprint


class Scratch(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="tme-footprint-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, True)

    def write(self, relative: str, size: int) -> Path:
        target = self.root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(b"\0" * size)
        return target


class DirectoryBytes(Scratch):
    def test_it_counts_what_is_there(self) -> None:
        self.write("a/b/one.bin", 3 * footprint.MEBIBYTE)
        self.write("a/two.bin", 1 * footprint.MEBIBYTE)
        measured = footprint.directory_bytes(self.root)
        self.assertGreaterEqual(measured, 4 * footprint.MEBIBYTE)
        self.assertLess(measured, 6 * footprint.MEBIBYTE)

    def test_a_hard_link_is_counted_once(self) -> None:
        original = self.write("one.bin", 4 * footprint.MEBIBYTE)
        alone = footprint.directory_bytes(self.root)
        (self.root / "also-one.bin").hardlink_to(original)
        self.assertEqual(footprint.directory_bytes(self.root), alone)

    def test_a_directory_that_is_not_there_costs_nothing(self) -> None:
        self.assertEqual(footprint.directory_bytes(self.root / "absent"), 0)

    def test_an_unreadable_subtree_does_not_raise(self) -> None:
        """A build directory is being rewritten while this walks it."""
        self.write("locked/one.bin", footprint.MEBIBYTE)
        (self.root / "locked").chmod(0o000)
        self.addCleanup((self.root / "locked").chmod, 0o700)
        footprint.directory_bytes(self.root)


class Mebibytes(unittest.TestCase):
    def test_a_nonzero_footprint_never_reports_zero(self) -> None:
        self.assertEqual(footprint.mebibytes(1), 1)
        self.assertEqual(footprint.mebibytes(0), 0)
        self.assertEqual(footprint.mebibytes(footprint.MEBIBYTE + 1), 2)


class FreeBytes(Scratch):
    def test_it_answers_for_a_path_that_does_not_exist_yet(self) -> None:
        self.assertGreater(footprint.free_bytes(self.root / "not" / "made" / "yet"), 0)


class TheLeanProfile(unittest.TestCase):
    def test_the_callers_value_does_not_win(self) -> None:
        merged = footprint.lean_environment({"CARGO_INCREMENTAL": "1", "PATH": "/bin"})
        self.assertEqual(merged["CARGO_INCREMENTAL"], "0")
        self.assertEqual(merged["PATH"], "/bin")

    def test_it_leaves_the_source_environment_alone(self) -> None:
        source = {"CARGO_INCREMENTAL": "1"}
        footprint.lean_environment(source)
        self.assertEqual(source, {"CARGO_INCREMENTAL": "1"})

    def test_incremental_state_and_full_debuginfo_are_both_refused(self) -> None:
        self.assertEqual(footprint.LEAN_BUILD_ENV["CARGO_INCREMENTAL"], "0")
        for name in ("CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"):
            self.assertIn(footprint.LEAN_BUILD_ENV[name], {"0", "line-tables-only"})

    def test_the_summary_names_every_variable(self) -> None:
        summary = footprint.lean_summary()
        for name, value in footprint.LEAN_BUILD_ENV.items():
            self.assertIn(f"{name}={value}", summary)


class TargetDirectory(unittest.TestCase):
    def test_the_environment_wins_where_it_is_set(self) -> None:
        self.assertEqual(
            footprint.target_directory({"CARGO_TARGET_DIR": "/tmp/x"}, Path("/repo")),
            Path("/tmp/x"),
        )

    def test_cargos_default_otherwise(self) -> None:
        self.assertEqual(footprint.target_directory({}, Path("/repo")), Path("/repo/target"))


class ThePeak(Scratch):
    def test_it_reports_the_peak_and_not_the_size_left_behind(self) -> None:
        """Cargo deletes superseded artifacts; the disk still had to hold them."""
        sampler = footprint.PeakFootprint(self.root, interval=0.01)
        with sampler:
            big = self.write("big.bin", 8 * footprint.MEBIBYTE)
            settled = threading.Event()
            while not settled.wait(0.02):
                if sampler.peak_bytes >= 8 * footprint.MEBIBYTE:
                    break
            big.unlink()
        self.assertGreaterEqual(sampler.peak_bytes, 8 * footprint.MEBIBYTE)
        self.assertLess(footprint.directory_bytes(self.root), footprint.MEBIBYTE)

    def test_a_build_that_finishes_between_ticks_is_still_measured(self) -> None:
        sampler = footprint.PeakFootprint(self.root, interval=3600.0)
        with sampler:
            self.write("late.bin", 2 * footprint.MEBIBYTE)
        self.assertGreaterEqual(sampler.peak_bytes, 2 * footprint.MEBIBYTE)

    def test_the_sampling_thread_does_not_outlive_the_block(self) -> None:
        before = threading.active_count()
        with footprint.PeakFootprint(self.root, interval=0.01):
            pass
        self.assertEqual(threading.active_count(), before)


class Describe(Scratch):
    def test_it_names_the_directory_its_cost_and_what_is_left(self) -> None:
        self.write("one.bin", footprint.MEBIBYTE)
        line = footprint.describe(self.root)
        self.assertIn(str(self.root), line)
        self.assertIn("MiB free", line)


if __name__ == "__main__":
    unittest.main()
