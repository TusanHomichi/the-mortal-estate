"""Tests for the banned-term check, including its P9 mutants.

Every term used here is invented nonsense from tests/fixtures/synthetic-terms.txt.
The real denylist is never carried by this tree, so these tests prove the
mechanism without proving it on the thing it protects.
"""

from __future__ import annotations

import os
import unittest

from boundary_test_support import (
    SYNTHETIC_TERMS,
    BoundaryTestCase,
    running_as_root,
)

import check_boundary_terms
from boundary_common import (
    EXIT_FAIL_CLOSED,
    EXIT_OK,
    EXIT_VIOLATION,
    ConfigError,
)


class TermMatching(unittest.TestCase):
    def compile_one(self, term: str):
        return check_boundary_terms.compile_terms([term])[0][1]

    def test_matching_is_case_insensitive(self) -> None:
        pattern = self.compile_one("zorbelquux")
        self.assertTrue(pattern.search("The ZoRbElQuUx returns."))

    def test_word_boundaries_prevent_substring_hits(self) -> None:
        pattern = self.compile_one("plithnak")
        self.assertIsNone(pattern.search("unplithnaked"))
        self.assertIsNone(pattern.search("plithnaks9"))
        self.assertIsNotNone(pattern.search("a plithnak, then"))

    def test_separators_inside_a_term_are_tolerant(self) -> None:
        pattern = self.compile_one("vorpal grimble")
        for variant in (
            "vorpal grimble",
            "Vorpal.Grimble",
            "vorpal_grimble",
            "vorpal-grimble",
            "VorpalGrimble",
        ):
            with self.subTest(variant=variant):
                self.assertIsNotNone(pattern.search(variant))

    def test_term_without_alphanumeric_content_fails_closed(self) -> None:
        with self.assertRaises(ConfigError):
            check_boundary_terms.compile_terms(["---"])


class CleanTree(BoundaryTestCase):
    def test_tree_without_terms_passes(self) -> None:
        self.repo.write("README.md", "The Mortal Estate.\n")
        self.repo.track("README.md")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_OK, output)
        self.assertIn("banned-terms: OK", output)

    def test_ignored_files_are_not_scanned(self) -> None:
        self.repo.write(".gitignore", "private/\n")
        self.repo.write("private/notes.md", "zorbelquux\n")
        self.repo.track(".gitignore")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_OK, output)

    def test_binary_content_is_skipped_but_its_name_is_not(self) -> None:
        self.repo.write_bytes("blob.bin", b"\x00\x01zorbelquux\x00")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_OK, output)

    def test_the_term_file_does_not_indict_itself(self) -> None:
        self.repo.write("terms.txt", "zorbelquux\n")
        self.repo.track("terms.txt")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(self.repo.path / "terms.txt")
        )
        self.assertEqual(code, EXIT_OK, output)


class Mutants(BoundaryTestCase):
    """P9: deliberate violations the check must kill."""

    def test_mutant_term_in_file_contents_is_killed(self) -> None:
        self.repo.write("docs/lore.md", "A line.\nThe mimsywort grows here.\n")
        self.repo.track("docs/lore.md")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("docs/lore.md:2", output)

    def test_mutant_term_in_file_path_is_killed(self) -> None:
        self.repo.write("content/quendaraff-notes.md", "nothing here\n")
        self.repo.track("content/quendaraff-notes.md")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("in the file path", output)

    def test_mutant_term_in_untracked_but_committable_file_is_killed(self) -> None:
        self.repo.write("staged_later.md", "zorbelquux\n")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_term_in_binary_file_name_is_killed(self) -> None:
        self.repo.write_bytes("art/plithnak.png", b"\x89PNG\x00\x00binary")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(SYNTHETIC_TERMS)
        )
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("in the file path", output)


class FailClosed(BoundaryTestCase):
    def test_missing_term_file_fails_closed(self) -> None:
        self.repo.write("README.md", "clean\n")
        code, output = self.run_check(
            check_boundary_terms.main,
            "--terms",
            str(self.repo.path / "absent" / "terms.txt"),
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("FAIL CLOSED", output)
        self.assertIn("missing", output)

    def test_default_term_path_missing_fails_closed(self) -> None:
        self.repo.write("README.md", "clean\n")
        code, output = self.run_check(check_boundary_terms.main)
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)

    def test_empty_term_file_fails_closed(self) -> None:
        self.repo.write("terms.txt", "# only a comment\n\n")
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(self.repo.path / "terms.txt")
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("no entries", output)

    @unittest.skipIf(running_as_root(), "uid 0 ignores the permission bits")
    def test_unreadable_term_file_fails_closed(self) -> None:
        terms = self.repo.write("terms.txt", "zorbelquux\n")
        os.chmod(terms, 0o000)
        self.addCleanup(os.chmod, terms, 0o644)
        code, output = self.run_check(
            check_boundary_terms.main, "--terms", str(terms)
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("unreadable", output)


if __name__ == "__main__":
    unittest.main()
