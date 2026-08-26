"""The clean-clone proof's copy and disposal, without paying for a build.

The expensive half — building and testing inside the copy — is what the
`cleanclone` lane runs. What is proven here is everything either side of it:
the copy is the carried set and contains none of the private roots, and the
several gigabytes of build output the proof creates are reported and removed on
every path out, including the failing one.
"""

from __future__ import annotations

import contextlib
import io
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from verification_test_support import REPO_ROOT

from verification import footprint

import run_clean_clone_proof as proof

ROOTS = proof.forbidden_roots(REPO_ROOT)


class TempRepo:
    """A throwaway git work tree. The caller must register `cleanup`."""

    def __init__(self) -> None:
        self.path = Path(tempfile.mkdtemp(prefix="tme-clean-clone-test-")).resolve()
        subprocess.run(["git", "-C", str(self.path), "init", "-q"], check=True)

    def cleanup(self) -> None:
        shutil.rmtree(self.path, ignore_errors=True)

    def write(self, relative: str, content: str) -> None:
        target = self.path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    def add(self, *names: str) -> None:
        subprocess.run(["git", "-C", str(self.path), "add", "--", *names], check=True)


class TheCopyIsTheCarriedSet(unittest.TestCase):
    def setUp(self) -> None:
        # Both roots are registered for cleanup here rather than left to the
        # operating system: this suite runs inside the clean-clone proof, which
        # runs inside a full verification run, and a test that leaks a scratch
        # tree per case fills a machine quietly.
        self.source = TempRepo()
        self.addCleanup(self.source.cleanup)
        self.destination = Path(tempfile.mkdtemp(prefix="tme-clean-clone-out-")).resolve()
        self.addCleanup(shutil.rmtree, self.destination, True)

    def test_tracked_and_untracked_files_both_travel(self) -> None:
        self.source.write("tracked.md", "# tracked\n")
        self.source.add("tracked.md")
        self.source.write("untracked.md", "# untracked but committable\n")
        count = proof.populate(self.destination, self.source.path)
        self.assertEqual(count, 2)
        self.assertTrue((self.destination / "tracked.md").is_file())
        self.assertTrue((self.destination / "untracked.md").is_file())

    def test_ignored_files_do_not_travel(self) -> None:
        self.source.write(".gitignore", ".boundary/\n")
        self.source.add(".gitignore")
        self.source.write(".boundary/banned-terms.txt", "private\n")
        proof.populate(self.destination, self.source.path)
        self.assertFalse((self.destination / ".boundary").exists())

    def test_a_copy_carrying_a_private_root_is_refused(self) -> None:
        (self.destination / ".boundary").mkdir()
        with self.assertRaises(proof.CleanCloneError) as raised:
            proof.assert_clean(self.destination, ROOTS)
        self.assertIn(".boundary", str(raised.exception))

    def test_a_clean_copy_is_accepted(self) -> None:
        proof.assert_clean(self.destination, ROOTS)

    def test_the_build_directory_is_refused_too(self) -> None:
        (self.destination / "target").mkdir()
        with self.assertRaises(proof.CleanCloneError):
            proof.assert_clean(self.destination, ROOTS)


class TheForbiddenRoots(unittest.TestCase):
    """The list is read from `.gitignore`, so it cannot drift away from it."""

    def test_every_ignored_directory_root_is_covered(self) -> None:
        declared = {
            line.split("#", 1)[0].strip().strip("/")
            for line in (REPO_ROOT / ".gitignore").read_text(encoding="utf-8").splitlines()
            if line.split("#", 1)[0].strip().endswith("/")
        }
        self.assertEqual(set(ROOTS), declared)
        self.assertTrue(declared)

    def test_a_gitignore_with_no_roots_fails_closed(self) -> None:
        scratch = Path(tempfile.mkdtemp(prefix="tme-gitignore-"))
        self.addCleanup(shutil.rmtree, scratch, True)
        (scratch / ".gitignore").write_text("# nothing but a comment\n", encoding="utf-8")
        with self.assertRaises(proof.CleanCloneError):
            proof.forbidden_roots(scratch)

    def test_this_repository_carries_none_of_them_in_its_carried_set(self) -> None:
        for name in proof.carried_paths(REPO_ROOT):
            for root in ROOTS:
                self.assertFalse(name.startswith(root + "/"), name)


class TheBuildOutputIsThisProofsProperty(unittest.TestCase):
    """Several gigabytes, removed on every path out — and reported either way.

    The lane inside the copy is stubbed to fail here rather than run: what is
    being proven is the *disposal*, and paying for a real cold build to learn
    whether a `finally` block runs would be its own kind of waste. The build
    itself is proven by the lane, on a real copy, in CI.
    """

    def _run(self, *, returncode: int, keep: bool) -> tuple[int, str]:
        real = subprocess.run

        def runner(argv, **kwargs):
            if argv and argv[0] == sys.executable:
                return subprocess.CompletedProcess(argv, returncode)
            return real(argv, **kwargs)

        stream = io.StringIO()
        with mock.patch.object(proof.subprocess, "run", runner):
            # stderr too: the failure cases print a real diagnostic, and a
            # passing suite that shouts is a suite people stop reading.
            with contextlib.redirect_stdout(stream), contextlib.redirect_stderr(io.StringIO()):
                code = proof.main(["--keep"] if keep else [])
        return code, stream.getvalue()

    def _build_directory(self, output: str) -> Path:
        line = next(l for l in output.splitlines() if l.startswith("build output: "))
        return Path(line.split("build output: ", 1)[1])

    def test_a_failed_lane_still_takes_its_build_output_with_it(self) -> None:
        code, output = self._run(returncode=1, keep=False)
        self.assertEqual(code, 1)
        self.assertFalse(self._build_directory(output).exists())

    def test_a_failed_lane_still_reports_what_it_spent(self) -> None:
        _code, output = self._run(returncode=1, keep=False)
        self.assertIn("TME_CLEAN_CLONE_PEAK_MiB=", output)

    def test_keep_keeps_the_copy_and_not_the_build_output(self) -> None:
        code, output = self._run(returncode=0, keep=True)
        self.assertEqual(code, 0)
        build = self._build_directory(output)
        self.addCleanup(shutil.rmtree, build.parent, True)
        self.assertFalse(build.exists())
        self.assertTrue((build.parent / "the-mortal-estate").is_dir())

    def test_the_copy_is_built_with_the_disposable_profile(self) -> None:
        _code, output = self._run(returncode=0, keep=False)
        for name, value in footprint.LEAN_BUILD_ENV.items():
            self.assertIn(f"{name}={value}", output)


if __name__ == "__main__":
    unittest.main()
