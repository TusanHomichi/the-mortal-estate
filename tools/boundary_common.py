"""Shared helpers for The Mortal Estate public-boundary checks.

Every check in this directory answers the same question about the same file
set: what will this repository publish? The helpers here define that file set
once, so no check can drift into scanning a different tree than its siblings.

**The carried set.** A check scans the files git carries *or would carry*:
tracked files plus untracked files that are not ignored. Scanning only
`git ls-files` would let a violation sit unexamined in the working tree until
the moment it is committed — the check would pass right up to the instant it
mattered. Ignored files are excluded because they are, by construction, the
private side of the boundary.

**Exit codes** are shared by every check so a runner can tell a violation from
a broken check:

* 0 — clean.
* 1 — violations found. The tree is wrong.
* 2 — usage error (argparse's own).
* 3 — FAIL CLOSED. The check could not run as specified: a missing or
  unreadable configuration input, or git unavailable. A boundary check that
  silently passes when its input is absent is worse than no check, so this is
  never a skip and never a pass.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

EXIT_OK = 0
EXIT_VIOLATION = 1
EXIT_USAGE = 2
EXIT_FAIL_CLOSED = 3

_BINARY_SNIFF_BYTES = 8192


class ConfigError(Exception):
    """A check cannot run as specified. Always fails closed (exit 3)."""


class Finding:
    """One violation: where it is, and what is wrong with it."""

    def __init__(self, path: str, detail: str, line: int | None = None) -> None:
        self.path = path
        self.detail = detail
        self.line = line

    def __str__(self) -> str:
        location = self.path if self.line is None else f"{self.path}:{self.line}"
        return f"{location}: {self.detail}"

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"Finding({self!s})"


def resolve_root(argument: str | None) -> Path:
    """Return the repository root to scan.

    With no argument, walk up from this file to the enclosing git work tree.
    """
    start = Path(argument).resolve() if argument else Path(__file__).resolve().parent
    if not start.exists():
        raise ConfigError(f"root does not exist: {start}")
    result = _git(start, "rev-parse", "--show-toplevel")
    return Path(result.strip()).resolve()


def carried_files(root: Path) -> list[str]:
    """Return every file git carries or would carry, as sorted relative paths.

    Tracked files that no longer exist on disk (staged deletions) are dropped:
    they publish nothing.
    """
    tracked = _git_z(root, "ls-files", "-z")
    untracked = _git_z(root, "ls-files", "--others", "--exclude-standard", "-z")
    names = set(tracked) | set(untracked)
    return sorted(name for name in names if (root / name).is_file())


def read_text(path: Path) -> str | None:
    """Return a file's text, or None when it is binary or undecodable.

    Binary detection is a null-byte sniff over the head of the file. Callers
    still check binary files' *names*: a filename is text no matter what it
    labels.
    """
    try:
        with path.open("rb") as handle:
            head = handle.read(_BINARY_SNIFF_BYTES)
            if b"\0" in head:
                return None
            rest = handle.read()
    except OSError:
        return None
    try:
        return (head + rest).decode("utf-8")
    except UnicodeDecodeError:
        return None


def load_list_file(path: Path, label: str) -> list[str]:
    """Load a one-entry-per-line data file, stripping '#' comments.

    Raises ConfigError when the file is missing, unreadable, or carries no
    entries once comments and blank lines are removed. An empty list file is a
    broken input, not an empty policy.
    """
    try:
        raw = path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ConfigError(f"{label} file is missing: {path}") from error
    except OSError as error:
        raise ConfigError(f"{label} file is unreadable: {path} ({error})") from error
    except UnicodeDecodeError as error:
        raise ConfigError(f"{label} file is not valid UTF-8: {path}") from error

    entries = []
    for line in raw.splitlines():
        entry = line.split("#", 1)[0].strip()
        if entry:
            entries.append(entry)
    if not entries:
        raise ConfigError(f"{label} file has no entries: {path}")
    return entries


def report(check_name: str, findings: list[Finding], stream=None) -> int:
    """Print a check's result and return its exit code."""
    stream = stream or sys.stdout
    if not findings:
        print(f"{check_name}: OK", file=stream)
        return EXIT_OK
    print(f"{check_name}: {len(findings)} violation(s)", file=stream)
    for finding in findings:
        print(f"  {finding}", file=stream)
    return EXIT_VIOLATION


def fail_closed(check_name: str, error: ConfigError, stream=None) -> int:
    """Print a fail-closed diagnostic and return its exit code."""
    stream = stream or sys.stderr
    print(f"{check_name}: FAIL CLOSED — {error}", file=stream)
    return EXIT_FAIL_CLOSED


def _git(root: Path, *args: str) -> str:
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as error:
        raise ConfigError("git is not available") from error
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() or error.stdout.strip()
        raise ConfigError(f"git {' '.join(args)} failed in {root}: {detail}") from error
    return completed.stdout


def _git_z(root: Path, *args: str) -> list[str]:
    return [name for name in _git(root, *args).split("\0") if name]


def git_is_ignored(root: Path, relative_path: str) -> bool:
    """Return True when git would refuse to add the given path."""
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), "check-ignore", "-q", "--", relative_path],
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as error:
        raise ConfigError("git is not available") from error
    if completed.returncode in (0, 1):
        return completed.returncode == 0
    detail = completed.stderr.strip()
    raise ConfigError(f"git check-ignore failed for {relative_path}: {detail}")
