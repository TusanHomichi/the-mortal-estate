"""Tests for the review_refs resolution check, including its P9 mutants."""

from __future__ import annotations

import json
import unittest

from boundary_test_support import BoundaryTestCase

import check_review_refs
from boundary_common import EXIT_OK, EXIT_VIOLATION


def content_document(refs: list) -> str:
    return json.dumps(
        {
            "id": "example",
            "research_boundary": {
                "status": "clean_original_fixture",
                "review_refs": refs,
                "notes": "original authored content",
            },
        },
        indent=2,
    )


class EmptyAndCleanTrees(BoundaryTestCase):
    def test_empty_tree_is_green(self) -> None:
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_OK, output)

    def test_resolving_reference_passes(self) -> None:
        self.repo.write("docs/provenance.md", "the record\n")
        self.repo.write("content/thing.json", content_document(["docs/provenance.md"]))
        self.repo.track("docs/provenance.md", "content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_OK, output)

    def test_directory_reference_with_carried_files_passes(self) -> None:
        self.repo.write("docs/records/one.md", "a record\n")
        self.repo.write("content/thing.json", content_document(["docs/records"]))
        self.repo.track("docs/records/one.md", "content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_OK, output)

    def test_section_anchor_is_stripped_before_resolving(self) -> None:
        self.repo.write("docs/provenance.md", "the record\n")
        self.repo.write(
            "content/thing.json", content_document(["docs/provenance.md#sources"])
        )
        self.repo.track("docs/provenance.md", "content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_OK, output)

    def test_nested_research_boundary_is_still_found(self) -> None:
        document = {"entries": [{"research_boundary": {"review_refs": ["docs/x.md"]}}]}
        self.repo.write("docs/x.md", "record\n")
        self.repo.write("content/nested.json", json.dumps(document))
        self.repo.track("docs/x.md", "content/nested.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_OK, output)


class Mutants(BoundaryTestCase):
    """P9: deliberate violations the check must kill."""

    def test_mutant_orphaned_reference_is_killed(self) -> None:
        self.repo.write(
            "content/thing.json",
            content_document(["docs/internal/research-boundaries.md"]),
        )
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("does not resolve", output)

    def test_mutant_reference_to_untracked_path_is_killed(self) -> None:
        self.repo.write(".gitignore", "private/\n")
        self.repo.write("private/record.md", "hidden\n")
        self.repo.write("content/thing.json", content_document(["private/record.md"]))
        self.repo.track(".gitignore", "content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_empty_reference_entry_is_killed(self) -> None:
        self.repo.write("content/thing.json", content_document(["   "]))
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_empty_reference_array_is_killed(self) -> None:
        self.repo.write("content/thing.json", content_document([]))
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("is empty", output)

    def test_mutant_absolute_reference_is_killed(self) -> None:
        self.repo.write("content/thing.json", content_document(["/etc/hostname"]))
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("repository-relative", output)

    def test_mutant_escaping_reference_is_killed(self) -> None:
        self.repo.write("content/thing.json", content_document(["../elsewhere/x.md"]))
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("escapes", output)

    def test_mutant_non_string_reference_is_killed(self) -> None:
        self.repo.write("content/thing.json", content_document([42]))
        self.repo.track("content/thing.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("not a string", output)

    def test_mutant_unparseable_json_is_killed(self) -> None:
        self.repo.write("content/broken.json", "{ not json")
        self.repo.track("content/broken.json")
        code, output = self.run_check(check_review_refs.main)
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("does not parse", output)


if __name__ == "__main__":
    unittest.main()
