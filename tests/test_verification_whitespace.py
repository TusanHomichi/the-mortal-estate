"""The whitespace step's coverage, proven in throwaway git repositories.

Issue 71: the step ran `git diff --check`, which reads the working tree and
never the index's diff against HEAD. A whitespace error that was staged while
the worktree matched the index, the ordinary state mid-commit, passed. The
step inspects index and working-tree differences independently, including when
an unstaged correction would conceal a staged defect in a combined HEAD diff.

The tests run the step's own argv through `execute.run_step` in real
repositories, because a hand-fed diff would say nothing about the command the
runner actually executes.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

from verification import execute, table

CLEAN = "line one\nline two\n"
DIRTY = "line one  \nline two\n"  # trailing whitespace on line one


class TempGitRepo:
    """A repository with one commit, isolated from host git configuration."""

    def __init__(self) -> None:
        self.path = Path(tempfile.mkdtemp(prefix="tme-whitespace-")).resolve()
        self.environ = {
            **os.environ,
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_AUTHOR_NAME": "whitespace test",
            "GIT_AUTHOR_EMAIL": "whitespace@example.invalid",
            "GIT_COMMITTER_NAME": "whitespace test",
            "GIT_COMMITTER_EMAIL": "whitespace@example.invalid",
        }
        self.git("init", "-q")
        self.write("note.txt", CLEAN)
        self.write("second.txt", CLEAN)
        self.git("add", "--", "note.txt", "second.txt")
        self.git("commit", "-qm", "base")

    def git(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", "-C", str(self.path), *args],
            env=self.environ,
            capture_output=True,
            text=True,
            check=False,
        )

    def write(self, relative: str, content: str) -> None:
        (self.path / relative).write_text(content, encoding="utf-8")

    def run_step(self) -> tuple[bool, str]:
        """Run `docs.whitespace` here, exactly as the runner would.

        The step is not a captured mode, so a failing `git` streams its
        diagnostic to this process's stdout. What the test asserts is the
        runner's own verdict: pass with an empty detail, or `exit 2`, git's
        "problems found" code, rather than an OSError's text.
        """
        step = table.STEPS["docs.whitespace"]
        argv = (step.argv[0], str(REPO_ROOT / step.argv[1]), *step.argv[2:])
        return execute.run_step(step, argv, root=self.path, environ=self.environ)

    def cleanup(self) -> None:
        shutil.rmtree(self.path)


class TheWhitespaceStep(unittest.TestCase):
    def setUp(self) -> None:
        self.repo = TempGitRepo()
        self.addCleanup(self.repo.cleanup)

    def test_a_clean_tree_passes(self) -> None:
        self.assertEqual(self.repo.run_step(), (True, ""))

    def test_an_unstaged_whitespace_error_fails(self) -> None:
        """Staged work elsewhere must not hide an unstaged trailing space."""
        self.repo.write("note.txt", "line one\nline two\nline three\n")
        self.repo.git("add", "--", "note.txt")
        self.repo.write("second.txt", DIRTY)
        self.assertEqual(self.repo.run_step(), (False, "exit 2"))

    def test_a_staged_whitespace_error_fails(self) -> None:
        """The reported defect: the pre-fix command saw nothing here."""
        self.repo.write("note.txt", DIRTY)
        self.repo.git("add", "--", "note.txt")
        prefixed = self.repo.git("diff", "--check")
        self.assertEqual(prefixed.returncode, 0, "the old argv should miss this")
        self.assertEqual(self.repo.run_step(), (False, "exit 2"))

    def test_a_staged_change_without_whitespace_errors_passes(self) -> None:
        self.repo.write("note.txt", "line one\nline two\nline three\n")
        self.repo.git("add", "--", "note.txt")
        self.assertEqual(self.repo.run_step(), (True, ""))

    def test_unstaged_correction_does_not_hide_a_staged_defect(self) -> None:
        self.repo.write("note.txt", DIRTY)
        self.repo.git("add", "--", "note.txt")
        self.repo.write("note.txt", CLEAN)
        self.assertEqual(self.repo.git("diff", "HEAD", "--check").returncode, 0)
        self.assertEqual(self.repo.run_step(), (False, "exit 2"))

    def test_committed_whitespace_is_history_not_a_pending_defect(self) -> None:
        """The baseline is HEAD, as it was before the fix."""
        self.repo.write("note.txt", DIRTY)
        self.repo.git("add", "--", "note.txt")
        self.repo.git("commit", "-qm", "carry a trailing space")
        self.assertEqual(self.repo.run_step(), (True, ""))


class TheRunnerContract(unittest.TestCase):
    def test_the_step_key_lane_and_mode_are_unchanged(self) -> None:
        step = table.STEPS["docs.whitespace"]
        self.assertEqual(step.owner, "docs")
        self.assertEqual(step.mode, "command")


if __name__ == "__main__":
    unittest.main()
