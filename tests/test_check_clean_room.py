"""Tests for the clean-room check, including its P9 mutants.

Every private-root string below lives in a temporary repository. The one place
this file names a private root outside a temp repo is its own allowlist entry
in tools/clean-room-allowlist.txt, which is why that entry exists.
"""

from __future__ import annotations

import os
import unittest

from boundary_test_support import (
    PRIVATE_ROOT_IGNORE_RULES,
    BoundaryTestCase,
    running_as_root,
)

import check_clean_room
from boundary_common import EXIT_FAIL_CLOSED, EXIT_OK, EXIT_VIOLATION

ALLOWLIST_GITIGNORE = ".gitignore  # must name the roots to ignore them\n"


class CleanRoomTestCase(BoundaryTestCase):
    def allowlist(self, content: str = ALLOWLIST_GITIGNORE) -> str:
        path = self.repo.path / "allowlist.txt"
        path.write_text(content, encoding="utf-8")
        return str(path)

    def check(self, allowlist_content: str = ALLOWLIST_GITIGNORE):
        return self.run_check(
            check_clean_room.main, "--allowlist", self.allowlist(allowlist_content)
        )

    def make_compliant_tree(self) -> None:
        self.repo.write(".gitignore", PRIVATE_ROOT_IGNORE_RULES)
        self.repo.track(".gitignore")


class CompliantTree(CleanRoomTestCase):
    def test_compliant_tree_passes(self) -> None:
        self.make_compliant_tree()
        self.repo.write("README.md", "The Mortal Estate.\n")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)
        self.assertIn("clean-room: OK", output)

    def test_allowlisted_doc_may_name_the_roots(self) -> None:
        self.make_compliant_tree()
        self.repo.write("docs/policy.md", "This tree never reads Research/ paths.\n")
        self.repo.track("docs/policy.md")
        code, output = self.check(
            ALLOWLIST_GITIGNORE + "docs/policy.md  # states the policy\n"
        )
        self.assertEqual(code, EXIT_OK, output)


class Mutants(CleanRoomTestCase):
    """P9: deliberate violations the check must kill."""

    def test_mutant_load_bearing_private_root_reference_is_killed(self) -> None:
        self.make_compliant_tree()
        self.repo.write(
            "tools/import_thing.py",
            'SOURCE = "Research/02_EXTRACTED/tables.json"\n',
        )
        self.repo.track("tools/import_thing.py")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("tools/import_thing.py:1", output)

    def test_mutant_placeholder_root_reference_is_killed(self) -> None:
        self.make_compliant_tree()
        self.repo.write("content/manifest.json", '{"art": "placeholders/art/x.png"}')
        self.repo.track("content/manifest.json")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_private_root_present_on_disk_is_killed(self) -> None:
        self.make_compliant_tree()
        self.repo.write("Research/notes.md", "material\n")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("absent entirely", output)

    def test_mutant_unignored_private_root_is_killed(self) -> None:
        self.repo.write(".gitignore", "# ignores nothing relevant\n*.tmp\n")
        self.repo.track(".gitignore")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("could be committed by accident", output)

    def test_mutant_stale_allowlist_entry_is_killed(self) -> None:
        self.make_compliant_tree()
        code, output = self.check(
            ALLOWLIST_GITIGNORE + "docs/vanished.md  # a doc that no longer exists\n"
        )
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("does not carry", output)


class FailClosed(CleanRoomTestCase):
    def test_missing_allowlist_fails_closed(self) -> None:
        self.make_compliant_tree()
        code, output = self.run_check(
            check_clean_room.main,
            "--allowlist",
            str(self.repo.path / "absent.txt"),
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("missing", output)

    def test_entry_less_allowlist_fails_closed(self) -> None:
        self.make_compliant_tree()
        code, output = self.check("# nothing\n")
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("no entries", output)

    @unittest.skipIf(running_as_root(), "uid 0 ignores the permission bits")
    def test_unreadable_allowlist_fails_closed(self) -> None:
        self.make_compliant_tree()
        path = self.repo.path / "allowlist.txt"
        path.write_text(ALLOWLIST_GITIGNORE, encoding="utf-8")
        os.chmod(path, 0o000)
        self.addCleanup(os.chmod, path, 0o644)
        code, output = self.run_check(
            check_clean_room.main, "--allowlist", str(path)
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("unreadable", output)


if __name__ == "__main__":
    unittest.main()
