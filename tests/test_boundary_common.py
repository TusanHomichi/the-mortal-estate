"""Tests for the shared scan surface every boundary check stands on.

If `carried_files` is wrong, every check is wrong in the same direction and
silently, so it gets its own tests rather than being proven by implication.
"""

from __future__ import annotations

import os
import unittest

from boundary_test_support import BoundaryTestCase, running_as_root

from boundary_common import (
    ConfigError,
    carried_files,
    git_is_ignored,
    load_list_file,
    read_text,
)


class CarriedFiles(BoundaryTestCase):
    def test_includes_tracked_and_untracked_but_not_ignored(self) -> None:
        self.repo.write(".gitignore", "secret/\n")
        self.repo.write("tracked.md", "a\n")
        self.repo.track(".gitignore", "tracked.md")
        self.repo.write("untracked.md", "b\n")
        self.repo.write("secret/hidden.md", "c\n")

        carried = carried_files(self.repo.path)
        self.assertIn("tracked.md", carried)
        self.assertIn("untracked.md", carried)
        self.assertIn(".gitignore", carried)
        self.assertNotIn("secret/hidden.md", carried)

    def test_drops_tracked_paths_deleted_from_disk(self) -> None:
        self.repo.write("gone.md", "a\n")
        self.repo.track("gone.md")
        os.remove(self.repo.path / "gone.md")
        self.assertNotIn("gone.md", carried_files(self.repo.path))


class ReadText(BoundaryTestCase):
    def test_binary_content_returns_none(self) -> None:
        path = self.repo.write_bytes("blob.bin", b"abc\x00def")
        self.assertIsNone(read_text(path))

    def test_text_content_returns_the_text(self) -> None:
        path = self.repo.write("notes.md", "hello\n")
        self.assertEqual(read_text(path), "hello\n")

    @unittest.skipIf(running_as_root(), "uid 0 ignores the permission bits")
    def test_unreadable_file_returns_none(self) -> None:
        path = self.repo.write("notes.md", "hello\n")
        os.chmod(path, 0o000)
        self.addCleanup(os.chmod, path, 0o644)
        self.assertIsNone(read_text(path))


class ListFiles(BoundaryTestCase):
    def test_comments_and_blanks_are_stripped(self) -> None:
        path = self.repo.write(
            "list.txt",
            "# header\n\nalpha\nbeta  # with a reason\n   \n",
        )
        self.assertEqual(load_list_file(path, "list"), ["alpha", "beta"])

    def test_missing_file_raises(self) -> None:
        with self.assertRaises(ConfigError):
            load_list_file(self.repo.path / "absent.txt", "list")

    def test_entry_less_file_raises(self) -> None:
        path = self.repo.write("list.txt", "# only comments\n")
        with self.assertRaises(ConfigError):
            load_list_file(path, "list")


class IgnoreProbe(BoundaryTestCase):
    def test_reports_ignore_status(self) -> None:
        self.repo.write(".gitignore", "secret/\n")
        self.repo.track(".gitignore")
        self.assertTrue(git_is_ignored(self.repo.path, "secret/probe"))
        self.assertFalse(git_is_ignored(self.repo.path, "docs/probe"))


if __name__ == "__main__":
    unittest.main()
