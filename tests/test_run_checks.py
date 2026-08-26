"""Tests for the boundary-check runner.

The runner's only job is to run all five checks and summarize honestly. The
failure mode worth testing is a runner that reports green while a check under
it is broken, so the fail-closed precedence gets its own case.
"""

from __future__ import annotations

import contextlib
import io
import unittest

from boundary_test_support import (
    PRIVATE_ROOT_IGNORE_RULES,
    REPO_ROOT,
    BoundaryTestCase,
)

import run_checks
from boundary_common import EXIT_FAIL_CLOSED, EXIT_OK, EXIT_VIOLATION


def run_against(root, terms=None) -> tuple[int, str]:
    argv = ["--root", str(root)]
    if terms is not None:
        argv += ["--terms", str(terms)]
    stdout, stderr = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
        code = run_checks.main(argv)
    return code, stdout.getvalue() + stderr.getvalue()


#: The private denylist, which a clean clone does not have.
PRIVATE_TERMS = REPO_ROOT / ".boundary/banned-terms.txt"
#: What CI and the verification runner point banned-terms at when it is absent.
SYNTHETIC_TERMS = REPO_ROOT / "tools/ci-synthetic-banned-terms.txt"


class RunnerOnThisRepository(unittest.TestCase):
    def test_this_repository_passes_every_check(self) -> None:
        """Against whichever denylist this checkout actually has.

        Asserting the real private list unconditionally makes this test's result
        a property of the *machine* rather than of the tree: on the owner's
        machine it passes, and on a clean clone — which has no `.boundary/` by
        design — it exits 3 FAIL CLOSED and reads like a defect. So it degrades
        the way the verification runner and CI degrade, onto the tracked
        synthetic fixture, and the claim narrows honestly with it.
        """
        private = PRIVATE_TERMS.is_file()
        code, output = run_against(REPO_ROOT, None if private else SYNTHETIC_TERMS)
        self.assertEqual(code, EXIT_OK, output)
        self.assertIn("all checks passed", output)
        self.assertEqual(output.count("PASS"), len(run_checks.CHECKS))

    def test_an_absent_denylist_fails_closed_rather_than_passing(self) -> None:
        """The degradation above must never be reachable by accident.

        Whatever this machine carries, pointing banned-terms at a file that is
        not there is exit 3 — never a quiet pass, and never a skip.
        """
        code, output = run_against(REPO_ROOT, REPO_ROOT / ".boundary/no-such-list.txt")
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("FAILED CLOSED", output)


class RunnerOnABrokenTree(BoundaryTestCase):
    def test_missing_configuration_reports_fail_closed(self) -> None:
        """A temp repo has none of the check configuration, so all five fail closed."""
        self.repo.write("README.md", "nothing here\n")
        code, output = run_against(self.repo.path)
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("FAILED CLOSED", output)

    def test_a_violation_alone_reports_violation(self) -> None:
        """Configuration present and valid, one check dirty: exit 1, not 3."""
        self.repo.write(".gitignore", PRIVATE_ROOT_IGNORE_RULES)
        self.repo.write(".boundary/banned-terms.txt", "zorbelquux\n")
        self.repo.write(
            "tools/hostname-allowlist.txt", "placeholder.invalid  # non-empty\n"
        )
        self.repo.write(
            "tools/clean-room-allowlist.txt", ".gitignore  # names the roots\n"
        )
        self.repo.write("docs/lore.md", "the zorbelquux appears\n")
        code, output = run_against(self.repo.path)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("violations found", output)
        self.assertIn("FAIL", output)


class RunnerWithARedirectedDenylist(BoundaryTestCase):
    """`--terms` exists so CI can run the banned-terms MECHANISM against a
    tracked synthetic list, because the real list is private and absent from a
    CI checkout. The property that matters is that redirecting is not the same
    as disarming: a bad path must still fail closed.
    """

    def configure(self) -> None:
        self.repo.write(".gitignore", PRIVATE_ROOT_IGNORE_RULES)
        self.repo.write(".boundary/banned-terms.txt", "zorbelquux\n")
        self.repo.write(
            "tools/hostname-allowlist.txt", "placeholder.invalid  # non-empty\n"
        )
        self.repo.write(
            "tools/clean-room-allowlist.txt", ".gitignore  # names the roots\n"
        )
        self.repo.write("tools/ci-terms.txt", "flibberwock\n")

    def test_terms_swaps_which_list_is_enforced(self) -> None:
        """The default list's term stops being a violation and the redirected
        list's term starts being one — proving the flag reaches the check
        rather than merely being accepted."""
        self.configure()
        self.repo.write("docs/lore.md", "the zorbelquux appears\n")
        terms = self.repo.path / "tools" / "ci-terms.txt"

        code, output = run_against(self.repo.path)
        self.assertEqual(code, EXIT_VIOLATION, output)

        code, output = run_against(self.repo.path, terms)
        self.assertEqual(code, EXIT_OK, output)

        self.repo.write("docs/lore.md", "the flibberwock appears\n")
        code, output = run_against(self.repo.path, terms)
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_the_other_four_checks_still_run_when_terms_is_redirected(self) -> None:
        self.configure()
        code, output = run_against(
            self.repo.path, self.repo.path / "tools" / "ci-terms.txt"
        )
        self.assertEqual(code, EXIT_OK, output)
        self.assertEqual(output.count("PASS"), len(run_checks.CHECKS))

    def test_an_unusable_terms_path_fails_closed_rather_than_skipping(self) -> None:
        """A deleted or emptied CI fixture must turn the run red. If either of
        these ever returns EXIT_OK, CI is green with the check disarmed."""
        self.configure()
        self.repo.write("tools/empty-terms.txt", "")
        for name in ("tools/absent-terms.txt", "tools/empty-terms.txt"):
            with self.subTest(terms=name):
                code, output = run_against(self.repo.path, self.repo.path / name)
                self.assertEqual(code, EXIT_FAIL_CLOSED, output)
                self.assertIn("FAILED CLOSED", output)


if __name__ == "__main__":
    unittest.main()
