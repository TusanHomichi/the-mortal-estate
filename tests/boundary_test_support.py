"""Shared scaffolding for the public-boundary check tests.

The checks answer a question about a git working tree, so the tests give them
real git working trees in temporary directories. A mutant proved against a
hand-built file list would be proving something the check does not do.

Nothing here ever writes to the repository under test by the checks themselves;
mutants live only in temporary directories, never in this tree.
"""

from __future__ import annotations

import contextlib
import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
TOOLS = REPO_ROOT / "tools"
SYNTHETIC_TERMS = REPO_ROOT / "tests" / "fixtures" / "synthetic-terms.txt"

# Ignore rules a temp repository needs before the clean-room check will pass.
# Kept here so the private-root literals live in as few carried files as
# possible; this file is on the clean-room doc allowlist for exactly that.
PRIVATE_ROOT_IGNORE_RULES = "Research/\nplaceholders/\n.boundary/\n"

if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))


class TempRepo:
    """A throwaway git work tree a check can be pointed at."""

    def __init__(self) -> None:
        self.path = Path(tempfile.mkdtemp(prefix="tme-boundary-")).resolve()
        self._git("init", "-q")

    def _git(self, *args: str) -> None:
        subprocess.run(
            ["git", "-C", str(self.path), *args],
            check=True,
            capture_output=True,
            text=True,
        )

    def write(self, relative: str, content: str) -> Path:
        target = self.path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
        return target

    def write_bytes(self, relative: str, content: bytes) -> Path:
        target = self.path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
        return target

    def track(self, *relatives: str) -> None:
        self._git("add", "--", *relatives)

    def cleanup(self) -> None:
        shutil.rmtree(self.path, ignore_errors=True)


class BoundaryTestCase(unittest.TestCase):
    """Base case with a temp repo and output capture."""

    def setUp(self) -> None:
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)

    def run_check(self, entry_point, *arguments: str) -> tuple[int, str]:
        """Run a check's main(), returning its exit code and combined output."""
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = entry_point(["--root", str(self.repo.path), *arguments])
        return code, stdout.getvalue() + stderr.getvalue()


def running_as_root() -> bool:
    """chmod-based unreadability does not hold for uid 0."""
    return hasattr(os, "geteuid") and os.geteuid() == 0
